//! 测试资源画像的执行侧助手 — `cost: mode=exclusive` 测试体的独占文件锁（#486 第 1/2 项）
//!
//! 为什么锁在测试体内、而不是靠 CI 配置：`cargo test` 的 `--test-threads` 只约束
//! **同一次调用内**的并发；两个进程之间（两个 worktree 的重测、一次 `run fast` 与一次
//! 手工 `cargo test`）没有任何互斥。8 GiB 档的重测一旦与另一路重测撞车，门禁照样 OOM。
//! 故独占性必须写在**测试体自己**身上：本助手在测试体开头取一把跨进程文件锁，拿不到
//! 就轮询等到截止时间，超时即 panic（不静默跳过、不降级为 defer）。
//!
//! 锁文件 = 多 worktree 共享的 `target/test-budget/<test-name>.lock`：路径优先由
//! `git rev-parse --path-format=absolute --git-common-dir` 的父目录解析（worktree 的
//! common-dir 一律指向主仓 `.git`，故主仓与全部 worktree 共用同一把锁），git 不可用时
//! 退回 `CARGO_MANIFEST_DIR/target/`。
//!
//! 观测面：等待与持有都**直写** `stderr`，不经 `eprintln!`——libtest 默认捕获宏输出，
//! 而「有没有并发者、等了多久」正是本助手唯一要留下的证据，必须在批量调用（不加
//! `--nocapture`）里也可见。
//!
//! 预算通道（`scripts/dev/testsuite.py`）读的是同一份就地声明的 `cost:` 标记；本助手
//! 只负责「已经开跑之后不再与并发者抢内存」，两者合起来才是 #486 ② 的「保证有效」。

use std::fs::{File, OpenOptions, TryLockError};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// 默认等待上限（秒）：`YX_TEST_LOCK_TIMEOUT` 可整秒覆盖。
pub const DEFAULT_LOCK_TIMEOUT_SECS: u64 = 600;

/// 轮询间隔：够密以尽快接上对方释放，又不至于把 CPU 烧在 `try_lock` 上。
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// 独占锁守卫：持有期间锁文件被本进程锁定，Drop 时释放（文件关闭即解锁）。
pub struct ExclusiveGuard {
    file: File,
    path: PathBuf,
    test_name: String,
    waited: Duration,
}

impl ExclusiveGuard {
    /// 锁文件绝对路径
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 测试名（锁文件名去掉 `.lock` 之前的原始名）
    pub fn test_name(&self) -> &str {
        &self.test_name
    }

    /// 从开始等待到实际持有之间的时长
    pub fn waited(&self) -> Duration {
        self.waited
    }
}

impl Drop for ExclusiveGuard {
    fn drop(&mut self) {
        // 显式解锁；即便此处失败，进程退出 / 文件关闭时内核也会释放。
        let _ = self.file.unlock();
    }
}

/// 取 `mode=exclusive` 测试的独占文件锁；等到即打印 `held <路径> (waited Xs)` 后返回。
///
/// 语义（#486 ②）：
/// - 轮询 `try_lock` 到截止时间（默认 600 s，`YX_TEST_LOCK_TIMEOUT` 以整秒覆盖）
/// - 第一次撞锁时打印 `blocked on <路径>`——并发归属的唯一线索
/// - 超时 **panic**，消息里带锁路径与并发提示：不静默跳过、不降级为 defer
pub fn exclusive_lock(test_name: &str) -> ExclusiveGuard {
    assert!(
        !test_name.trim().is_empty(),
        "exclusive_lock 需要非空测试名（锁文件名由它派生）"
    );
    let timeout = lock_timeout();
    let path = lock_path(test_name);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .unwrap_or_else(|e| panic!("独占锁目录无法创建：{}（{e}）", dir.display()));
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .unwrap_or_else(|e| panic!("独占锁文件无法打开：{}（{e}）", path.display()));

    let start = Instant::now();
    let mut announced_block = false;
    loop {
        match file.try_lock() {
            Ok(()) => break,
            Err(TryLockError::WouldBlock) => {
                if !announced_block {
                    announced_block = true;
                    note(&format!(
                        "[test-support] {test_name}: blocked on {} (waiting up to {}s)",
                        path.display(),
                        timeout.as_secs()
                    ));
                }
                let waited = start.elapsed();
                if waited >= timeout {
                    panic!(
                        "等待独占锁超时（{} s）：{}\n\
                         并发提示：同一主仓下的另一个进程（另一 worktree 的重测，或另一路 plan/run）正持有该锁。\n\
                         该测试的 `cost:` 标记声明了 mode=exclusive（峰值内存装不下第二个实例），\
                         故这里既不降级为并发执行、也不静默跳过。\n\
                         处置：等对方结束；确需延长等待，设 YX_TEST_LOCK_TIMEOUT=<整秒>。",
                        timeout.as_secs(),
                        path.display()
                    );
                }
                std::thread::sleep(POLL_INTERVAL.min(timeout - waited));
            }
            Err(TryLockError::Error(e)) => {
                panic!("独占锁获取失败：{}（{e}）", path.display())
            }
        }
    }
    let waited = start.elapsed();
    note(&format!(
        "[test-support] {test_name}: held {} (waited {:.1}s)",
        path.display(),
        waited.as_secs_f64()
    ));
    ExclusiveGuard {
        file,
        path,
        test_name: test_name.to_string(),
        waited,
    }
}

/// 等待上限：`YX_TEST_LOCK_TIMEOUT`（整秒）优先，非法值直接 panic（不静默退回默认）。
fn lock_timeout() -> Duration {
    match std::env::var("YX_TEST_LOCK_TIMEOUT") {
        Ok(raw) if !raw.trim().is_empty() => {
            let secs: u64 = raw
                .trim()
                .parse()
                .unwrap_or_else(|_| panic!("YX_TEST_LOCK_TIMEOUT 必须是整秒数，实际为 {raw:?}"));
            Duration::from_secs(secs)
        }
        _ => Duration::from_secs(DEFAULT_LOCK_TIMEOUT_SECS),
    }
}

/// 锁文件路径：`<共享 target>/test-budget/<test-name>.lock`
fn lock_path(test_name: &str) -> PathBuf {
    lock_dir().join(format!("{}.lock", sanitize_name(test_name)))
}

/// 文件名安全化：测试名通常是标识符，这里只把非法字符换成 `_` 以防跨平台踩坑。
fn sanitize_name(test_name: &str) -> String {
    test_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// 多 worktree 共享的锁目录
fn lock_dir() -> PathBuf {
    if let Some(dir) = git_common_lock_dir() {
        return dir;
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test-budget")
}

/// 由 `git rev-parse --path-format=absolute --git-common-dir` 的父目录解析锁目录；
/// git 不可用 / 非仓库时返回 `None`（调用方退回 `CARGO_MANIFEST_DIR/target/`）。
fn git_common_lock_dir() -> Option<PathBuf> {
    // 全仓唯一允许构造 git 子进程的入口（`package::source::tests::git` 的
    // `test_git_spawn_only_through_git_command` 守卫会拒绝裸 spawn）：它同时剥离
    // 继承来的 GIT_DIR/GIT_COMMON_DIR 等定位变量，免得锁目录被钩子环境重定向。
    let mut command = crate::package::source::git::git_command();
    let output = command
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let common = PathBuf::from(raw);
    // worktree 的 common-dir 指向主仓 .git；其父目录即主仓根 → 与全部 worktree 共享。
    let root = if common.file_name().map(|n| n == ".git").unwrap_or(false) {
        common.parent()?.to_path_buf()
    } else {
        common
    };
    Some(root.join("target").join("test-budget"))
}

/// 直写 stderr（绕过 libtest 的宏捕获，见模块文档「观测面」）
fn note(line: &str) {
    let mut stderr = std::io::stderr();
    let _ = writeln!(stderr, "{line}");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 子进程助手的完整测试路径（libtest `--exact` 用）
    const CHILD_HELPER: &str =
        "util::test_support::tests::test_exclusive_lock_child_process_helper";

    /// 子进程助手：被同文件的两条跨进程判据以 `--exact` 唤起，只做「取锁 → 退出」。
    /// 在常规套件里也会被执行一次（无并发者时立即拿到锁，成本可忽略）。
    #[test]
    fn test_exclusive_lock_child_process_helper() {
        // Arrange
        let name = std::env::var("YX_TEST_LOCK_NAME")
            .unwrap_or_else(|_| format!("selftest_child_{}", std::process::id()));

        // Act
        let guard = exclusive_lock(&name);

        // Assert
        assert!(
            guard.path().is_file(),
            "子进程也应落地锁文件：{}",
            guard.path().display()
        );
    }

    /// 判据 ② 的路径面：锁文件落在共享 `target/test-budget/`，命名为 `<test-name>.lock`，
    /// 且释放后可立刻重新获得（无残留）。
    #[test]
    fn test_exclusive_lock_uses_shared_target_lock_file() {
        // Arrange
        let name = format!("selftest_acquire_{}", std::process::id());

        // Act
        let guard = exclusive_lock(&name);
        let path = guard.path().to_path_buf();
        let waited = guard.waited();
        drop(guard);
        let again = exclusive_lock(&name);

        // Assert
        assert!(
            path.is_file(),
            "锁文件应落盘且路径形如 target/test-budget/<name>.lock：{}",
            path.display()
        );
        assert!(
            path.to_string_lossy()
                .replace('\\', "/")
                .contains("/target/test-budget/"),
            "锁文件必须位于共享 target/test-budget/ 下：{}",
            path.display()
        );
        assert!(
            path.to_string_lossy().ends_with(&format!("{name}.lock")),
            "锁文件必须以 <test-name>.lock 命名：{}",
            path.display()
        );
        assert_eq!(again.test_name(), name, "守卫应回报原始测试名");
        assert!(
            waited < Duration::from_secs(1),
            "无并发者时不应等待，实际 waited {:?}",
            waited
        );
        assert!(
            again.waited() < Duration::from_secs(1),
            "释放后重新获取应立即可得，实际 waited {:?}",
            again.waited()
        );
    }

    /// 判据 ② 的跨进程面：父进程持锁时，子进程必须先 `blocked`、等父进程放手后
    /// `held`（打印 waited Xs）。2 s 持有窗口远大于进程启动开销，避免假阴性。
    #[test]
    fn test_exclusive_lock_second_process_blocks_then_reports_held() {
        // Arrange
        let name = format!("selftest_two_process_{}", std::process::id());
        let parent = exclusive_lock(&name);
        let exe = std::env::current_exe().expect("应当能取到本测试二进制路径");

        // Act
        let child = std::process::Command::new(&exe)
            .args(["--exact", CHILD_HELPER, "--test-threads=1", "--nocapture"])
            .env("YX_TEST_LOCK_NAME", &name)
            .env("YX_TEST_LOCK_TIMEOUT", "60")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("应当能唤起本测试二进制的子进程");
        std::thread::sleep(Duration::from_secs(2));
        drop(parent);
        let output = child.wait_with_output().expect("子进程应当正常结束");

        // Assert
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "子进程拿到锁后应成功退出，实际状态 {:?}，stderr：{stderr}",
            output.status
        );
        assert!(
            stderr.contains("blocked on"),
            "子进程应先报 blocked（证明真的撞上父进程的锁）：{stderr}"
        );
        assert!(stderr.contains("held"), "子进程拿到锁后应报 held：{stderr}");
        assert!(
            stderr.contains("waited"),
            "子进程应打印 waited Xs：{stderr}"
        );
        assert!(
            stderr.contains(&format!("{name}.lock")),
            "子进程应打印锁路径（含 <test-name>.lock）：{stderr}"
        );
        // 观测面：把子进程的 held/blocked 行转写到本进程 stderr —— 加 --nocapture
        // 时，这两行就是「两个并发进程争抢同一把锁（一个 held、另一个 blocked）」的
        // 可核证据，而不是只存在于断言里的字串。
        for line in stderr.lines().filter(|l| l.contains("[test-support]")) {
            note(line);
        }
    }

    /// 判据 ② 的失败面：等到截止时间必须 panic（带锁路径与并发提示），
    /// 不得静默跳过、不得降级为 defer。
    #[test]
    fn test_exclusive_lock_timeout_panics_with_path_and_concurrency_hint() {
        // Arrange
        let name = format!("selftest_timeout_{}", std::process::id());
        let parent = exclusive_lock(&name);
        let exe = std::env::current_exe().expect("应当能取到本测试二进制路径");

        // Act
        let output = std::process::Command::new(&exe)
            .args(["--exact", CHILD_HELPER, "--test-threads=1", "--nocapture"])
            .env("YX_TEST_LOCK_NAME", &name)
            .env("YX_TEST_LOCK_TIMEOUT", "0")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .expect("应当能唤起本测试二进制的子进程");
        drop(parent);

        // Assert
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "超时必须让子进程失败（panic），实际状态 {:?}，stderr：{stderr}",
            output.status
        );
        assert!(stderr.contains("等待独占锁超时"), "应报超时：{stderr}");
        assert!(
            stderr.contains(&format!("{name}.lock")),
            "超时消息必须打印锁路径：{stderr}"
        );
        assert!(
            stderr.contains("并发提示"),
            "超时消息必须打印并发提示：{stderr}"
        );
        // 观测面：把子进程的 panic 行转写到本进程 stderr（--nocapture 时可见锁路径与提示）
        for line in stderr
            .lines()
            .filter(|l| l.contains("[test-support]") || l.contains("等待独占锁超时"))
        {
            note(line);
        }
    }
}
