use crate::cli::Args;

pub fn run(args: Args, fallback_bind: String) {
    println!("Running health check...");
    let bind = args.check_bind.unwrap_or(fallback_bind);
    let endpoint = format!("http://{bind}/api/healthcheck");
    println!("Health check endpoint: {endpoint}");
    // 用 HTTP 客户端调用 /api/healthcheck 接口
    match minreq::get(endpoint).with_timeout(1).send() {
        Ok(resp) => {
            let status_code = resp.status_code;

            // 如果 状态码不是 200, 则使用错误码退出程序
            if status_code != 200 {
                eprintln!("Health check failed with status: {status_code}");
                std::process::exit(1);
            } else {
                println!("Health check passed");
            }
        }
        Err(e) => {
            eprintln!("Health check failed: {e}");
            std::process::exit(1);
        }
    }
}
