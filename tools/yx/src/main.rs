//! yx — YaoXiang 工具链前门（RFC-037）
//!
//! 前门/引擎分离：`yx` 只做版本解析与派发；编译/运行/包管理/fmt/lsp
//! 由引擎 `yaoxiang-rs` 承担。保留动词仅 `toolchain` 与 `self`，
//! 其余动词逐字透传引擎（rustup/Go GOTOOLCHAIN 对位）。
//!
//! 有意不用 clap：透传动词面无上限，手写分发保证参数逐字转发。

// ureq::Error 体量大（272B），把它 Box 进自有 Error 不值得——前门是短命进程，
// 错误只走 eprintln 一条路
#![allow(clippy::result_large_err)]

mod dispatch;
mod dl;
mod error;
mod home;
mod pin;
mod platform;
mod resolve;
mod self_update;
mod settings;
mod toolchain;

use error::Result;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // 清理上次 self update 留下的旧二进制（Windows 下运行中删不掉）
    if let Ok(bin) = home::bin_dir() {
        let _ = std::fs::remove_file(bin.join("yx.old"));
    }

    match run(&args) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("yx: {e}");
            std::process::exit(1);
        }
    }
}

fn run(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        None => dispatch::run(&[]),
        Some("toolchain") => toolchain::run(&args[1..]),
        Some("self") => match args.get(1).map(String::as_str) {
            Some("update") => self_update::run(),
            _ => {
                println!("usage: yx self update");
                std::process::exit(if args.len() > 1 { 2 } else { 0 });
            }
        },
        Some("--version" | "-V") => {
            println!("yx {VERSION}");
            Ok(())
        }
        Some("--help" | "-h") => {
            print_help();
            Ok(())
        }
        Some(_) => dispatch::run(args),
    }
}

fn print_help() {
    println!(
        "yx — YaoXiang toolchain front door (RFC-037)\n\
         \n\
         usage:\n\
           yx <command> [args...]        dispatch to the yaoxiang-rs engine\n\
           yx toolchain install [ver]    install a toolchain version (default: stable)\n\
           yx toolchain default <ver>    set the default version\n\
           yx toolchain list             list installed versions\n\
           yx toolchain uninstall <ver>  remove a version\n\
           yx toolchain update           update to latest stable\n\
           yx self update                update yx itself\n\
           yx --version                  show version\n\
         \n\
         version pinning: create yx-toolchain.toml with `toolchain = \"<version>\"`"
    );
}
