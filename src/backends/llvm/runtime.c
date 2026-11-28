// AOT-1 最小原生运行时。此文件在每次原生构建时由系统 C 编译器编译，
// 最终产物只引用这里导出的 C ABI 与目标系统 C 基础库，不引用 Rust/VM/LLVM。
#include <inttypes.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    const char *file;
    uint64_t file_len;
    uint64_t line;
    uint64_t column;
} YxLocation;

__attribute__((noreturn)) static void yx_fail(
    uint64_t code,
    const char *message,
    uint64_t message_len,
    YxLocation location
) {
    fprintf(stderr, "error[E%04" PRIu64 "]: ", code);
    if (message != NULL) {
        fwrite(message, 1, (size_t)message_len, stderr);
    }
    fputc('\n', stderr);
    if (location.file != NULL && location.file_len != 0) {
        fputs(" --> ", stderr);
        fwrite(location.file, 1, (size_t)location.file_len, stderr);
        fprintf(stderr, ":%" PRIu64 ":%" PRIu64 "\n", location.line, location.column);
    }
    fflush(stderr);
    exit(1);
}

static void yx_overflow(YxLocation location) {
    static const char message[] = "integer overflow";
    yx_fail(6007, message, sizeof(message) - 1, location);
}

static void yx_division_by_zero(YxLocation location) {
    static const char message[] = "division by zero";
    yx_fail(6001, message, sizeof(message) - 1, location);
}

#define YX_LOCATION_ARGS const char *file, uint64_t file_len, uint64_t line, uint64_t column
#define YX_LOCATION ((YxLocation){file, file_len, line, column})

int64_t __yx_rt_add_i64(int64_t lhs, int64_t rhs, YX_LOCATION_ARGS) {
    int64_t result;
    if (__builtin_add_overflow(lhs, rhs, &result)) {
        yx_overflow(YX_LOCATION);
    }
    return result;
}

int64_t __yx_rt_sub_i64(int64_t lhs, int64_t rhs, YX_LOCATION_ARGS) {
    int64_t result;
    if (__builtin_sub_overflow(lhs, rhs, &result)) {
        yx_overflow(YX_LOCATION);
    }
    return result;
}

int64_t __yx_rt_mul_i64(int64_t lhs, int64_t rhs, YX_LOCATION_ARGS) {
    int64_t result;
    if (__builtin_mul_overflow(lhs, rhs, &result)) {
        yx_overflow(YX_LOCATION);
    }
    return result;
}

int64_t __yx_rt_neg_i64(int64_t value, YX_LOCATION_ARGS) {
    int64_t result;
    if (__builtin_sub_overflow((int64_t)0, value, &result)) {
        yx_overflow(YX_LOCATION);
    }
    return result;
}

int64_t __yx_rt_div_i64(int64_t lhs, int64_t rhs, YX_LOCATION_ARGS) {
    if (rhs == 0) {
        yx_division_by_zero(YX_LOCATION);
    }
    if (lhs == INT64_MIN && rhs == -1) {
        yx_overflow(YX_LOCATION);
    }
    return lhs / rhs;
}

int64_t __yx_rt_mod_i64(int64_t lhs, int64_t rhs, YX_LOCATION_ARGS) {
    if (rhs == 0) {
        yx_division_by_zero(YX_LOCATION);
    }
    if (lhs == INT64_MIN && rhs == -1) {
        yx_overflow(YX_LOCATION);
    }
    int64_t result = lhs % rhs;
    if (result != 0 && ((result < 0) != (rhs < 0))) {
        result += rhs;
    }
    return result;
}

void __yx_rt_assert(
    uint64_t condition,
    YX_LOCATION_ARGS,
    const char *message,
    uint64_t message_len
) {
    if (!condition) {
        yx_fail(6005, message, message_len, YX_LOCATION);
    }
}

// ── 受检移位（批次 4.2）────────────────────────────────────────────
//
// 位移量的有效范围与 VM 同口径：先截断成 u32（`b as u32`），只有 < 64 才
// 是有效位移；其余（含负数与 ≥64）走本入口报 E6007，与 VM 的
// `integer overflow in ...` 同码同类。移位本身不做值溢出检查——位被移出不是
// 错误（与 VM 的 checked_shl/checked_shr 一致）。

__attribute__((noreturn)) void __yx_rt_shift_overflow(YX_LOCATION_ARGS) {
    static const char message[] = "integer overflow in shift amount";
    yx_fail(6007, message, sizeof(message) - 1, YX_LOCATION);
}

// ── 控制台输出与码点长度（批次 4.6）──────────────────────────────────
//
// 显示口径与 VM 的 `format_value_with_prefix`（`src/std/io.rs`）逐字节一致：
// Int 十进制、Bool `true`/`false`、String 按显式长度输出原始 UTF-8 字节。
// 全部走 `fwrite` + 显式长度：字符串不以 NUL 结尾，禁止 `%s`/`strlen`/`puts`。
// 换行只由调用方（`println`）通过 `newline` 参数追加一个 `\n`。

void __yx_rt_print_i64(int64_t value, int64_t newline) {
    char buffer[32];
    const int written = snprintf(buffer, sizeof buffer, "%" PRId64, value);
    if (written > 0) {
        fwrite(buffer, 1, (size_t)written, stdout);
    }
    if (newline != 0) {
        fputc('\n', stdout);
    }
}

void __yx_rt_print_bool(int64_t value, int64_t newline) {
    const char *text = value != 0 ? "true" : "false";
    fwrite(text, 1, value != 0 ? 4u : 5u, stdout);
    if (newline != 0) {
        fputc('\n', stdout);
    }
}

void __yx_rt_print_str(const char *ptr, uint64_t len, int64_t newline) {
    if (len != 0) {
        fwrite(ptr, 1, (size_t)len, stdout);
    }
    if (newline != 0) {
        fputc('\n', stdout);
    }
}

// `std.string.len` 的口径是 Unicode 码点数（`src/std/string.rs` 的 `native_len`，
// #385 D1/D2 定案），不是 UTF-8 字节数：合法 UTF-8 里码点数 = 非续接字节数。
int64_t __yx_rt_str_len_codepoints(const char *ptr, uint64_t len) {
    int64_t count = 0;
    for (uint64_t index = 0; index < len; index++) {
        if (((unsigned char)ptr[index] & 0xC0u) != 0x80u) {
            count += 1;
        }
    }
    return count;
}

// ── 浮点 floor 取模（批次 4.5）────────────────────────────────────
//
// 与 VM 的 `crate::util::arith::floor_mod_f64` 逐行同语义：结果符号随除数
// （`-7.5 % 2.0 == 0.5`、`7.5 % -2.0 == -0.5`），**不是** C 的 `fmod` 截断语义。
// 除数为 0 或任一操作数为 NaN 时返回 NaN（与 Rust 的 `a % b` + 符号修正一致）。

double __yx_rt_floor_mod_f64(double lhs, double rhs) {
    const double remainder = fmod(lhs, rhs);
    if (remainder != 0.0 && ((remainder < 0.0) != (rhs < 0.0))) {
        return remainder + rhs;
    }
    return remainder;
}

// ── 字符串（批次 4.1）──────────────────────────────────────────────
//
// 值形状 { const uint8_t *ptr; uint64_t len }：len 是 UTF-8 字节数，不是码点
// 数，也不保证 NUL 结尾。一律按显式长度处理——禁止 strlen/strcmp 一类以 NUL
// 为界的接口。长度为 0 时 ptr 可以是任意值，不得解引用。
//
// 比较语义按 UTF-8 字节字典序；对合法 UTF-8 它与码点序一致，与 VM 侧
// exec_compare 的 String 分支（Rust str 的比较）同结果。内容相等不等于指针
// 相等：两个内容相同的独立字面量是两个全局，必须比较相等。

int64_t __yx_rt_str_eq(
    const uint8_t *lhs,
    uint64_t lhs_len,
    const uint8_t *rhs,
    uint64_t rhs_len
) {
    if (lhs_len != rhs_len) {
        return 0;
    }
    if (lhs_len == 0) {
        return 1;
    }
    return memcmp(lhs, rhs, (size_t)lhs_len) == 0 ? 1 : 0;
}

int64_t __yx_rt_str_cmp(
    const uint8_t *lhs,
    uint64_t lhs_len,
    const uint8_t *rhs,
    uint64_t rhs_len
) {
    const uint64_t common = lhs_len < rhs_len ? lhs_len : rhs_len;
    if (common != 0) {
        const int partial = memcmp(lhs, rhs, (size_t)common);
        if (partial != 0) {
            return partial < 0 ? -1 : 1;
        }
    }
    if (lhs_len == rhs_len) {
        return 0;
    }
    return lhs_len < rhs_len ? -1 : 1;
}
