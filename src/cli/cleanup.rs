use std::time::Duration;

use crate::{
    janitor::FallbackJanitor,
    vars::{CAPINDE_NAMESPACE_BASE, MAX_TTL_SECS},
};

pub fn run() {
    println!("Running cleanup...");
    let mut janitor = FallbackJanitor::new(
        Duration::from_secs(*MAX_TTL_SECS),    // 过期时间
        String::from(*CAPINDE_NAMESPACE_BASE), // 扫描命名空间目录
    );

    match janitor.clean_expired_files() {
        Ok(total) => {
            println!("Cleanup completed. Total files cleaned: {total}");
        }
        Err(e) => {
            panic!("Error during cleanup: {e}");
        }
    }
}
