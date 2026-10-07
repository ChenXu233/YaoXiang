//! 语料行为探针（WBS 2.3.1 / 07-equivalence-oracle §三层判据·第三层）
//!
//! 对三层语料逐文件执行并捕获**归一化行为记录**（JSONL，stdout 一行一文件）：
//! - `tests/yaoxiang/**/*.yx`（语言可用性语料，RFC-036 §9）
//! - `src/std/tests/**/*.yx`（库测试）
//! - `tests/yaoxiang-multifile/*/`（多文件项目夹具，入口约定 main.yx，D48）
//!
//! 归一化规则（07 §第三层比对表）：
//! - 诊断列表：从子进程 stderr 解析 `error|warning [CODE]` 及其首个 `--> file:line:col`，
//!   按 (code, file, line) 排序入库；**消息文本不入库**（类型表示收敛会改措辞）
//! - 退出码：直接记录
//! - 运行时 stdout/stderr：按行排序入库（归并顺序，不归并内容）；
//!   stderr 内仓库绝对路径剥前缀并统一正斜杠（跨 checkout/跨平台可比）；
//!   指针形态（`@` 后接 ≥9 位十进制，如 `struct@2189...`）归一为 `@<ptr>`——
//!   地址不属行为语义（99-demos/borrow_conflict_basic.yx 实测每次进程分配不同）
//! - `dump_bytecode`：本探针不覆盖——字节码比对仅属 C1/C2 阶段（07 表注）
//!
//! 判定语义（expect/skip/mode）由 `util::test_markers::TestFileSpec` 单点解析——
//! 与 yx_runner / yx_multifile_runner / `yaoxiang test` 同一实现（禁令三）。
//!
//! 用法：
//! ```text
//! cargo build --bin yaoxiang-rs
//! cargo run --quiet --example corpus_probe > target/corpus-current.jsonl
//! python scripts/ci/check-corpus-parity.py tests/baselines/corpus-parity.jsonl target/corpus-current.jsonl
//! ```
//!
//! bin 落后于引擎源码时探针直接拒跑（见 main 的守卫）：`cargo run --example`
//! 不重编 bin，过期引擎录出的基线是旧行为且无声无息。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use yaoxiang::util::test_markers::{Expectation, TestFileSpec};

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ext = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };
    let binary = manifest
        .join("target")
        .join("debug")
        .join(format!("yaoxiang-rs{ext}"));
    if !binary.exists() {
        eprintln!(
            "error: {} 不存在——先运行 cargo build --bin yaoxiang-rs",
            binary.display()
        );
        std::process::exit(2);
    }
    // 新鲜度守卫：bin 落后于引擎源码 = 磁盘引擎可能不含最新编译器行为，
    // 录出的基线无声无息地是旧行为（#385 期实证：改 String 语义后未重建
    // bin，string_index 语料被录成旧诊断）。
    //
    // 处置是自愈而非硬拒：mtime 只是启发式（Cargo.lock 元数据刷新等
    // 会在 cargo 判定无需重编时误报），发现过期先自动 cargo build，
    // cargo 的指纹才是新鲜度的权威——它重编或判定已新鲜都放行；
    // 只有 build 本身失败（含无 cargo 可用）才拒跑并给手动命令。
    if let Some(newest_src) = newest_engine_source_mtime(&manifest) {
        let bin_mtime = || std::fs::metadata(&binary).and_then(|m| m.modified()).ok();
        if bin_mtime().is_none_or(|t| t < newest_src) {
            let built = Command::new("cargo")
                .args(["build", "--bin", "yaoxiang-rs"])
                .current_dir(&manifest)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if !built {
                eprintln!(
                    "error: {} 比引擎源码旧，且自动 cargo build 未能完成——先手动运行 cargo build --bin yaoxiang-rs",
                    binary.display()
                );
                std::process::exit(2);
            }
        }
    }

    let mut files = Vec::new();
    collect_yx(&manifest.join("tests").join("yaoxiang"), &mut files);
    collect_yx(&manifest.join("src").join("std").join("tests"), &mut files);
    // 多文件项目夹具（D48）：含 yaoxiang.toml 的一级子目录，入口 main.yx
    let mf_root = manifest.join("tests").join("yaoxiang-multifile");
    if mf_root.is_dir() {
        for entry in std::fs::read_dir(&mf_root).unwrap() {
            let dir = entry.unwrap().path();
            if dir.is_dir() && dir.join("yaoxiang.toml").is_file() {
                files.push(dir.join("main.yx"));
            }
        }
    }
    files.sort();

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for file in &files {
        let spec = TestFileSpec::parse(file);
        // skip/指令非法：与运行器同一语义——不执行、不入基线
        if spec.skip_reason.is_some() || spec.invalid.is_some() {
            continue;
        }
        let relative = normalize_path(file.strip_prefix(&manifest).unwrap_or(file));
        let record = probe_file(&binary, &manifest, file, &relative, &spec);
        writeln!(out, "{}", serde_json::to_string(&record).unwrap()).unwrap();
    }
}

fn collect_yx(
    dir: &Path,
    out: &mut Vec<PathBuf>,
) {
    if !dir.is_dir() {
        return;
    }
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_yx(&path, out);
        } else if path.extension().is_some_and(|e| e == "yx") {
            out.push(path);
        }
    }
}

/// 路径归一化：UNC 前缀剥离 + 反斜杠转正斜杠（跨机可比）。
fn normalize_path(p: &Path) -> String {
    let s = p.display().to_string().replace('\\', "/");
    s.strip_prefix("//?/").unwrap_or(&s).to_string()
}

fn probe_file(
    binary: &Path,
    manifest: &Path,
    file: &Path,
    relative: &str,
    spec: &TestFileSpec,
) -> Value {
    match &spec.expectation {
        Expectation::Behavior => json!({
            "file": relative,
            "expectation": "behavior",
            "run": capture_run(binary, manifest, file, spec.mode.as_ref()),
        }),
        Expectation::CompileError(_) => json!({
            "file": relative,
            "expectation": "compile-error",
            "check": capture_check(binary, manifest, file),
        }),
        Expectation::RuntimeError(_) => json!({
            "file": relative,
            "expectation": "runtime-error",
            "check": capture_check(binary, manifest, file),
            "run": capture_run(binary, manifest, file, spec.mode.as_ref()),
        }),
    }
}

/// check 步捕获：退出码 + 归一化诊断列表（消息文本不入库，07 表规则）。
fn capture_check(
    binary: &Path,
    manifest: &Path,
    file: &Path,
) -> Value {
    let output = Command::new(binary)
        .arg("check")
        .arg(file)
        .output()
        .unwrap_or_else(|e| panic!("spawn check 失败 {}: {e}", file.display()));
    let stderr = String::from_utf8_lossy(&output.stderr);
    json!({
        "exit": output.status.code().unwrap_or(-1),
        "diagnostics": parse_diagnostics(&stderr, manifest),
    })
}

/// run 步捕获：退出码 + 按行排序的 stdout/stderr（07 表规则）。
/// stderr 先做仓库路径归一（栈回溯跨 checkout/跨平台可比）。
fn capture_run(
    binary: &Path,
    manifest: &Path,
    file: &Path,
    mode: Option<&String>,
) -> Value {
    let mut command = Command::new(binary);
    command.arg("run");
    if let Some(mode) = mode {
        command.arg("--runtime").arg(mode);
    }
    let output = command
        .arg(file)
        .output()
        .unwrap_or_else(|e| panic!("spawn run 失败 {}: {e}", file.display()));
    json!({
        "exit": output.status.code().unwrap_or(-1),
        "stdout": sorted_lines(&String::from_utf8_lossy(&output.stdout)),
        "stderr": sorted_lines(&scrub_repo_paths(
            &String::from_utf8_lossy(&output.stderr),
            manifest,
        )),
    })
}

/// 按行切分、去空行、回车符剥离、指针归一、排序（并发交错只归并顺序）。
fn sorted_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text
        .lines()
        .map(|l| scrub_pointer(l.trim_end_matches('\r')))
        .filter(|l| !l.is_empty())
        .collect();
    lines.sort();
    lines
}

/// 指针/地址形态归一：`@` 后接 ≥9 位十进制 → `@<ptr>`。
///
/// 阈值 9 位对应 32/64 位进程地址的实际长度；短数字串（版本号、日期）不受影响。
fn scrub_pointer(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut seg = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            let mut end = i + 1;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end - (i + 1) >= 9 {
                out.push_str(&line[seg..=i]);
                out.push_str("<ptr>");
                seg = end;
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out.push_str(&line[seg..]);
    out
}

/// 从渲染后的 stderr 解析诊断条目：[code, file, line]，排序后返回。
///
/// 取每条 `error|warning [CODE]` 头部之后的**首个** `-->` 位置（related/help
/// 的附加位置不入库）；无位置的诊断记 file 为空串、line 为 0。
fn parse_diagnostics(
    stderr: &str,
    manifest: &Path,
) -> Vec<Value> {
    let mut out = Vec::new();
    let mut pending_code: Option<String> = None;
    for line in stderr.lines() {
        let trimmed = line.trim_start();
        if let Some(code) = parse_diag_head(trimmed) {
            pending_code = Some(code);
            continue;
        }
        if let Some(code) = pending_code.take() {
            if let Some(rest) = trimmed.strip_prefix("-->") {
                let (file, line) = parse_location(rest.trim(), manifest);
                out.push(json!([code, file, line]));
            } else {
                out.push(json!([code, "", 0]));
            }
        }
    }
    if let Some(code) = pending_code.take() {
        out.push(json!([code, "", 0]));
    }
    out.sort_by_key(|a| a.to_string());
    out
}

/// `error [E4018]` / `warning [W1002]` 头部 → 错误码。
fn parse_diag_head(trimmed: &str) -> Option<String> {
    for prefix in ["error [", "warning ["] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            if let Some(end) = rest.find(']') {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

/// `--> path:line:col` 的位置段 → (归一化 file, line)；仓库外路径原样保留。
fn parse_location(
    rest: &str,
    manifest: &Path,
) -> (String, u32) {
    // 形如 <path>:<line>:<col>；Windows 盘符含冒号，从右侧拆两段
    let mut parts = rest.rsplitn(3, ':');
    let _col = parts.next();
    let line: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let file_raw = parts.next().unwrap_or("");
    // 引擎可能吐 UNC 规范化路径（\\?\E:\...）——PathBuf::strip_prefix 对
    // 普通 manifest 不命中，导致绝对路径漏进基线；字符串级归一化
    // （UNC 剥离 + 正斜杠，与 normalize_path 同规）后再剥仓库前缀。
    let file_norm = normalize_path(&PathBuf::from(file_raw));
    let manifest_norm = normalize_path(manifest);
    let file = file_norm
        .strip_prefix(&manifest_norm)
        .map(|r| r.trim_start_matches('/').to_string())
        .unwrap_or(file_norm);
    (file, line)
}

/// bin 新鲜度守卫的源码侧：引擎源码（src/**.rs + Cargo.toml/Cargo.lock/
/// build.rs）的最新 mtime。bin 落后于它 = 磁盘引擎不含最新编译器行为。
fn newest_engine_source_mtime(manifest: &Path) -> Option<std::time::SystemTime> {
    let mut newest: Option<std::time::SystemTime> = None;
    let mut consider = |p: &Path| {
        if let Ok(m) = p.metadata().and_then(|m| m.modified()) {
            if newest.is_none_or(|n| m > n) {
                newest = Some(m);
            }
        }
    };
    consider(&manifest.join("Cargo.toml"));
    consider(&manifest.join("Cargo.lock"));
    consider(&manifest.join("build.rs"));
    walk_rs_files(&manifest.join("src"), &mut consider);
    newest
}

fn walk_rs_files(
    dir: &Path,
    consider: &mut dyn FnMut(&Path),
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_rs_files(&path, consider);
        } else if path.extension().is_some_and(|e| e == "rs") {
            consider(&path);
        }
    }
}

/// run.stderr 的仓库路径归一：引擎栈回溯/错误渲染携带绝对路径（可能是
/// `\\?\` UNC 规范化形态），逐处剥仓库前缀并把路径段统一为正斜杠——
/// 基线跨 checkout / 跨平台逐字节可比。非路径内容（含 stdout）不触碰。
fn scrub_repo_paths(
    text: &str,
    manifest: &Path,
) -> String {
    let root = normalize_path(manifest);
    let mut out = text.to_string();
    if cfg!(windows) {
        let root_back = root.replace('/', "\\");
        // 长 variant 优先：UNC 形态包含普通形态，先吃掉避免短前缀误配
        for variant in [
            format!(r"\\?\{root_back}"),
            format!("//?/{root}"),
            root_back,
            root,
        ] {
            out = replace_root(&out, &variant);
        }
    } else {
        out = replace_root(&out, &root);
    }
    out
}

/// 把 `text` 里每个 `<root>` 后跟路径分隔符的出现替换为仓库相对段（段内
/// `\`→`/`）。路径段终止于 `:`（行号起点）、括号或串尾；根形字符串后
/// 不接分隔符的出现（非路径）原样保留。
fn replace_root(
    text: &str,
    root: &str,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find(root) {
        let after = &rest[pos + root.len()..];
        if !after.starts_with('\\') && !after.starts_with('/') {
            out.push_str(&rest[..pos + root.len()]);
            rest = after;
            continue;
        }
        out.push_str(&rest[..pos]);
        let bytes = after.as_bytes();
        let mut end = 0usize;
        while end < bytes.len() {
            let c = bytes[end];
            if c == b':' || c == b'(' || c == b')' || c == b'\n' || c == b'\r' {
                break;
            }
            end += 1;
        }
        let span = after[..end]
            .trim_start_matches(['\\', '/'])
            .replace('\\', "/");
        out.push_str(&span);
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}
