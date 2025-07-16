use std::sync::LazyLock;

macro_rules! env_config {
    ($name:ident, $env_key:expr, $default:expr) => {
        paste::paste! {
            pub static [<CAPINDE_ $name>]: ::std::sync::LazyLock<&'static str> = ::std::sync::LazyLock::new(|| {
                ::std::boxed::Box::leak(
                    ::std::env::var($env_key)
                        .unwrap_or_else(|_| $default.to_string())
                        .into_boxed_str()
                )
            });
        }
    };
    ($name:ident, $default:expr) => {
        env_config!($name, stringify!([<CAPINDE_ $name>]), $default);
    };
}

env_config!(HOST, "localhost");
env_config!(PORT, "8080");
env_config!(WORKING_MODE, "hosted");
env_config!(NAMESPACE_BASE, "namespace");
env_config!(UPLOADED_DIR, "uploaded");
env_config!(ALBUMS_BASE, "albums");
env_config!(MAX_TTL_HOURS, "12");
env_config!(MAX_UPLOAD_SIZE_MB, "300");
env_config!(API_KEY, "");

pub static MAX_TTL_SECS: LazyLock<u64> = LazyLock::new(|| {
    CAPINDE_MAX_TTL_HOURS
        .parse::<u64>()
        .expect("Invalid CAPINDE_MAX_TTL_HOURS value")
        * 3600
});

pub static MAX_UPLOAD_SIZE: LazyLock<usize> = LazyLock::new(|| {
    CAPINDE_MAX_UPLOAD_SIZE_MB
        .parse::<usize>()
        .expect("Invalid CAPINDE_MAX_UPLOAD_SIZE_MB value")
        * 1024
        * 1024 // Convert MB to bytes
});
