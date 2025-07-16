use crate::{errors::Error, vars::CAPINDE_API_KEY};
use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};

pub async fn auth(req: Request, next: Next) -> Response {
    // Check if the request has a valid authentication token
    if let Some(authorization) = req.headers().get("Authorization") {
        if authorization == format!("Bearer {}", *CAPINDE_API_KEY).as_str() {
            // If the token is valid, allow the request to proceed
            return next.run(req).await;
        }
    }

    Error::Unauthorized.into_response()
}
