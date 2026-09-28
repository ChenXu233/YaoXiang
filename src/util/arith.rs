//! 算术原语——语言语义要求的、与具体层无关的数值函数。
//!
//! 存在的理由：`%` 的 floor 取模语义由 RFC-011b 定案（结果符号跟随除数，
//! Python 语义），而它需要出现在**五个**独立位置——常量折叠（IR 生成）、
//! 解释器整型/浮点臂、编译期求值（const_eval / operations）。此前每处各写
//! 一份内联实现（两处甚至逐字节相同的独立函数），改语义时容易漏改。
//! RFC-011b 的规格说「三处同步修改」，实际是五处——本模块是那「一处」。

/// 可做 floor 取模的带符号整数（`i64` / `i128`——运行时值与编译期值各用其一）。
pub trait SignedInt: Copy {
    fn checked_rem_int(
        self,
        rhs: Self,
    ) -> Option<Self>;
    fn checked_add_int(
        self,
        rhs: Self,
    ) -> Option<Self>;
    fn is_neg(self) -> bool;
    fn is_zero(self) -> bool;
}

macro_rules! impl_signed_int {
    ($($t:ty),*) => { $(impl SignedInt for $t {
        fn checked_rem_int(self, rhs: Self) -> Option<Self> { self.checked_rem(rhs) }
        fn checked_add_int(self, rhs: Self) -> Option<Self> { self.checked_add(rhs) }
        fn is_neg(self) -> bool { self < 0 }
        fn is_zero(self) -> bool { self == 0 }
    })* };
}
impl_signed_int!(i64, i128);

/// 整型 floor 取模（结果符号跟随除数）。
///
/// `-7 % 3 == 2`（截断余数会给 `-1`），`7 % -3 == -2`。
/// 除数语义由调用方保证（`b == 0` 应在调用前拦截并报错）。
///
/// 返回 `None` 仅当 `checked_rem` 溢出（`MIN % -1`）或调整时 `checked_add`
/// 溢出——调用方据此报整数溢出，而非 panic。
pub fn floor_mod_signed<T: SignedInt>(
    a: T,
    b: T,
) -> Option<T> {
    let r = a.checked_rem_int(b)?;
    if !r.is_zero() && r.is_neg() != b.is_neg() {
        r.checked_add_int(b)
    } else {
        Some(r)
    }
}

/// `i64` 便捷入口（运行时整型值）。
pub fn floor_mod_i64(
    a: i64,
    b: i64,
) -> Option<i64> {
    floor_mod_signed(a, b)
}

/// 浮点 floor 取模（结果符号跟随除数），与 [`floor_mod_i64`] 同语义。
///
/// 不用 `rem_euclid`：那是**恒非负**的欧几里得取模（`7.0 % -3.0 == 1.0`），
/// 不符合语言参考「乘除取模」的符号约定。
pub fn floor_mod_f64(
    a: f64,
    b: f64,
) -> f64 {
    let r = a % b;
    if r != 0.0 && (r < 0.0) != (b < 0.0) {
        r + b
    } else {
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC-011b §`%` 语义修正：符号跟随除数（Python 语义）。
    /// 语料是 RFC 里逐条列出的例子，防回归到截断余数。
    #[test]
    fn test_floor_mod_int_sign_follows_divisor() {
        assert_eq!(floor_mod_i64(-7, 3), Some(2), "-7 % 3 == 2");
        assert_eq!(floor_mod_i64(7, -3), Some(-2), "7 % -3 == -2");
        assert_eq!(floor_mod_i64(-7, -3), Some(-1), "-7 % -3 == -1");
        assert_eq!(floor_mod_i64(7, 3), Some(1), "7 % 3 == 1");
        assert_eq!(floor_mod_i64(-6, 3), Some(0), "整除结果恒 0");
    }

    /// 溢出不得 panic：`MIN % -1` 是整型溢出的真实边界。
    #[test]
    fn test_floor_mod_int_overflow_returns_none() {
        assert_eq!(
            floor_mod_i64(i64::MIN, -1),
            None,
            "MIN % -1 必须报溢出而非 panic"
        );
    }

    /// 浮点同语义（含负数与小数）。
    #[test]
    fn test_floor_mod_float_sign_follows_divisor() {
        assert_eq!(floor_mod_f64(-7.5, 3.0), 1.5, "-7.5 % 3.0 == 1.5");
        assert_eq!(floor_mod_f64(2.5, -1.5), -0.5, "2.5 % -1.5 == -0.5");
        assert_eq!(floor_mod_f64(7.0, 3.0), 1.0, "7.0 % 3.0 == 1.0");
    }

    /// 两实现的语义必须一致（整型结果提升到 f64 后与浮点版相等）。
    #[test]
    fn test_int_and_float_implementations_agree() {
        for a in -7i64..=7 {
            for b in [-3i64, -2, 2, 3] {
                let i = floor_mod_i64(a, b).expect("非溢出组合");
                let f = floor_mod_f64(a as f64, b as f64);
                assert_eq!(i as f64, f, "整型与浮点 floor 取模必须同语义：{a} % {b}");
            }
        }
    }
}
