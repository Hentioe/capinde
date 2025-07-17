use crate::{
    cli::Args,
    errors::Result,
    models::WorkingMode,
    vars::{
        CAPINDE_API_KEY, CAPINDE_HOST, CAPINDE_NAMESPACE_BASE, CAPINDE_PORT, CAPINDE_WORKING_MODE,
        MAX_UPLOAD_SIZE, init_started_at,
    },
};
use axum::{
    Router,
    body::Body,
    extract::{DefaultBodyLimit, MatchedPath},
    http::Request,
    middleware,
    routing::{delete, get, post, put},
};
use clap::Parser;
use log::info;
use std::str::FromStr;
use tokio::signal;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::{info_span, warn};

mod captchas;
mod cli;
mod errors;
mod handlers;
mod janitor;
mod keys;
mod logger;
mod middlewares;
mod models;
mod provider;
mod routes;
mod scueduler;
mod vars;
mod verification;

#[tokio::main]
async fn main() -> Result<()> {
    let bind = format!("{}:{}", *CAPINDE_HOST, *CAPINDE_PORT);
    let args = Args::parse();
    if args.healthcheck {
        cli::healthcheck::run(args, bind);
    } else if args.cleanup {
        cli::cleanup::run();
    } else {
        web_serve(&bind).await?;
    }

    Ok(())
}

async fn web_serve(bind: &str) -> Result<()> {
    // Initialize the logger
    logger::init();
    // Set up the application environment variables
    env_setup().await?;
    // Parse the work mode
    let working_mode = WorkingMode::from_str(*CAPINDE_WORKING_MODE)?;
    // Initialize the janitor
    janitor::init();
    // Initialize the scheduler
    scueduler::init().await;
    // Initialize the provider
    provider::init();
    // Is API authentication enabled
    let is_auth_enabled = !(*CAPINDE_API_KEY).is_empty();
    // Provider routes
    let provider_routes = Router::new()
        .route("/deployed", get(routes::provider::deployed))
        .route("/uploaded", get(routes::provider::get_uploaded))
        .route("/uploaded", delete(routes::provider::delete_uploaded))
        .route("/deploy", put(routes::provider::deploy))
        .route("/reload", put(routes::provider::reload))
        .route(
            "/upload",
            post(routes::provider::upload).layer(DefaultBodyLimit::max(*MAX_UPLOAD_SIZE)),
        );
    // Janitor routes
    let janitor_routes = Router::new()
        .route("/status", get(routes::janitor::status))
        .route("/schedule", put(routes::janitor::schedule));

    // Server routes
    let server_routes = Router::new().route("/info", get(routes::server::info));

    let mut app = Router::new()
        .route("/api/generate", post(routes::generate))
        .route("/api/verify", post(routes::verify))
        .nest("/api/provider", provider_routes)
        .nest("/api/janitor", janitor_routes)
        .nest("/api/server", server_routes)
        .route("/api/healthcheck", get(routes::healthcheck));

    app = if is_auth_enabled {
        info!("API authentication is enabled");
        keys::check_key(&CAPINDE_API_KEY)?;
        app.route_layer(middleware::from_fn(middlewares::auth))
    } else {
        warn!("API authentication is not enabled (dangerous!)");
        app
    };
    app = match working_mode {
        WorkingMode::Hosted => {
            if !is_auth_enabled {
                panic!("API authentication must be enabled in hosted mode");
            }
            info!(
                "Working in {working_mode} mode, serving static files from: {} (/assets)",
                *CAPINDE_NAMESPACE_BASE
            );
            app.nest_service("/assets", ServeDir::new(*CAPINDE_NAMESPACE_BASE))
        }
        WorkingMode::Localized => {
            // 图片仅本地访问
            info!("Working in {working_mode} mode, output files are only accessible locally");
            app
        }
    };
    // 将日志追踪层添加到最后面
    app = app.layer(trace_layer());
    // 初始化启动时间
    init_started_at();
    // 输出启动日志
    info!("Starting server at http://{bind}");
    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

type MyTraceLayer<M> = TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    M,
>;

fn trace_layer() -> MyTraceLayer<impl Fn(&Request<Body>) -> tracing::Span + Clone> {
    TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
        // 获取匹配的路由路径（带占位符，如 /users/:id）
        let matched_path = request
            .extensions()
            .get::<MatchedPath>()
            .map(MatchedPath::as_str);

        // 获取查询参数字符串
        let query = request.uri().query();

        // 获取 Content-Type
        let content_type = request
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok());

        info_span!(
            "http_request",
            method = %request.method(),
            matched_path = matched_path,
            query = query,
            content_type = content_type,
            status_code = tracing::field::Empty,
            latency_ms = tracing::field::Empty,
            response_size = tracing::field::Empty,
        )
    })
}

async fn env_setup() -> Result<()> {
    // Load environment variables from .env file if it exists
    if dotenvy::dotenv().is_ok() {
        info!("loaded .env file");
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
