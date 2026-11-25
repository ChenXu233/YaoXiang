//! 范围模式（RangePattern）端到端行为测试 — issue #405，SPEC syntax §2.8
//!
//! 规范锚（SPEC syntax §2.8「范围模式」引注）：
//! - `lo..hi` 是双闭区间判定：`x 命中 lo..hi ⟺ lo <= x <= hi`（Int 域）
//! - 范围模式受检非兜底：带范围臂的 match 仍需通配/标识符兜底（E1030，由
//!   `src/frontend/core/sema/tests/rfc011b.rs` 单测钉住）
//! - 负端点语法层归一化；或模式备选/守卫失败落下一臂/顺序执行与既有规一致
//!
//! 落位说明：行为锚走集成测试内联源码（test-specification 规则 8.1/7.3），
//! **不落 `tests/yaoxiang/`**——`corpus_probe` 全量枚举该目录，新语料文件
//! 会造成语料差分「新增记录」非空（语言特性新增批次的差分必须零漂移，
//! `tests/baselines/` 禁止改写）；IR 判定链形态由
//! `src/middle/lower/tests/pattern_range.rs` 单测钉住。
//! 同函数体内重复声明会报 E2002（遮蔽是块级语义），故每个绑定独占一名。

use yaoxiang::run;

fn run_ok(source: &str) {
    let source = crate::fixture::with_main_invoked(source);
    run(&source).unwrap_or_else(|e| panic!("Execution failed:\n{:?}\n\nSource:\n{source}", e));
}

/// issue #405 原始夹具：85 分得 B（教学文档示例的合法化）。
/// 值级 oracle 用程序内 assert 表达（跑通即算对，execution.rs 同款）。
#[test]
fn test_range_pattern_issue_fixture_85_prints_b() {
    run_ok(
        r#"
use std.assert

main: () -> Void = {
    grade = match 85 {
        90..99 -> "A",
        80..89 -> "B",
        _ -> "F",
    }
    assert.assert(grade == "B", "85 in 80..89 => B")
}
    "#,
    );
}

/// 双闭端点：下界 80、上界 89 都命中 `80..89`，90 落入下一臂 `90..99`，
/// 79 以下落兜底。
#[test]
fn test_range_pattern_closed_bounds_hit_80_89_pass_90() {
    run_ok(
        r#"
use std.assert

main: () -> Void = {
    at_lo = match 80 {
        90..99 -> "A",
        80..89 -> "B",
        _ -> "F",
    }
    assert.assert(at_lo == "B", "lower bound 80 is included (closed)")

    at_hi = match 89 {
        90..99 -> "A",
        80..89 -> "B",
        _ -> "F",
    }
    assert.assert(at_hi == "B", "upper bound 89 is included (closed)")

    above = match 90 {
        90..99 -> "A",
        80..89 -> "B",
        _ -> "F",
    }
    assert.assert(above == "A", "90 passes 80..89 into 90..99")

    below = match 79 {
        90..99 -> "A",
        80..89 -> "B",
        _ -> "F",
    }
    assert.assert(below == "F", "79 below the range falls to fallback")
}
    "#,
    );
}

/// 负端点：语法层归一化为负 Int，`-5..-1` 双闭含 -5 与 -1。
#[test]
fn test_range_pattern_negative_bounds_normalized() {
    run_ok(
        r#"
use std.assert

main: () -> Void = {
    at_lo = match -5 {
        -5..-1 -> "neg",
        0..10 -> "nonneg",
        _ -> "other",
    }
    assert.assert(at_lo == "neg", "lower bound -5 is included")

    at_hi = match -1 {
        -5..-1 -> "neg",
        0..10 -> "nonneg",
        _ -> "other",
    }
    assert.assert(at_hi == "neg", "upper bound -1 is included (closed)")

    zero = match 0 {
        -5..-1 -> "neg",
        0..10 -> "nonneg",
        _ -> "other",
    }
    assert.assert(zero == "nonneg", "0 outside -5..-1 falls to 0..10")
}
    "#,
    );
}

/// 范围臂作或模式备选：备选顺序尝试，95 落单值臂、91/97 落两个范围备选。
#[test]
fn test_range_pattern_in_or_alternatives() {
    run_ok(
        r#"
use std.assert

main: () -> Void = {
    mid = match 95 {
        90..94 | 96..99 -> "gap",
        95 -> "hit95",
        _ -> "other",
    }
    assert.assert(mid == "hit95", "95 skips the gap alternatives")

    lo = match 91 {
        90..94 | 96..99 -> "gap",
        95 -> "hit95",
        _ -> "other",
    }
    assert.assert(lo == "gap", "91 hits the first range alternative")

    hi = match 97 {
        90..94 | 96..99 -> "gap",
        95 -> "hit95",
        _ -> "other",
    }
    assert.assert(hi == "gap", "97 hits the second range alternative")
}
    "#,
    );
}

/// 范围臂带守卫：守卫失败落下一臂，守卫命中取首臂（RFC-010b 守卫规不变）。
#[test]
fn test_range_pattern_guard_falls_to_next_arm() {
    run_ok(
        r#"
use std.assert

threshold = 85

main: () -> Void = {
    low = match 83 {
        80..89 if 83 >= threshold -> "high B",
        80..89 -> "low B",
        _ -> "other",
    }
    assert.assert(low == "low B", "guard failure falls to next arm")

    high = match 88 {
        80..89 if 88 >= threshold -> "high B",
        80..89 -> "low B",
        _ -> "other",
    }
    assert.assert(high == "high B", "guard success takes the first arm")
}
    "#,
    );
}

/// 顺序执行：范围臂在前先命中；与字面量臂共存互不干扰。
#[test]
fn test_range_pattern_order_and_coexistence_with_literals() {
    run_ok(
        r#"
use std.assert

main: () -> Void = {
    first = match 70 {
        60..79 -> "range hit",
        70 -> "literal hit",
        _ -> "other",
    }
    assert.assert(first == "range hit", "first matching arm wins")

    in_range = match 200 {
        404 -> "Not Found",
        200..299 -> "Success",
        _ -> "Unknown",
    }
    assert.assert(in_range == "Success", "200 in 200..299")

    on_literal = match 404 {
        404 -> "Not Found",
        200..299 -> "Success",
        _ -> "Unknown",
    }
    assert.assert(on_literal == "Not Found", "404 keeps its literal arm")
}
    "#,
    );
}
