use crate::errors::Error;
use axum::{http::StatusCode, response::IntoResponse};
use log::error;
use serde::Serialize;
use strum::EnumProperty;

#[derive(Debug, Clone, Serialize)]
pub struct ApiError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
}

impl ApiError {
    fn from_internal_error(message: impl Into<String>) -> Self {
        ApiError {
            message: message.into(),
            code: None,
        }
    }

    pub fn new(message: impl Into<String>, code: Option<i64>) -> Self {
        ApiError {
            message: message.into(),
            code,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        if let Some(status_code) = self.get_int("status_code") {
            // 有状态码，根据状态码和错误码构建响应
            let status_code = match StatusCode::from_u16(status_code as u16) {
                Ok(status_code) => status_code,
                Err(_e) => {
                    // 这可能是一个 bug：某个错误类型定义了无效的状态码
                    error!(
                        "This may be a bug: an error type defined an invalid status code: {status_code}"
                    );

                    StatusCode::INTERNAL_SERVER_ERROR
                }
            };
            let code = self.get_int("code");

            (
                status_code,
                axum::Json(ApiError::new(self.to_string(), code)),
            )
                .into_response()
        } else if let Some(code) = self.get_int("code") {
            // 没有状态码，但有错误码
            (
                StatusCode::OK,
                axum::Json(ApiError::new(self.to_string(), Some(code))),
            )
                .into_response()
        } else {
            // 没有状态码，也没有错误码
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(ApiError::from_internal_error(self.to_string())),
            )
                .into_response()
        }
    }
}
