//! T2 的**已登记差异与处置**（人工复核过的基线；未登记即视为漂移）。
//!
//! 每条 = 检查号 + 名字 + 事实（reason）+ 处置（action）。6.1「名称规范化」按 action 落地后，
//! 相应条目应从本表删除并让 T2 转全绿——本文件是**待办清单**，不是豁免白名单：
//! 新增差异必须在此登记理由，否则 `--report` 计入「未登记漂移」。
//!
//! **WBS 6.1.2（2026-10-27 定稿）、6.4（2026-10-29）与 P10 S11.1（issue #513）
//! 之后的现状**：三表逐名一致，本表为空。
//! - `from_builtin_name` 已收敛为 15 个内置名；22 个实现内拼写（6.1.2）与
//!   `DateTime` 临时兼容别名（S11.1）均已删除（T2.5 的逐名哨兵守着，任一回退
//!   即报红）。6.0 勘察阶段登记在本表的「补 Never 进 LSP 清单」「删 LSP 的
//!   Int128 / Uint」三条**已按 action 兑现并删除**。
//! - WBS 6.4 按 action 兑现了 10 条 T2.C1：`CONST_PARAM_TYPES` 删掉 10 个旧拼写
//!   （I8/I16/I32/I64/U8/U16/U32/U64/F32/F64）后只剩规范名，C1（本表 ⊆ 内置名表）清零。
//! - P10 S11.1 按 action 兑现了 T2.C2 的 `DateTime` 条目：std.time 的签名串直接写
//!   `Int`，`from_builtin_name` 不再接受 `DateTime`，「LSP 清单不收录 DateTime」
//!   与解析表自然一致，本条按本文件「待办清单」的语义删除。
//! - 与 RFC-011 文档的逐名比对及其实 16 条登记**已按 2026-10-30 用户裁决删除**：RFC-011
//!   是泛型系统设计，不承载内置类型名清单，比对对象不存在（内置名的载体只有 T2 对账的
//!   三张代码侧表）。

/// 一条已登记差异
#[derive(Debug)]
pub struct Disposition {
    /// 检查号：`T2.C1` / `T2.C2` / `T2.C3`
    pub check: &'static str,
    pub name: &'static str,
    pub reason: &'static str,
    pub action: &'static str,
}

/// T2：三表逐名差异
///
/// WBS 6.4：`CONST_PARAM_TYPES` 删掉 10 个旧拼写后，C1（本表 ⊆ 内置名表）已清零，
/// 原先 10 条 T2.C1 登记随之兑现并删除。P10 S11.1（issue #513）迁出 `DateTime` 别名后
/// C2 的唯一条目同样兑现——现本表为空。
pub const T2: &[Disposition] = &[];

/// 按 (检查号, 名字) 查处置
pub fn lookup(
    check: &str,
    name: &str,
) -> Option<&'static Disposition> {
    T2.iter().find(|d| d.check == check && d.name == name)
}

#[cfg(test)]
mod tests {
    use super::{lookup, T2};

    // 登记机制本身仍被 T2 使用（刻意差异靠它登记），故查找必须按 (检查号, 名字) 精确命中，
    // 且不为未登记的 (检查号, 名字) 组合凭空给出处置——否则「未登记漂移」会被掩盖。
    // S11.1（issue #513）兑现 DateTime 条目后本表为空；测试按「当前为空也必须成立、
    // 未来再登记任何条目时仍自洽可查」的形态写。
    #[test]
    fn test_lookup_hits_registered_entry_and_skips_unregistered() {
        // Arrange：未登记组合固定两条（C1 已随 6.4 清零；C2 已随 S11.1 兑现）
        // Act + Assert：未登记组合不得凭空给出处置
        assert!(
            lookup("T2.C1", "Int").is_none(),
            "未登记的组合不得凭空给出处置（否则漂移会被掩盖）"
        );
        assert!(
            lookup("T2.C2", "DateTime").is_none(),
            "T2.C2 的 DateTime 条目已随 P10 S11.1（issue #513）兑现删除，不得再留有处置"
        );
        // Assert：登记表（现为空）里若有任何条目，都必须自洽可查
        for entry in T2 {
            assert!(
                lookup(entry.check, entry.name).is_some(),
                "登记表每一条都必须自洽可查：{} / {}",
                entry.check,
                entry.name
            );
        }
    }
}
