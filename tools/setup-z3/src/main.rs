/// setup-z3: 为 YaoXiang 编译器下载预编译 Z3。
///
/// 用法: 在 tools/setup-z3/ 目录下运行 `cargo run`。
///
/// 这个工具自动检测平台，从 GitHub Releases 下载 Z3 4.16.0 预编译包，
/// 解压到项目根目录的 .z3/，写入 .cargo/config.toml 的 [env] 段，
/// 让 z3-sys 能找到 Z3 header。

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const Z3_VERSION: &str = "4.16.0";

fn main() {
    // 项目根目录 = tools/setup-z3/ 往上两级
    let exe = env::current_exe().unwrap();
    let project_root = find_project_root(&exe);

    println!("Project root: {}", project_root.display());

    // 检测平台
    let target = detect_target();
    println!("Platform: {:?}", target);

    let z3_dir = project_root.join(".z3").join(&target.dir_name());

    // 如果已安装，跳过下载
    if z3_dir.join("include").join("z3.h").exists() {
        println!("Z3 already installed at {}", z3_dir.display());
    } else {
        download_and_extract(&project_root, &target, &z3_dir);
    }

    // 写入 .cargo/config.toml
    let header = z3_dir
        .join("include")
        .join("z3.h")
        .to_str()
        .unwrap()
        .replace('\\', "/");
    write_cargo_config(&project_root, &header);

    // Windows: 复制 DLL 到 target/ 目录（如果存在）
    if cfg!(target_os = "windows") {
        copy_dll(&z3_dir, &project_root);
    }

    println!();
    println!("Done. Run `cargo build` in the project root.");
}

fn find_project_root(exe: &Path) -> PathBuf {
    // 从 target/debug/setup-z3.exe 往上找 Cargo.toml
    let mut dir = exe.to_path_buf();
    loop {
        dir = dir.parent().unwrap().to_path_buf();
        if dir.join("Cargo.toml").exists() {
            return dir;
        }
        // 兜底: 从工作目录找
        if dir.parent().is_none() {
            return env::current_dir().unwrap()
                .parent().unwrap()  // tools/setup-z3
                .parent().unwrap()  // tools
                .to_path_buf();     // project root
        }
    }
}

#[derive(Debug)]
struct Target {
    tag: &'static str,
}

impl Target {
    fn dir_name(&self) -> String {
        format!("z3-{}-{}", Z3_VERSION, self.tag)
    }

    fn archive_name(&self) -> String {
        format!("{}.zip", self.dir_name())
    }

    fn url(&self) -> String {
        format!(
            "https://github.com/Z3Prover/z3/releases/download/z3-{}/{}",
            Z3_VERSION,
            self.archive_name()
        )
    }
}

fn detect_target() -> Target {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    let tag = match (os, arch) {
        ("windows", "x86_64") => "x64-win",
        ("linux", "x86_64") => "x64-glibc-2.35",
        ("macos", "x86_64") => "x64-osx-13.7.4",
        ("macos", "aarch64") => "arm64-osx-13.7.4",
        _ => {
            eprintln!(
                "Unsupported platform: {}/{}.\n\
                 Please install Z3 manually and set Z3_SYS_Z3_HEADER env var.",
                os, arch
            );
            std::process::exit(1);
        }
    };

    Target { tag }
}

fn download_and_extract(project_root: &Path, target: &Target, dest: &Path) {
    let z3_root = project_root.join(".z3");
    fs::create_dir_all(&z3_root).unwrap();

    let archive = z3_root.join(target.archive_name());
    let url = target.url();

    if !archive.exists() {
        println!("Downloading {} ...", url);
        download(&url, &archive);
    }

    println!("Extracting...");
    extract_zip(&archive, &z3_root);
    fs::remove_file(&archive).ok();

    // Z3 zip 包含顶层目录 z3-x.y.z-platform/，解压到 z3_root
    // 检查是否解压到了正确位置
    if !dest.join("include").join("z3.h").exists() {
        // 可能嵌套了一层，尝试移动
        eprintln!("Warning: Z3 structure unexpected, checking...");
        for entry in fs::read_dir(&z3_root).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() && path.join("include").join("z3.h").exists() && path != dest {
                // 重命名到预期位置
                if dest.exists() {
                    fs::remove_dir_all(&dest).ok();
                }
                fs::rename(&path, &dest).ok();
                println!("Moved {} -> {}", path.display(), dest.display());
            }
        }
    }

    println!("Z3 installed to {}", dest.display());
}

fn download(url: &str, dest: &Path) {
    let status = if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Invoke-WebRequest -Uri '{}' -OutFile '{}'",
                    url,
                    dest.display()
                ),
            ])
            .status()
    } else {
        Command::new("curl")
            .args(["-sL", url, "-o"])
            .arg(dest)
            .status()
    };

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => panic!("Download failed, exit code: {:?}", s.code()),
        Err(e) => panic!("Download failed: {}", e),
    }
}

fn extract_zip(archive: &Path, dest: &Path) {
    let status = if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    archive.display(),
                    dest.display()
                ),
            ])
            .status()
    } else {
        Command::new("unzip")
            .args(["-q", "-o"])
            .arg(archive)
            .arg("-d")
            .arg(dest)
            .status()
    };

    match status {
        Ok(s) if s.success() => {}
        _ => panic!("Failed to extract Z3 archive"),
    }
}

fn write_cargo_config(project_root: &Path, header: &str) {
    let cargo_dir = project_root.join(".cargo");
    fs::create_dir_all(&cargo_dir).unwrap();
    let config_path = cargo_dir.join("config.toml");

    let existing = if config_path.exists() {
        fs::read_to_string(&config_path).unwrap_or_default()
    } else {
        String::new()
    };

    // 删除旧的 Z3 配置段
    let cleaned: Vec<&str> = existing
        .lines()
        .take_while(|l| !l.contains("Z3 path — auto-generated"))
        .collect();

    let mut content = cleaned.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }

    content.push_str(&format!(
        "\n# Z3 path — auto-generated by tools/setup-z3\n\
         [env]\n\
         Z3_SYS_Z3_HEADER = \"{}\"\n",
        header
    ));

    fs::write(&config_path, &content).unwrap();
    println!("Written {}", config_path.display());
}

fn copy_dll(z3_dir: &Path, project_root: &Path) {
    let dll = z3_dir.join("bin").join("libz3.dll");
    if !dll.exists() {
        return;
    }

    for profile in &["debug", "release"] {
        let target_dir = project_root.join("target").join(profile);
        if target_dir.exists() {
            let _ = fs::copy(&dll, target_dir.join("libz3.dll"));
            let _ = fs::create_dir_all(target_dir.join("deps"));
            let _ = fs::copy(&dll, target_dir.join("deps").join("libz3.dll"));
            println!("Copied libz3.dll to target/{}", profile);
        }
    }
}
