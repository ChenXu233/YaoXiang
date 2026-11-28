//! YaoXiang 编译脚本
//!
//! 自动下载并配置 Z3。首次运行下载到 .z3/，后续复用缓存。

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

const Z3_VERSION: &str = "4.16.0";

fn main() {
    // 重建触发面显式化：build.rs 只在这些路径与环境变量（及自身/Cargo.toml）变化时重跑。
    //
    // locales 用目录级声明——触发面必须 ⊇ 构建期校验的输入面（#458）：下文
    // code_tables::validate 的输入面是全部 locale 文件。「孤立条目」判据对
    // 任何语言（zh 与全部 bot 语言）都是 error：翻译残留 bot 不会清理，任何
    // 语言的 locale 里出现实现未定义的码都拒绝编译；「bot 语言缺失」判据
    // 覆盖 en/ja/ru/zh-classical/zh-x-miao 全部 bot 语言。而 i18n bot
    // （auto-translate-i18n.yml）以 zh.json 为触发、异步写非 zh 文件；旧触发面
    // 只声明 zh.json ⇒ 非 zh 变更（bot 落地、人工清理翻译残留）永不触发校验：
    // 假告警停留在上一次运行的结论上（cargo 重放旧 output 的 warning），真孤立
    // 条目漏检。目录级声明由 cargo 扫描整个目录的变更，非 zh 文件变化即重跑。
    println!("cargo:rerun-if-changed=locales");
    // #548：触发面补齐其余三处输入面缺口（判据同 #458：触发面 ⊇ build.rs 输出
    // 结论的输入面）。cargo 一旦收到任意 rerun-if 声明就只跟踪已声明项，不再
    // 默认跟踪包内全部文件——漏声明的输入永远不触发重跑，结论停留在上一次
    // 运行的缓存上（cargo 重放旧 output）。
    //
    // 1) .z3 目录：host 分支的 find_local_z3 读它找本地 Z3，自动下载/解压写它。
    //    目录级声明由 cargo 递归扫描：内部内容变化（文件 mtime 超过 build script
    //    output 文件）或目录整个消失（删掉 .z3 后的增量构建也必须重新判定，禁
    //    缓存跳过）都触发重跑；下载/解压写 .z3 的自触发是幂等重跑（下一轮结论
    //    相同即停止传导），无害且正确。原 wasm 分支内的同名声明已合并至此——
    //    顶部声明对全部目标（含 wasm32）生效，路径相对 package root 解析，两处
    //    指向同一目录，保留两份只是平行事实。
    println!("cargo:rerun-if-changed=.z3");
    // 2) Z3_SYS_Z3_HEADER：host 分支读它决定是否改用系统 Z3（值对应路径存在 ⇒
    //    链接系统 Z3 并提前返回）。rerun-if-env-changed 按「值或缺省」入指纹
    //    哈希：从未设置→设置、值变化、设置→取消三种迁移都触发重跑。
    println!("cargo:rerun-if-env-changed=Z3_SYS_Z3_HEADER");
    // 3) CXXSTDLIB：link_z3 非 Windows 且非 macOS 分支读它决定
    //    cargo:rustc-link-lib 输出（未设置时回退 stdc++）——macOS 目标硬编码
    //    c++、不读该变量。只影响那一部分链接面；Windows/macOS 构建不读它，
    //    声明本身无害（值不变即不触发）。
    println!("cargo:rerun-if-env-changed=CXXSTDLIB");
    // RFC-013 路径单一源在 code_tables::RFC013_REL，此处复用避免双份漂移
    println!("cargo:rerun-if-changed={}", code_tables::RFC013_REL);
    println!("cargo:rerun-if-changed=src/util/diagnostic/codes");
    println!("cargo:rerun-if-changed=src/std/result.rs");
    // P6 6.0.1：类型表门禁 T1 的扫描面——变体定义、三类构造点与反向桥点变化都要重跑。
    // 路径常量单一源在 type_tables（与 code_tables::RFC013_REL 同一形态，避免双份漂移）。
    for path in [
        type_tables::AST_REL,
        type_tables::MONO_REL,
        type_tables::LSP_REL,
        type_tables::BYTECODE_REL,
        type_tables::FUNCTION_REL,
        type_tables::SCAN_ROOT_REL,
    ] {
        println!("cargo:rerun-if-changed={path}");
    }

    // #325/#326：构建期权威校验（解析/校验/比对逻辑单一实现于 tools/code-tables lib）——
    // 注册码唯一性、段位、zh 完备性、locales 孤立、RFC-013 码表区间一致性；
    // 任一 error 即拒绝编译。bot 语言缺失降级为 cargo:warning。
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = Path::new(&manifest_dir);
    let entries = code_tables::parse_registry(root).unwrap_or_else(|msg| panic!("{msg}"));
    let report = code_tables::validate(root, &entries).unwrap_or_else(|msg| panic!("{msg}"));
    for warning in &report.warnings {
        println!("cargo:warning={warning}");
    }
    for error in &report.errors {
        println!("cargo:warning={error}");
    }
    if !report.is_ok() {
        panic!(
            "错误码注册表校验失败（{} 项 error，见上方清单）。zh 是唯一人工翻译源：新增码请同步补 zh 条目。",
            report.errors.len()
        );
    }

    // RFC-013 码表区间一致性（漂移即拒绝编译；治愈：code-tables --fix）
    let rfc_path = root.join(code_tables::RFC013_REL);
    let doc = std::fs::read_to_string(&rfc_path)
        .unwrap_or_else(|e| panic!("读取 {} 失败: {}", rfc_path.display(), e));
    let zh = code_tables::load_locale(root, "zh").unwrap_or_else(|msg| panic!("{msg}"));
    let mismatches = code_tables::check_rfc_tables(&doc, &entries, &zh);
    if !mismatches.is_empty() {
        panic!(
            "RFC-013 码表区间与注册表不一致：
  {}
治愈：cargo run --manifest-path tools/code-tables/Cargo.toml -- --fix",
            mismatches.join(
                "
  "
            )
        );
    }

    // P10 S6.2 opcode 表门禁（F7+F8）：`opcode.rs` 的 `pub const` 声明与 `OPCODE_INFO`
    // 权威表逐项对账——码值唯一、每常量有表项、表项名非空且唯一、按 code 升序。
    // 判定逻辑单一实现于 `code_tables::opcode_gate`（与 type_tables 门禁同形），
    // **构建硬失败**：任一漂移即 panic 拒绝编译，通过时只打一行摘要。
    println!("cargo:rerun-if-changed={}", code_tables::OPCODE_REL);
    match code_tables::opcode_gate(root) {
        Ok(summary) => println!("cargo:warning={summary}"),
        Err(detail) => panic!("opcode 表门禁 F8 失败（构建已拒绝）：\n  {detail}"),
    }

    // P6 6.6.2 类型表门禁 T1（`ast::Type` 构造点三类分列 + 零构造清单 + 勘察基线对账）——
    // **构建硬失败**（用户裁决 2A，2026-10-30）：任一违规即 panic 拒绝编译，通过时只打一行摘要。
    match type_tables::build_gate(root) {
        Ok(summary) => println!("cargo:warning={summary}"),
        Err(detail) => panic!("类型表门禁 T1 失败（构建已拒绝）：\n{detail}"),
    }

    let _target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target_arch == "wasm32" {
        // wasm 不自动下载（没有预编译 wasm 二进制），只认本地预编译资产：
        // z3-wasm release 的 tar.gz 解压到 .z3/（含 libz3.a + Emscripten sysroot 库）。
        //
        // #435 裁决：wasm 形态必须带 Z3——无 Z3 时精化/终止性等核心功能不可用，
        // 语言残缺。「找不到资产就降级」曾是 warning 形态，静默产出无 Z3 的
        // 构建（8 天无人察觉的事故形态）；现在找不到 = 构建失败。
        let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
        let z3_root = Path::new(&manifest).join(".z3");
        let Some(dir) = find_local_z3_wasm(&z3_root) else {
            panic!(
                "wasm32 构建需要 Z3 wasm 预编译资产：\
                 下载 https://github.com/ChenXu233/YaoXiang/releases/download/z3-wasm/z3-wasm-{}.tar.gz \
                 解压到 .z3/（应含 lib/libz3.a + libc++-wasmexcept 等 sysroot 库）。\
                 无 Z3 的 wasm 形态不受支持（#435）。",
                Z3_VERSION
            );
        };
        println!("cargo:warning=Linking Z3 wasm from {:?}", dir);
        link_z3_wasm(&dir);
        return;
    }

    // 1. 尝试系统安装的 Z3（Z3_SYS_Z3_HEADER 环境变量）
    if let Ok(header) = env::var("Z3_SYS_Z3_HEADER") {
        if Path::new(&header).exists() {
            let dir = Path::new(&header).parent().unwrap().parent().unwrap();
            link_z3(dir);
            copy_shared_lib(dir);
            return;
        }
    }

    // 2. 查找项目本地 .z3/
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let z3_root = Path::new(&manifest).join(".z3");

    // 遍历 .z3/ 子目录，找第一个包含 include/z3.h 的
    let local = find_local_z3(&z3_root);
    if let Some(ref dir) = local {
        link_z3(dir);
        copy_shared_lib(dir);
        return;
    }

    // 3. 自动下载
    let target = match detect_target() {
        Some(t) => t,
        None => {
            let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
            let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
            println!(
                "cargo:warning=Z3 prebuilt binaries not available for {}/{}. \
                 Set Z3_SYS_Z3_HEADER or place Z3 in .z3/ to provide Z3.",
                os, arch
            );
            return;
        }
    };
    let archive_name = format!("z3-{}-{}.zip", Z3_VERSION, target);
    let url = format!(
        "https://github.com/Z3Prover/z3/releases/download/z3-{}/{}",
        Z3_VERSION, archive_name
    );

    fs::create_dir_all(&z3_root).ok();
    let archive = z3_root.join(&archive_name);
    let z3_dir = z3_root.join(format!("z3-{}-{}", Z3_VERSION, target));

    // 下载可能被截断（网络抖动、代理中途断流），而**截断的 zip 仍能解出前若干个条目**：
    // 症状是 bin/libz3.dll 在，include/z3.h 与 bin/libz3.lib 不在，最终以
    // 「LNK1181: cannot open input file 'libz3.lib'」这种与病因无关的链接错误暴露
    // （v0.8.9 Windows dist 连续两轮如此；同一个 zip 本机完整下载 66 MB，解压正常）。
    // 故解压后必须验证锚点文件，缺则清掉半成品重下；重试耗尽则点名 Z3 失败，
    // 不再让链接器替这里兜底。
    const Z3_MAX_ATTEMPTS: u32 = 3;
    let mut attempt = 0;
    while !z3_dir.join("include").join("z3.h").exists() {
        attempt += 1;
        if attempt > Z3_MAX_ATTEMPTS {
            panic!(
                "Z3 {} 解压后仍缺少 include/z3.h（{}）：重试 {} 次均失败。\
                 检查到 {} 的网络，或设置 Z3_SYS_Z3_HEADER / 手动把 Z3 放到 .z3/。",
                Z3_VERSION,
                z3_dir.display(),
                Z3_MAX_ATTEMPTS,
                url
            );
        }
        if attempt > 1 {
            // 半成品目录会被下一次解压复用（只补新增条目），先整体清掉
            println!(
                "cargo:warning=Z3 解压不完整，清理后重试（第 {} / {} 次）...",
                attempt, Z3_MAX_ATTEMPTS
            );
            let _ = fs::remove_dir_all(&z3_dir);
        }
        let _ = fs::remove_file(&archive);
        println!(
            "cargo:warning=Downloading Z3 {} for {}...",
            Z3_VERSION, target
        );
        download(&url, &archive);
        // 下限只能拦「下到的根本不是压缩包」；截断只能在解压后按锚点文件判定
        let meta = fs::metadata(&archive).expect("Failed to read Z3 archive metadata");
        if meta.len() < 1024 * 1024 {
            let _ = fs::remove_file(&archive);
            panic!(
                "Z3 archive too small ({} bytes), download likely failed. \
                 Set Z3_SYS_Z3_HEADER or place Z3 in .z3/ to skip download.",
                meta.len()
            );
        }
        println!("cargo:warning=Extracting Z3...");
        extract(&archive, &z3_root);
        let _ = fs::remove_file(&archive);
    }

    link_z3(&z3_dir);
    copy_shared_lib(&z3_dir);
}

fn link_z3(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    // Z3 官方包把静态库放 lib/ 或 bin/（osx/linux 资产在 bin/，含 libz3.a）；
    // 静态链接时搜索目录必须含 libz3.a
    let lib_dir = ["lib", "bin"]
        .iter()
        .map(|s| z3_dir.join(s))
        .find(|d| {
            d.exists()
                && (d.join("libz3.a").exists()
                    || d.join("z3.lib").exists()
                    || d.join("libz3.dll").exists()
                    || d.join("libz3.dylib").exists()
                    || d.join("libz3.so").exists())
        })
        .unwrap_or_else(|| z3_dir.join("bin"));

    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // RFC-037：全平台动态链接。共享库随发行包 bin/ 分发，用户可整体替换升级 Z3。
    if target_os == "windows" {
        // MSVC import lib 命名为 libz3.lib
        println!("cargo:rustc-link-lib=libz3");
    } else {
        // macOS 静态链接：官方 x64-osx 资产 4.16.0 的 bin/libz3.dylib 错装为
        // arm64（dist v0.8.0 链接实测 "found architecture 'arm64'"），动态链接
        // 在 x86_64 目标上不可用；bin/libz3.a 为 x86_64+arm64 双架构 fat 库，
        // 静态链接自包含且两个 Apple 目标通用。
        let static_macos = target_os == "macos" && lib_dir.join("libz3.a").exists();
        if static_macos {
            println!("cargo:rustc-link-lib=static=z3");
        } else {
            println!("cargo:rustc-link-lib=z3");
            // 动态链接器默认不搜二进制所在目录，必须注入 rpath，“解压即用”才成立
            // （发行包内 exe 与 libz3 同在 bin/；Windows 默认搜 exe 目录，无需处理）
            match target_os.as_str() {
                "linux" => println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN"),
                "macos" => println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path"),
                _ => {}
            }
        }
        if target_os == "linux" {
            // dist 管线用 lld（默认 --no-allow-shlib-undefined），预编译
            // libz3.so 自身的未定义符号（libstdc++ 等）在可执行链接期被拒——
            // 这些符号由 .so 的 DT_NEEDED 在运行期解析，显式放行
            println!("cargo:rustc-link-arg=-Wl,--allow-shlib-undefined");
        }
        let cxx = if target_os == "macos" {
            "c++".to_string()
        } else {
            env::var("CXXSTDLIB").unwrap_or_else(|_| "stdc++".into())
        };
        println!("cargo:rustc-link-lib={}", cxx);
    }
}

/// 从 OUT_DIR 上溯定位 cargo 的 profile 目录（`target/<triple>/<profile>`）。
///
/// 不按固定层数上溯：cargo 1.100 起把单元产物目录从
/// `<profile>/build/<pkg>-<hash>/out` 改为 `<profile>/build/<pkg>/<hash>/out`，
/// OUT_DIR 与 profile 的层距随工具链版本变化（旧布局 3 层、新布局 4 层）。
/// 两种布局的共同锚点是路径中最近的 `build` 目录：它的父目录恒为 profile
/// 目录，带 target triple 时（`target/<triple>/<profile>/build/...`）同样成立。
fn profile_dir(out: &Path) -> Option<&Path> {
    out.ancestors()
        .find(|p| p.file_name().is_some_and(|name| name == "build"))?
        .parent()
}

/// 把 Z3 共享库复制进 target profile 目录，本地 cargo run/测试才能加载；
/// Z3 许可证（MIT 分发义务）随库一并落盘。发版产物由 package-dist.sh
/// 从同一目录取用，不重复维护平台映射。
///
/// 布局差异：cargo < 1.100 的测试二进制落在 `<profile>/deps`，与依赖库同目录，
/// 因此旧布局还要副本进 `deps`；cargo >= 1.100 取消 `deps`，每个单元各占
/// `<profile>/build/<pkg>/<hash>/out`，构建脚本无法预知这些目录——该布局下
/// 跑测试须把 Z3 共享库所在目录加入动态库搜索路径（Windows `PATH`、
/// Linux `LD_LIBRARY_PATH`）。
fn copy_shared_lib(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let name = match target_os.as_str() {
        "windows" => "libz3.dll",
        "macos" => "libz3.dylib",
        "linux" => "libz3.so",
        _ => return,
    };
    // 布局不统一：官方发行包在 lib/ 或 bin/；系统 Z3（Debian multiarch）在
    // lib/<arch>-linux-gnu/
    let mut search_dirs = vec![z3_dir.join("lib"), z3_dir.join("bin")];
    if target_os == "linux" {
        let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
        search_dirs.push(z3_dir.join("lib").join(format!("{arch}-linux-gnu")));
    }
    let src = search_dirs
        .iter()
        .map(|d| d.join(name))
        .find(|p| p.exists());
    let src = match src {
        Some(p) => p,
        None => return,
    };
    let out = env::var("OUT_DIR").unwrap();
    let Some(profile) = profile_dir(Path::new(&out)) else {
        return;
    };
    let _ = fs::copy(&src, profile.join(name));
    // 旧布局的共享测试库目录；新布局不创建它，也就不复制
    let deps = profile.join("deps");
    if deps.is_dir() {
        let _ = fs::copy(&src, deps.join(name));
    }
    // MIT 要求分发二进制时附带许可文本；Z3 发行包根有 LICENSE.txt
    for license in ["LICENSE.txt", "LICENSE"] {
        let license_src = z3_dir.join(license);
        if license_src.exists() {
            let _ = fs::copy(&license_src, profile.join("LICENSE-Z3.txt"));
            break;
        }
    }
}

fn find_local_z3(z3_root: &Path) -> Option<std::path::PathBuf> {
    if !z3_root.exists() {
        return None;
    }
    for entry in fs::read_dir(z3_root).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        // #435：跳过 wasm 资产目录——同名 include/z3.h + lib/libz3.a 会被误认
        // 为 host 库（wasm 目录没有 host 的 libz3.lib/dll，误认即 LNK1181）
        if entry.file_name().to_string_lossy().contains("wasm") {
            continue;
        }
        if path.join("include").join("z3.h").exists() {
            return Some(path);
        }
    }
    None
}

fn detect_target() -> Option<&'static str> {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    match (os.as_str(), arch.as_str()) {
        ("windows", "x86_64") => Some("x64-win"),
        ("windows", "aarch64") => Some("arm64-win"),
        ("linux", "x86_64") => Some("x64-glibc-2.39"),
        ("linux", "aarch64") => Some("arm64-glibc-2.38"),
        ("macos", "x86_64") => Some("x64-osx-15.7.3"),
        ("macos", "aarch64") => Some("arm64-osx-15.7.3"),
        _ => None,
    }
}

fn download(
    url: &str,
    dest: &Path,
) {
    // 使用宿主平台的工具，不是目标平台
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
        Ok(s) => panic!("Download failed, exit: {:?}", s.code()),
        Err(e) => panic!("Download failed: {}", e),
    }
}

fn extract(
    archive: &Path,
    dest: &Path,
) {
    // 使用宿主平台的工具，不是目标平台
    let status = if cfg!(target_os = "windows") {
        // 不用 Expand-Archive：Windows PowerShell 5.1 会加载 PowerShell 7 的
        // Microsoft.PowerShell.Archive 模块，其内部调用的
        // ZipFileExtensions::ExtractToDirectory 三参数重载在 .NET Framework 上不存在，
        // 于是 cmdlet 只抛**非终止**错误、一个条目都不解压，而 `powershell -Command`
        // 仍以 0 退出——症状是「下载成功、解压声称成功、产物什么都没有」，最终由链接器
        // 以 LNK1181 报出来（v0.8.9 Windows dist 与本地增量构建均实测）。
        // bsdtar 随 Windows 10/11 与 GitHub runner 提供，退出码真实。
        Command::new("tar")
            .arg("-xf")
            .arg(archive)
            .arg("-C")
            .arg(dest)
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

/// 查找 .z3/ 目录下的 wasm 预编译 Z3
fn find_local_z3_wasm(z3_root: &Path) -> Option<std::path::PathBuf> {
    if !z3_root.exists() {
        return None;
    }
    for entry in fs::read_dir(z3_root).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        // 检查 lib/libz3.a 是否存在
        if path.join("lib").join("libz3.a").exists() {
            return Some(path);
        }
    }
    None
}

/// 链接 wasm 预编译的 Z3（Emscripten 产出的 .a 文件）+ sysroot 运行时链（#435）。
///
/// 库链名单是 wasm32-unknown-unknown 实证定稿（缺符号 194→0 的收敛结果）：
/// - libc++/libc++abi/libunwind 必须用 **wasmexcept**（wasm 原生异常）变体——
///   Z3 求解内核的控制流依赖 C++ 异常捕获，SjLj 变体会引入 invoke_*
///   JS 运行时依赖（wasm32-unknown-unknown 没有那层胶水）
/// - libclang_rt.builtins 必须是非后缀版——`wasmsjlj` 变体同样引入 invoke_*
/// - emmalloc 提供 malloc/free（musl libc.a 不含分配器）
/// - c++abi 与 c 重复列出：lld wasm 单趟 archive 扫描，循环依赖靠重列解决
///
/// 缺失任何库 = 资产不完整，构建失败（不发残缺产物）。
fn link_z3_wasm(z3_dir: &Path) {
    let lib_dir = z3_dir.join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=z3");
    // verbatim：库名含 '+' 和 '.'，绕过 -l 的 lib 前缀/后缀推导
    const SYSROOT_LIBS: &[&str] = &[
        "libc++-wasmexcept.a",
        "libc++abi-wasmexcept.a",
        "libunwind-wasmexcept.a",
        "libemmalloc.a",
        "libc.a",
        "libclang_rt.builtins.a",
    ];
    for name in SYSROOT_LIBS {
        assert!(
            lib_dir.join(name).exists(),
            "wasm Z3 资产不完整：缺 {name}——重新下载完整 z3-wasm release 资产（#435）"
        );
        println!("cargo:rustc-link-lib=static:+verbatim={name}");
    }
}
