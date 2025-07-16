pub type Result<T> = std::result::Result<T, Error>;

// 错误码大致规划：
//  - 100-200: 参数错误
//  - 400-500: 资源错误
//  - 内部错误无错误码（一律映射到 HTTP 500）
// 沿用的 HTTP 状态码：
//  - 401: 未授权
//  - 403: 禁止访问
//  - 404: 资源未找到
//  - 429: 请求过多

#[derive(Debug, thiserror::Error, strum_macros::EnumProperty)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum Error {
    // TTL 太大
    #[strum(props(code = 100))]
    #[error("the TTL exceeds maximum limit of {max} seconds, provided: {provided}")]
    TTLTooLarge { max: u64, provided: u64 },
    // 缺少字段
    #[strum(props(code = 101))]
    #[error("missing required field: {0}")]
    MissingField(String),
    // 命名空间包含非法字符
    #[strum(props(code = 102))]
    #[error(
        "namespace contains illegal characters: only letters, numbers, underscores, hyphens, and slashes are allowed"
    )]
    IllegalNamespace,
    // 命名空间不能以斜杠开头
    #[strum(props(code = 103))]
    #[error("namespace cannot start with a slash")]
    NamespaceStartsWithSlash,
    // 无效的网格布局
    #[strum(props(code = 110))]
    #[error("invalid grid layout: {0}, only '3x3' is supported")]
    InvalidGridLayout(String),
    // 没有已上传的压缩包
    #[strum(props(code = 410))]
    #[error("no uploaded archive found")]
    NoUploadedArchive,
    // 未找到验证缓存
    #[strum(props(status_code = 404, code = 411))]
    #[error("verification cache not found: {0}")]
    VerificationCacheNotFound(String),
    // 未授权
    #[strum(props(status_code = 401))]
    #[error("unauthorized access")]
    Unauthorized,
    // 禁止访问
    // #[strum(props(code = 403))]
    // #[error("forbidden access")]
    // Forbidden,
    // 资源未找到
    // #[strum(props(status_code = 401))]
    // #[error("resource not found")]
    // ResourceNotFound,
    // 无效的路径编码（属于内部错误）
    #[error("invalid path encoding")]
    InvalidPathEncoding,
    // 内部通用错误
    #[error("internal error: {0}")]
    Internal(String),
    // 包装 crate::keys::Error
    #[error("key error: {0}")]
    Key(#[from] crate::keys::KeyError),
    // 包装 tokio 的 JoinError
    #[error("task join error: {0}")]
    TokioTaskJoin(#[from] tokio::task::JoinError),
    // 包装 MultipartError
    #[error("multipart error: {0}")]
    Multipart(#[from] axum::extract::multipart::MultipartError),
    // 包装 std::io::Error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    // 包装 zip::result::ZipError
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    // 包装 JobSchedulerError
    #[error("job scheduler error: {0}")]
    JobScheduler(#[from] tokio_cron_scheduler::JobSchedulerError),
    // 包装 serde_yaml::Error
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    // 包装 magick_rust::MagickError
    #[error("ImageMagick error: {0}")]
    Magick(#[from] magick_rust::MagickError),
    // 包装 std::string::FromUtf8Error
    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

#[macro_export]
macro_rules! fail {
    ($msg:expr) => {
        $crate::errors::Error::Internal(format!($msg))
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::errors::Error::Internal(format!($fmt, $($arg)*))
    };
}

#[macro_export]
macro_rules! err {
    ($msg:expr) => {
        Err($crate::fail!($msg))
    };
    ($fmt:expr, $($arg:tt)*) => {
        Err($crate::fail!($fmt, $($arg)*))
    };
}
