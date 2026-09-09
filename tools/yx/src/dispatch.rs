//! 透传派发：其余动词原样转给引擎 `yaoxiang-rs`（rustup 代理模式）

use crate::resolve;

/// 以 `args` 启动引擎，继承 stdio，透传退出码后退出本进程
pub fn run(args: &[String]) -> ! {
    let engine = match resolve::resolve() {
        Ok(r) => r.engine().to_path_buf(),
        Err(e) => {
            eprintln!("yx: {e}");
            std::process::exit(1);
        }
    };
    match std::process::Command::new(&engine).args(args).status() {
        Ok(status) => std::process::exit(status.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("yx: failed to launch engine {}: {e}", engine.display());
            std::process::exit(1);
        }
    }
}
