//! #452 / #454 / t12-F1：命名实参与位置实参的**槽位绑定判据**（唯一权威点）。
//!
//! 语言语义（用户 2026-10-14 裁决，主流约定，**禁止静默择一**）：
//! - ① 位置实参按声明顺序占前导槽位（第 i 个 → 值形参 i）；
//! - ② 命名实参按形参名入槽；名字不在形参表 → E1014；
//! - ③ 位置与命名落在同一槽位 → E1111（不许命名覆盖位置，也不许丢掉位置实参）；
//! - ④ 命名实参之后不得再出现位置实参 → E1111；
//! - ⑤ 同一形参被重复命名 → E1015（唯一「字面重复」的一类）；
//! - ⑥ 必需形参必须全部有实参 → E1010。
//!
//! 此前这套判据在**三处**各写一份（方法与原生在 typecheck 各一份、ir_gen 内又
//! 两份），且已实体分歧：规则④只在方法级生效——方法 \`t.rep(new = "X", "aaa", "a")\`
//! 报 E1111，原生同形（\`string.replace\`）静默接受。本模块是收敛后的**唯一实现**。
//!
//! 形参表坐标：\`param_names\` 含**接收者位**（方法形态下标 0；原生/普通函数形态
//! 由 \`recv_offset = 1\` 声明首位是接收者位）。返回值是**逐实参的形参位**（顺序 =
//! 位置实参在前、命名实参按书写序）。

use crate::frontend::core::parser::ast::Expr;
use crate::util::diagnostic::{ErrorCodeDefinition, Result};
use crate::util::span::Span;

/// 把一次调用的实参绑定到形参槽位，并当场校验六条规则。
///
/// - \`key\`：诊断里显示的调用名（方法键 / 限定名 / 短名）
/// - \`param_names\`：声明序形参名，**含接收者位**（见模块文档）
/// - \`recv_offset\`：接收者位占几个形参槽（方法 1；原生/普通函数 0）
/// - \`optional\`：与形参表等长的可选位标记（原生 \`?name\` / 变参尾位为 true）
/// - 返回：逐实参的形参位（长度 = 位置实指数 + 命名实指数）
pub fn bind_args_to_params(
    key: &str,
    param_names: &[String],
    args: &[Expr],
    named_args: &[(String, Expr)],
    recv_offset: usize,
    optional: &[bool],
    call_span: Span,
) -> Result<Vec<usize>> {
    let total_params = param_names.len();
    let mut filled: Vec<bool> = vec![false; total_params];
    let mut positions: Vec<usize> = Vec::with_capacity(args.len() + named_args.len());

    // ① 位置实参按声明序占前导值形参槽位
    for i in 0..args.len() {
        let slot = recv_offset + i;
        if slot < total_params {
            filled[slot] = true;
        }
        positions.push(slot);
    }

    // ④ 命名实参之后不得再出现位置实参。解析器按书写序收集：表现成
    //    「args 非空且首个命名实参的源码位置早于最后一个位置实参」。
    if let (Some(last), Some((first_named_name, first_named))) = (args.last(), named_args.first()) {
        if first_named.span().start.offset < last.span().start.offset {
            return Err(
                ErrorCodeDefinition::argument_order_invalid(key, first_named_name)
                    .at(call_span)
                    .build(),
            );
        }
    }

    let mut seen_named: Vec<&str> = Vec::with_capacity(named_args.len());
    for (name, _) in named_args {
        // ② 名字必须是某个**可命名**的值形参（接收者位与空串占位位不在候选内）
        let Some(slot) = param_names
            .iter()
            .enumerate()
            .skip(recv_offset)
            .find(|(_, p)| !p.is_empty() && p.as_str() == name)
            .map(|(i, _)| i)
        else {
            return Err(ErrorCodeDefinition::unknown_argument(
                key,
                name,
                &param_names
                    .iter()
                    .skip(recv_offset)
                    .filter(|p| !p.is_empty())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", "),
            )
            .at(call_span)
            .build());
        };
        // ⑤ 同一形参被重复命名（唯一字面重复的一类）
        if seen_named.contains(&name.as_str()) {
            return Err(ErrorCodeDefinition::duplicate_argument(key, name)
                .at(call_span)
                .build());
        }
        seen_named.push(name.as_str());
        // ③ 该槽位已被位置实参占用 = 一个形参收到多个实参
        if filled[slot] {
            return Err(ErrorCodeDefinition::argument_order_invalid(key, name)
                .at(call_span)
                .build());
        }
        filled[slot] = true;
        positions.push(slot);
    }

    // ⑥ 必需位必须全部有实参（可选位允许缺席，但不得张冠李戴）
    let provided = args.len() + named_args.len();
    let missing_names: Vec<&str> = param_names
        .iter()
        .enumerate()
        .skip(recv_offset)
        .filter(|(i, _)| !filled.get(*i).copied().unwrap_or(false))
        .filter(|(i, _)| !optional.get(*i).copied().unwrap_or(false))
        .map(|(_, name)| name.as_str())
        .collect();
    if !missing_names.is_empty() {
        let value_count = total_params.saturating_sub(recv_offset);
        return Err(
            ErrorCodeDefinition::argument_count_mismatch(key, value_count, provided)
                .at(call_span)
                .with_help(format!(
                    "缺少必需形参：{}（命名实参只按形参名入槽，不占前导位置）",
                    missing_names.join(", ")
                ))
                .build(),
        );
    }
    Ok(positions)
}
