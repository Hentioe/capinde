pub mod generation;
pub mod janitor;
pub mod provider;
mod verification;

pub use generation::generate;
pub use verification::verify;

use log::info;

pub async fn healthcheck() -> String {
    info!("Health check endpoint hit");

    "ok".to_string()
}
