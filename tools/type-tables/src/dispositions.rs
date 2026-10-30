//! T2 的**已登记差异与处置**（人工复核过的基线；未登记即视为漂移）。
//!
//! 每条 = 检查号 + 名字 + 事实（reason）+ 处置（action）。6.1「名称规范化」按 action 落地后，
//! 相应条目应从本表删除并让 T2 转全绿——本文件是**待办清单**，不是豁免白名单：
//! 新增差异必须在此登记理由，否则 `--report` 计入「未登记漂移」。
//!
//! **WBS 6.1.2（2026-10-27 定稿）与 6.4（2026-10-29）之后的剩余项**：
//! - `from_builtin_name` 已收敛为 15 个内置名 + `DateTime` 临时兼容别名，22 个实现内
//!   拼写已删除（T2.5 的逐名哨兵守着，任一回退即报红）。6.0 勘察阶段登记在本表的
//!   「补 Never 进 LSP 清单」「删 LSP 的 Int128 / Uint」三条**已按 action 兑现并删除**。
//! - WBS 6.4 按 action 兑现了 10 条 T2.C1：`CONST_PARAM_TYPES` 删掉 10 个旧拼写
//!   （I8/I16/I32/I64/U8/U16/U32/U64/F32/F64）后只剩规范名，C1（本表 ⊆ 内置名表）清零。
//! - T2 剩余差异只有一类且属刻意选择：LSP 清单不收录 `DateTime`（待迁出的 std 层别名，
//!   不属内置名）。
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
/// 原先 10 条 T2.C1 登记随之兑现并删除。剩余只有 C2 一条（DateTime 刻意不收录）。
pub const T2: &[Disposition] = &[
    Disposition {
        check: "T2.C2",
        name: "DateTime",
        reason: "LSP 清单只列 15 个内置名，刻意不收录 DateTime——它是 std 层临时兼容别名\
                 （src/std/time.rs 与 src/std/interfaces/time.yx 的签名串仍在用），不属内置名",
        action: "P10 S11（issue #513）把它迁出 from_builtin_name 后本差异自动消失；在此之前不收录是定稿的刻意选择",
    },
];

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
    #[test]
    fn test_lookup_hits_registered_entry_and_skips_unregistered() {
        // Arrange：T2 现存登记里的第一条（(T2.C2, DateTime)）与两个未登记组合
        let first = &T2[0];

        // Act
        let hit = lookup(first.check, first.name);
        let wrong_check = lookup("T2.C1", first.name);
        let wrong_name = lookup(first.check, "Int");

        // Assert
        assert!(
            hit.is_some(),
            "登记过的 {} / {} 必须能被 lookup 命中",
            first.check,
            first.name
        );
        assert!(
            wrong_check.is_none(),
            "T2.C1 已随 WBS 6.4 清零，不得再留有处置"
        );
        assert!(
            wrong_name.is_none(),
            "未登记的名字不得凭空给出处置（否则漂移会被掩盖）"
        );
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
