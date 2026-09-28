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
