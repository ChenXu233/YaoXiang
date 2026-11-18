// AOT-1 最小原生运行时。此文件在每次原生构建时由系统 C 编译器编译，
// 最终产物只引用这里导出的 C ABI 与目标系统 C 基础库，不引用 Rust/VM/LLVM。
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

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
