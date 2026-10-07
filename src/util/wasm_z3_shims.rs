//! wasm32-unknown-unknown 平台胶水：Emscripten 预编译 libz3.a 的 musl/JS 符号 stub（#435）
//!
//! 背景：z3-wasm release 的 libz3.a 由 Emscripten 编译，其 iostream/stdio 体系
//! 引用 musl 的 syscall 底层（__syscall_openat/fcntl64/ioctl）与 JS 库函数
//! （_abort_js、exit）。Emscripten 运行时里这些由 JS 胶水实现；本 crate 目标是
//! wasm32-unknown-unknown（wasm-bindgen），没有那层胶水——此处提供定义使链接通过。
//!
//! 可达性论证：这些符号的调用点全部在 Z3 的文件读写/fatal 路径（filebuf、
//! debug.cpp 日志落盘、memory_manager 的 OOM exit）。编译器求解路径
//! （mk_context → assert → check → model）不做文件 IO，也不调 exit——
//! stub 只需存在，正常路径永不执行；真被调到说明 Z3 走进了文件/fatal 路径，
//! 返回失败码（syscall）或直接 trap（exit/abort）都是可诊断的诚实失败。
//!
//! 签名与 musl wasm32 原型一致（long = i32，LP32）：调用点 wasm 签名固定为
//! 四参 i32（Emscripten 对 variadic 的调用点定形），无需 C variadic。

/// musl stdio 的文件打开底层。返回 -1（失败方向，调用方走错误分支）。
#[no_mangle]
pub extern "C" fn __syscall_openat(
    _dirfd: i32,
    _path: *const u8,
    _flags: i32,
    _mode: i32,
) -> i32 {
    -1
}

/// 文件控制底层。返回 -1。
#[no_mangle]
pub extern "C" fn __syscall_fcntl64(
    _fd: i32,
    _cmd: i32,
    _arg: i32,
) -> i32 {
    -1
}

/// IO 控制底层。返回 -1。
#[no_mangle]
pub extern "C" fn __syscall_ioctl(
    _fd: i32,
    _req: i32,
    _arg: i32,
) -> i32 {
    -1
}

/// Emscripten JS 侧 abort。**调试期可返回**（trap 会拦截后续所有导出调用，
/// 导致 fd_write 捕获的 Z3 fatal 消息不可读）；正式形态评估后收紧。
/// 调用方视其为 noreturn——返回后继续执行属未定义路径，仅用于观测期。
#[no_mangle]
pub extern "C" fn _abort_js() {
    shim_log_write(b"[abort_js called]\n");
}

/// 进程退出。wasm 无进程语义——调试期可返回（理由同 _abort_js）。
#[no_mangle]
pub extern "C" fn exit(code: i32) {
    shim_log_write(format!("[exit({}) called]\n", code).as_bytes());
}

// ── Emscripten JS 库函数 stub（musl/libc++ 内部链的终点）──────────────────
//
// 来源链：Z3 的 timed_mutex/condition_variable（libc++）→ musl sched_yield/
// __tz/sysconf → 这三个 JS 函数。wasm 单线程下锁无竞争、时区对求解无意义、
// 堆上限只被查询——stub 语义对各调用点均安全。

/// Emscripten 单调时钟（毫秒）。逻辑时钟：每次调用 +1——恒 0 会让 Z3 内部
/// 超时判定永不触发（求解无界挂死），逻辑时钟使「采样次数 ≈ 预算」方向正确
/// 且有界。wasm 单线程，Relaxed 足够。
#[no_mangle]
pub extern "C" fn emscripten_get_now() -> f64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static TICK: AtomicU64 = AtomicU64::new(0);
    TICK.fetch_add(1, Ordering::Relaxed) as f64
}

/// 时区设置（musl __tz 的 JS 桥）。时区不影响 SMT 求解——空操作。
#[no_mangle]
pub extern "C" fn _tzset_js(
    _timezone: *mut i32,
    _daylight: *mut i32,
    _std_name: *mut u8,
    _dst_name: *mut u8,
) {
}

/// 堆上限查询（musl sysconf → emmalloc 增长决策）。返回 wasm32 常规上限 2GiB。
#[no_mangle]
pub extern "C" fn emscripten_get_heap_max() -> usize {
    2 * 1024 * 1024 * 1024
}

// ── Emscripten/WASI import 的 wasm 内实现（消除 env/wasi_snapshot_preview1 import）──
//
// 这些符号本是 Emscripten JS 胶水/WASI 宿主函数；wasm32-unknown-unknown
// （wasm-bindgen）形态下没有宿主——在 wasm 内直接实现，让产物回归纯
// wasm-bindgen import 面（浏览器/Node 均无需额外胶水）。

/// emmalloc 堆增长（Emscripten JS 库函数的 wasm 内实现）：memory.grow 直通。
/// 返回 1 成功 / 0 失败——emmalloc 依赖它做真实堆扩展（Z3 求解的分配靠它）。
#[no_mangle]
pub extern "C" fn emscripten_resize_heap(requested: usize) -> i32 {
    let current = core::arch::wasm32::memory_size(0).saturating_mul(65536);
    if requested <= current {
        return 1;
    }
    let pages = (requested - current).div_ceil(65536);
    // memory_grow 返回旧页数；usize::MAX（-1）= 失败
    if core::arch::wasm32::memory_grow(0, pages) == usize::MAX {
        0
    } else {
        1
    }
}

/// WASI 时钟（musl wasm32 的 `__wasi_` 前缀命名）：逻辑时钟（每次 +1ms）写回 time_ptr。恒 0 会让 Z3 超时判定
/// 失效（永不超时=求解无界）；逻辑时钟使采样次数构成有界预算。
#[no_mangle]
pub unsafe extern "C" fn __wasi_clock_time_get(
    _id: i32,
    _precision: i64,
    time: *mut i64,
) -> i32 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static TICK: AtomicU64 = AtomicU64::new(0);
    let now_ns = TICK.fetch_add(1_000_000, Ordering::Relaxed);
    unsafe { *time = now_ns as i64 };
    0
}

/// WASI 环境变量：空环境（count=0, buf_size=0）。
#[no_mangle]
pub unsafe extern "C" fn __wasi_environ_sizes_get(
    count: *mut i32,
    buf_size: *mut i32,
) -> i32 {
    unsafe {
        *count = 0;
        *buf_size = 0;
    }
    0
}

/// WASI 环境变量读取：恒空（配合 environ_sizes_get 的零尺寸）。
#[no_mangle]
pub extern "C" fn __wasi_environ_get(
    _environ: *mut *mut u8,
    _buf: *mut u8,
) -> i32 {
    0
}

/// WASI 文件写：Z3/libc++/musl 初始化期的 stderr/stdout 输出落点。
/// 捕获进固定缓冲，JS 侧经 `shim_log_ptr`/`shim_log_len` 裸导出直读内存
/// （实例化期 panic/abort 消息的唯一观察口——terminated 后 guard 拦不到裸导出）。
/// 报成功（0）防重试风暴。
#[no_mangle]
pub extern "C" fn __wasi_fd_write(
    _fd: i32,
    iovs: *const i32,
    iovs_len: i32,
    nwritten: *mut i32,
) -> i32 {
    let mut total = 0i32;
    for i in 0..iovs_len {
        let (ptr, len) = unsafe { (*iovs.offset(i as isize), *iovs.offset(i as isize + 1)) };
        let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
        shim_log_write(bytes);
        total += len;
    }
    unsafe { *nwritten = total };
    0
}

/// shim 日志固定缓冲（不堆分配——terminated 后 JS 侧经裸导出指针直读内存）。
static mut SHIM_BUF: [u8; 65536] = [0; 65536];
static SHIM_LEN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// 追加字节到 shim 日志（截尾防越界）。
fn shim_log_write(bytes: &[u8]) {
    use std::sync::atomic::Ordering;
    let len = SHIM_LEN.load(Ordering::Relaxed);
    let avail = 65536usize.saturating_sub(len);
    let n = bytes.len().min(avail);
    if n > 0 {
        unsafe {
            let base = std::ptr::addr_of_mut!(SHIM_BUF) as *mut u8;
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), base.add(len), n);
        }
        SHIM_LEN.store(len + n, Ordering::Relaxed);
    }
}

/// 裸导出：日志缓冲指针（wasm-bindgen guard 拦不到 command_export——
/// terminated 后唯一可读通道）。
#[no_mangle]
pub extern "C" fn shim_log_ptr() -> *const u8 {
    std::ptr::addr_of!(SHIM_BUF) as *const u8
}

/// 裸导出：日志长度。
#[no_mangle]
pub extern "C" fn shim_log_len() -> usize {
    use std::sync::atomic::Ordering;
    SHIM_LEN.load(Ordering::Relaxed)
}

/// WASI 文件读：恒 EBADF。
#[no_mangle]
pub extern "C" fn __wasi_fd_read(
    _fd: i32,
    _iovs: *const i32,
    _iovs_len: i32,
    _nread: *mut i32,
) -> i32 {
    9
}

/// WASI 文件关闭：恒成功（无文件语义）。
#[no_mangle]
pub extern "C" fn __wasi_fd_close(_fd: i32) -> i32 {
    0
}

/// WASI 文件定位：恒 ESPIPE（29，不可定位）。
#[no_mangle]
pub extern "C" fn __wasi_fd_seek(
    _fd: i32,
    _offset: i64,
    _whence: i32,
    _newoffset: *mut i64,
) -> i32 {
    29
}

/// Emscripten 3.1.73 musl 时间源的墙钟（Date.now 语义）。逻辑时钟复用
/// emscripten_get_now 的同一计数器——墙钟/单调钟同构即可，求解不依赖绝对时刻。
#[no_mangle]
pub extern "C" fn emscripten_date_now() -> f64 {
    emscripten_get_now()
}

/// Emscripten 3.1.73 单调钟特性查询：恒 1（是单调——与本模块逻辑时钟语义一致）。
#[no_mangle]
pub extern "C" fn _emscripten_get_now_is_monotonic() -> i32 {
    1
}

/// WASI 进程退出（musl exit 的终点）。wasm 无进程语义：记日志后 trap——
/// 正常求解不可达（Z3 fatal 路径），trap 前消息已入 shim 日志可读。
#[no_mangle]
pub extern "C" fn __wasi_proc_exit(code: i32) -> ! {
    shim_log_write(format!("[proc_exit({}) called]\n", code).as_bytes());
    panic!(
        "wasm: proc_exit({}) （Z3 fatal 路径——正常求解不可达）",
        code
    )
}

// ── 全局分配器统一（#435 关键修正）────────────────────────────────────────
//
// 事故实证：Rust wasm32-unknown-unknown 默认 dlmalloc 与 Z3 链入的 Emscripten
// emmalloc **从同一 __heap_base 起点各自管理堆**——两份分配器互不知晓，
// 各自分配的内存区域重叠互踩，表现为随机 misaligned pointer dereference
// （termination.rs:594 的 0x1 地址就是这么来的——栈/堆被踩坏后的次生 panic）。
//
// 解法：Rust 全局分配器桥接到 emmalloc（malloc/free/realloc 由链接的
// libemmalloc.a 提供），全进程单一分配器。dlmalloc 对象因不再被引用而不编入。
extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
    fn realloc(
        ptr: *mut u8,
        size: usize,
    ) -> *mut u8;
    fn memalign(
        align: usize,
        size: usize,
    ) -> *mut u8;
}

/// emmalloc 桥的 GlobalAlloc。emmalloc 的 malloc 保证 8 字节对齐
/// （Emscripten 实测；Rust 侧 align>8 的类型（i128 等）走 memalign——
/// 误判 16 会在 Vec::from_raw_parts_mut 炸 UB 检查）。
struct EmMalloc;

unsafe impl std::alloc::GlobalAlloc for EmMalloc {
    unsafe fn alloc(
        &self,
        layout: std::alloc::Layout,
    ) -> *mut u8 {
        unsafe {
            if layout.align() > 8 {
                memalign(layout.align(), layout.size())
            } else {
                malloc(layout.size())
            }
        }
    }
    unsafe fn dealloc(
        &self,
        ptr: *mut u8,
        _layout: std::alloc::Layout,
    ) {
        unsafe { free(ptr) }
    }
    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: std::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        if layout.align() > 8 {
            // memalign 无 realloc 语义——保守：分配新块+拷贝+释放旧块
            let new_ptr = unsafe { memalign(layout.align(), new_size) };
            if !new_ptr.is_null() {
                unsafe {
                    std::ptr::copy_nonoverlapping(ptr, new_ptr, layout.size().min(new_size));
                    free(ptr);
                }
            }
            return new_ptr;
        }
        unsafe { realloc(ptr, new_size) }
    }
}

#[global_allocator]
static EM_MALLOC: EmMalloc = EmMalloc;
