#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""check-synth-boundary.py — RFC-039 D22 / WBS 9.8 AST 合成边界门禁。

扫描 01-routing 对应的 L3/L4 生产面：sema（现 frontend/core/sema 与
frontend/core/types）、proof、middle、backends 与 std。唯一允许合成
ast::Expr / ast::Stmt / ast::Type 的文件是 src/middle/lower/synth.rs。

判据只接受表达式位置的构造，不把 match / let / matches! 的模式、关联查询、
注释、字符串、tests/ 路径或 #[cfg(test)] / #[test] 项误报为构造。--selftest
在同一进程内构造临时正反倒例，证明真构造会失败、非构造读取保持干净。
"""

from __future__ import annotations

import argparse
import io
import re
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCAN_DIRS = (
    "src/frontend/core/sema",
    "src/frontend/core/types",
    "src/sema",
    "src/proof",
    "src/middle",
    "src/backends",
    "src/std",
)
SYNTH_FILE = "src/middle/lower/synth.rs"
CANONICAL = {"Expr", "Stmt", "Type"}


@dataclass(frozen=True)
class Token:
    text: str
    start: int
    end: int


def sanitize(source: str) -> str:
    """空白化注释与字面量，保留长度、换行和 Rust 代码分隔符。"""
    chars = list(source)
    result = list(source)
    i = 0
    block_depth = 0
    while i < len(chars):
        if block_depth:
            if source.startswith("/*", i):
                result[i] = result[i + 1] = " "
                block_depth += 1
                i += 2
            elif source.startswith("*/", i):
                result[i] = result[i + 1] = " "
                block_depth -= 1
                i += 2
            else:
                if chars[i] != "\n":
                    result[i] = " "
                i += 1
            continue
        if source.startswith("//", i):
            while i < len(chars) and chars[i] != "\n":
                result[i] = " "
                i += 1
            continue
        if source.startswith("/*", i):
            result[i] = result[i + 1] = " "
            block_depth = 1
            i += 2
            continue

        raw = re.match(r"(?:b?r)(#+)\"", source[i:])
        if raw:
            prefix_len = len(raw.group(0))
            hashes = raw.group(1)
            terminator = '\"' + hashes
            end = source.find(terminator, i + prefix_len)
            end = len(chars) if end < 0 else end + len(terminator)
            for j in range(i, end):
                if chars[j] != "\n":
                    result[j] = " "
            i = end
            continue
        if source.startswith('r"', i) or source.startswith('br"', i):
            prefix_len = 2 if source.startswith('r"', i) else 3
            end = source.find('"', i + prefix_len)
            end = len(chars) if end < 0 else end + 1
            for j in range(i, end):
                if chars[j] != "\n":
                    result[j] = " "
            i = end
            continue
        if chars[i] == '"' or (
            chars[i] in "bc" and i + 1 < len(chars) and chars[i + 1] == '"'
        ):
            start = i + (1 if chars[i] in "bc" else 0)
            j = start + 1
            while j < len(chars):
                if chars[j] == "\\":
                    j += 2
                    continue
                if chars[j] == '"':
                    j += 1
                    break
                j += 1
            for k in range(i, min(j, len(chars))):
                if chars[k] != "\n":
                    result[k] = " "
            i = j
            continue
        if chars[i] == "'":
            lifetime = re.match(r"'[A-Za-z_][A-Za-z0-9_]*", source[i:])
            if lifetime and i + len(lifetime.group(0)) < len(chars) and chars[i + len(lifetime.group(0))] != "'":
                i += len(lifetime.group(0))
                continue
            j = i + 1
            while j < len(chars):
                if chars[j] == "\\":
                    j += 2
                    continue
                if chars[j] == "'" or chars[j] == "\n":
                    j += 1
                    break
                j += 1
            for k in range(i, min(j, len(chars))):
                if chars[k] != "\n":
                    result[k] = " "
            i = j
            continue
        i += 1
    return "".join(result)


def tokenize(code: str) -> list[Token]:
    return [
        Token(match.group(0), match.start(), match.end())
        for match in re.finditer(r"[A-Za-z_][A-Za-z0-9_]*|\d+|::|=>|[^\s]", code)
    ]


def delimiter_pairs(tokens: list[Token]) -> dict[int, int]:
    pairs: dict[int, int] = {}
    stack: list[tuple[str, int]] = []
    closing = {")": "(", "]": "[", "}": "{", ">": "<"}
    for index, token in enumerate(tokens):
        if token.text in ("(", "[", "{"):
            stack.append((token.text, index))
        elif token.text in (")", "]", "}"):
            if not stack or stack[-1][0] != closing[token.text]:
                continue
            _, opening = stack.pop()
            pairs[opening] = index
            pairs[index] = opening
    return pairs


def test_item_ranges(code: str, tokens: list[Token]) -> list[tuple[int, int]]:
    pairs = delimiter_pairs(tokens)
    ranges: list[tuple[int, int]] = []
    attribute = re.compile(
        r"#\s*\[\s*(?:cfg\s*\(\s*test\s*\)|test)\s*\]"
    )
    for match in attribute.finditer(code):
        later = [index for index, token in enumerate(tokens) if token.start >= match.end()]
        opening = next((index for index in later if tokens[index].text == "{"), None)
        if opening is None or opening not in pairs:
            continue
        semicolon = next((index for index in later if tokens[index].text == ";"), None)
        if semicolon is not None and semicolon < opening:
            continue
        closing = pairs[opening]
        ranges.append((match.start(), tokens[closing].end))
    return ranges


def pattern_mask(tokens: list[Token], pairs: dict[int, int]) -> list[bool]:
    """标记 Rust 模式 token；表达式位置的同形构造不在掩码内。"""
    mask = [False] * len(tokens)

    def mark(start: int, end: int) -> None:
        for index in range(max(0, start), min(len(tokens), end)):
            mask[index] = True

    for index, token in enumerate(tokens):
        if token.text == "let":
            cursor = index + 1
            while cursor < len(tokens):
                if cursor in pairs and tokens[cursor].text in ("(", "[", "{"):
                    cursor = pairs[cursor] + 1
                    continue
                if tokens[cursor].text in ("=", ";"):
                    break
                cursor += 1
            if cursor < len(tokens) and tokens[cursor].text == "=":
                mark(index + 1, cursor)
        elif token.text == "matches" and index + 2 < len(tokens):
            if tokens[index + 1].text != "!" or tokens[index + 2].text != "(":
                continue
            opening = index + 2
            if opening not in pairs:
                continue
            closing = pairs[opening]
            cursor = opening + 1
            while cursor < closing:
                if cursor in pairs and tokens[cursor].text in ("(", "[", "{"):
                    cursor = pairs[cursor] + 1
                    continue
                if tokens[cursor].text == ",":
                    mark(cursor + 1, closing)
                    break
                cursor += 1
        elif token.text == "match":
            cursor = index + 1
            opening = None
            while cursor < len(tokens):
                if cursor in pairs and tokens[cursor].text in ("(", "["):
                    cursor = pairs[cursor] + 1
                    continue
                if tokens[cursor].text == "{":
                    opening = cursor
                    break
                if tokens[cursor].text == ";":
                    break
                cursor += 1
            if opening is None or opening not in pairs:
                continue
            closing = pairs[opening]
            arm_start = opening + 1
            cursor = arm_start
            in_arm_body = False
            while cursor < closing:
                if cursor in pairs and tokens[cursor].text in ("(", "[", "{"):
                    cursor = pairs[cursor] + 1
                    if (
                        in_arm_body
                        and tokens[cursor - 1].text == "}"
                        and (cursor >= len(tokens) or tokens[cursor].text != "else")
                    ):
                        # if/else 的 then 块不是整个臂体；只在 else 不再延续时复位。
                        arm_start = cursor
                        in_arm_body = False
                    continue
                if tokens[cursor].text == "=>":
                    mark(arm_start, cursor)
                    in_arm_body = True
                elif tokens[cursor].text == ",":
                    arm_start = cursor + 1
                    in_arm_body = False
                cursor += 1
    return mask


def ast_names(code: str) -> tuple[dict[str, str], bool, set[str]]:
    """返回合法 Rust use 导入形成的 AST 名称、glob 状态与模块别名。"""
    names: dict[str, str] = {}
    glob_imported = False
    ast_roots = {"ast", "crate::ast", "self::ast", "super::ast"}

    def use_items(tail: str) -> list[str]:
        """按组内层级拆分 use 项，丢弃尾随逗号产生的空项。"""
        tail = tail.strip()
        if not tail:
            return []
        if not (tail.startswith("{") and tail.endswith("}")):
            return [tail]
        inner = tail[1:-1]
        parts: list[str] = []
        start = 0
        depth = 0
        for index, char in enumerate(inner):
            if char == "{":
                depth += 1
            elif char == "}":
                depth -= 1
            elif char == "," and depth == 0:
                parts.append(inner[start:index])
                start = index + 1
        parts.append(inner[start:])
        return [item for part in parts for item in use_items(part) if item.strip()]

    use_groups = [
        (root, use_items(tail))
        for root, tail in re.findall(
            r"\buse\s+((?:(?:crate|self|super)::)?ast|[A-Za-z_][A-Za-z0-9_]*)::(.*?);",
            code,
            flags=re.DOTALL,
        )
    ]
    module_targets = {
        alias: root
        for root, alias in re.findall(
            r"\buse\s+((?:(?:crate|self|super)::)?ast|[A-Za-z_][A-Za-z0-9_]*)\s+as\s+([A-Za-z_][A-Za-z0-9_]*)\s*;",
            code,
        )
    }
    for root, items in use_groups:
        if root not in ast_roots:
            continue
        for item in items:
            self_alias = re.fullmatch(r"self\s+as\s+([A-Za-z_][A-Za-z0-9_]*)", item)
            if self_alias:
                module_targets[self_alias.group(1)] = root

    modules: set[str] = set()
    while True:
        changed = False
        for alias, root in module_targets.items():
            if alias not in modules and (root in ast_roots or root in modules):
                modules.add(alias)
                changed = True
        if not changed:
            break

    for root, items in use_groups:
        if root not in ast_roots and root not in modules:
            continue
        for item in items:
            if item == "*":
                glob_imported = True
            imported = re.fullmatch(
                r"(Expr|Stmt|Type)(?:\s+as\s+([A-Za-z_][A-Za-z0-9_]*))?", item
            )
            if imported:
                canonical, alias = imported.groups()
                names[alias or canonical] = canonical
    alias_targets: dict[str, str] = {}
    for match in re.finditer(
        r"\btype\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)\s*;",
        code,
    ):
        alias, target = match.groups()
        alias_targets[alias] = target

    # 直接、模块别名、裸名与链式别名按固定点展开；非 AST 的同名类型不会被收录。
    while True:
        changed = False
        for alias, target in alias_targets.items():
            canonical = None
            qualified = re.fullmatch(r"(?:(?:crate|self|super)::)?ast::(Expr|Stmt|Type)", target)
            aliased = re.fullmatch(r"([A-Za-z_][A-Za-z0-9_]*)::(Expr|Stmt|Type)", target)
            if qualified:
                canonical = qualified.group(1)
            elif aliased and aliased.group(1) in modules:
                canonical = aliased.group(2)
            elif target in CANONICAL and (target in names or glob_imported):
                canonical = target
            elif target in names:
                canonical = names[target]
            if canonical is not None and names.get(alias) != canonical:
                names[alias] = canonical
                changed = True
        if not changed:
            break
    return names, glob_imported, modules


def find_constructions(path: Path, source: str) -> list[tuple[int, str]]:
    code = sanitize(source)
    tokens = tokenize(code)
    pairs = delimiter_pairs(tokens)
    patterns = pattern_mask(tokens, pairs)
    tests = test_item_ranges(code, tokens)
    names, glob_imported, module_aliases = ast_names(code)
    findings: list[tuple[int, str]] = []

    for index, token in enumerate(tokens):
        qualified = False
        canonical = names.get(token.text)
        if token.text in CANONICAL:
            canonical = token.text
            qualified = (
                index >= 2
                and tokens[index - 1].text == "::"
                and (
                    tokens[index - 2].text == "ast"
                    or tokens[index - 2].text in module_aliases
                )
            )
        if canonical is None:
            continue
        if (
            not qualified
            and token.text not in names
            and not (glob_imported and token.text in CANONICAL)
        ):
            continue
        if any(start <= token.start < end for start, end in tests):
            continue
        if patterns[index]:
            continue

        if canonical == "Stmt":
            if index + 1 < len(tokens) and tokens[index + 1].text == "{":
                line = source.count("\n", 0, token.start) + 1
                findings.append((line, f"{token.text} {{ … }} 结构构造"))
            continue

        if index + 3 >= len(tokens):
            continue
        if tokens[index + 1].text != "::":
            continue
        variant = tokens[index + 2].text
        following = tokens[index + 3].text
        if (not variant[:1].isupper() and variant != "call") or following not in ("(", "{"):
            continue
        if patterns[index + 2]:
            continue
        line = source.count("\n", 0, token.start) + 1
        findings.append((line, f"{token.text}::{variant} 表达式构造"))
    return findings


def is_test_path(relative: str) -> bool:
    return "tests" in Path(relative).parts


def scan(root: Path) -> tuple[int, list[str]]:
    files: dict[str, Path] = {}
    for directory in SCAN_DIRS:
        base = root / directory
        if not base.exists():
            continue
        for path in base.rglob("*.rs"):
            relative = path.relative_to(root).as_posix()
            files[relative] = path

    violations: list[str] = []
    for relative, path in sorted(files.items()):
        if relative == SYNTH_FILE or is_test_path(relative):
            continue
        source = io.open(path, "r", encoding="utf-8").read()
        for line, description in find_constructions(path, source):
            violations.append(f"{relative}:{line}: {description}（只允许 {SYNTH_FILE}）")
    return len(files), violations


def selftest() -> int:
    negative = r'''
use crate::ast::{Expr, Stmt, Type};
fn queries(expr: &Expr, stmt: &Stmt, ty: &Type) {
    match expr {
        Expr::Var(_, _) => {}
        Expr::Call { .. } => {}
        _ => {}
    }
    if let Expr::Lambda { .. } = expr {}
    while let Type::Fn { .. } = ty {}
    let Expr::Var(_, _) = expr else { return; };
    let Stmt { .. } = stmt;
    assert!(matches!(expr, Expr::Var(_, _)));
    assert!(matches!(ty, Type::Fn { .. }));
    let query = Expr::extract_binding_positions(expr);
    let types = stmt.type_annotation.as_ref().map(crate::ast::Type::fn_params_flat);
    // ast::Expr::Var("comment".into(), span)
    let text = "ast::Expr::Var(\"string\".into(), span)";
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Expr { Expr::Var("test".into(), Span::dummy()) }
}
'''
    positive = r'''
use crate::ast::{Expr, Stmt, Type as AstType};
fn constructs() {
    let expr = ast::Expr::Var("x".into(), Span::dummy());
    let other = Expr::Lambda { params: vec![], body: body, span };
    let statement = crate::ast::Stmt { kind, span, plan_id };
    let ty = AstType::Name { name, span };
}
'''
    block_arm = r'''
use crate::ast::{Expr, Stmt};
fn blocked(expr: &Expr) {
    match expr {
        Expr::Var(_, _) => {}
        Expr::Lit(_, _) => {
            let stmt = crate::ast::Stmt { kind, span, plan_id };
            let hidden = ast::Expr::Var("hidden".into(), Span::dummy());
        }
        Expr::Call { .. } => {}
    }
}
'''
    glob_import = 'use crate::ast::*;\nfn f() { let made = Expr::Var("x".into(), span); }'
    module_alias = 'use crate::ast as a;\nfn f() { let made = a::Expr::Var("x".into(), span); }'
    type_alias = 'type E = ast::Expr;\nfn f() { let made = E::Var("x".into(), span); }'
    named_constructor = 'use crate::ast::Expr;\nfn f() { let made = Expr::call(func, args, named, span, plan); }'
    qualified_constructor = 'fn f() { let made = ast::Expr::call(func, args, named, span, plan); }'
    alias_negative = r'''
use crate::ast as a;
type E = ast::Expr;
fn alias_patterns(expr: &Expr, condition: bool) {
    if let E::Var(_, _) = expr {}
    assert!(matches!(expr, a::Expr::Var(_, _)));
    let query = a::Expr::extract_binding_positions(expr);
    match expr {
        Expr::Var(_, _) => if condition {} else {
            let nested_query = a::Expr::extract_binding_positions(expr);
        }
        Expr::Call { .. } => {}
    }
}
'''
    if_else_arm = r'''
use crate::ast::Expr;
fn control_arm(expr: &Expr, condition: bool) {
    match expr {
        Expr::Var(_, _) => if condition {} else {
            let hidden = ast::Expr::Var("hidden".into(), span);
        }
        Expr::Call { .. } => {}
    }
}
'''
    module_alias_type_expr = '''use crate::ast as a;
type E = a::Expr;
fn f(expr: &a::Expr) {
    let made = E::Var("x".into(), span);
    if let E::Var(_, _) = expr {}
    let query = a::Expr::extract_binding_positions(expr);
}
'''
    module_alias_type_stmt = '''use crate::ast as a;
type E = a::Stmt;
type T = a::Type;
fn f(stmt: &a::Stmt) {
    let statement = E { kind, span, plan_id };
    let ty = T::Name { name, span };
    if let E { .. } = stmt {}
}
'''
    reimport_expr = '''use crate::ast as a;
use a::Expr;
fn f(expr: &Expr) {
    let made = Expr::Var("x".into(), span);
    if let Expr::Var(_, _) = expr {}
    let query = Expr::extract_binding_positions(expr);
}
'''
    reimport_expr_as = '''use crate::ast as a;
use a::Expr as E;
fn f(expr: &E) {
    let made = E::Var("x".into(), span);
    if let E::Var(_, _) = expr {}
    let query = E::extract_binding_positions(expr);
}
'''
    self_module_alias = '''use crate::ast::{self as a};
fn f(expr: &a::Expr) {
    let made = a::Expr::Var("x".into(), span);
    if let a::Expr::Var(_, _) = expr {}
    let query = a::Expr::extract_binding_positions(expr);
}
'''
    alias_import_matrix = {
        "mixed_self_as_rename": ('''use crate::ast::{self as a, Expr as E};
fn f(expr: &E) {
    let direct = E::Var("direct".into(), span);
    let hidden = a::Stmt { kind, span, plan_id };
    if let a::Stmt { .. } = &hidden {}
    let query = a::Expr::extract_binding_positions(expr);
}
''', [3, 4]),
        "mixed_self_as_plain": ('''use crate::ast::{self as a, Expr};
fn f(expr: &Expr) {
    let direct = Expr::Var("direct".into(), span);
    let hidden = a::Stmt { kind, span, plan_id };
    if let a::Stmt { .. } = &hidden {}
    let query = a::Expr::extract_binding_positions(expr);
}
''', [3, 4]),
        "mixed_self_as_after_stmt": ('''use crate::ast::{Stmt, self as a};
fn f(stmt: &Stmt, expr: &a::Expr) {
    let direct = Stmt { kind, span, plan_id };
    let hidden = a::Expr::Var("hidden".into(), span);
    if let a::Expr::Var(_, _) = expr {}
    let query = a::Expr::extract_binding_positions(expr);
}
''', [3, 4]),
        "module_rename": ('''use crate::ast as a;
use a as b;
type E = b::Expr;
fn f(expr: &E) {
    let hidden = E::Var("hidden".into(), span);
    if let E::Var(_, _) = expr {}
    let query = a::Expr::extract_binding_positions(expr);
}
''', [5]),
        "direct_as": ('''use crate::ast::Expr as E;
fn f() {
    let made = E::Var("x".into(), span);
}
''', [3]),
        "trailing_comma": ('''use crate::ast::{self as a, Expr,};
fn f() {
    let made = a::Stmt { kind, span, plan_id };
}
''', [3]),
        "nested_brace_group": ('''use crate::ast::{self as a, {Expr, Stmt as S}};
fn f() {
    let direct = Expr::Var("x".into(), span);
    let statement = S { kind, span, plan_id };
    let ty = a::Type::Name { name, span };
}
''', [3, 4, 5]),
        "relative_bare": ('''use ast::Expr as E;
fn f() {
    let made = E::Var("x".into(), span);
}
''', [3]),
        "relative_self": ('''use self::ast::Expr as E;
fn f() {
    let made = E::Var("x".into(), span);
}
''', [3]),
        "relative_super": ('''use super::ast::Expr as E;
fn f() {
    let made = E::Var("x".into(), span);
}
''', [3]),
        "pub_use": ('''pub use crate::ast::Expr as E;
fn f() {
    let made = E::Var("x".into(), span);
}
''', [3]),
        "module_rename_chain": ('''use b as c;
use a as b;
use crate::ast as a;
type E = c::Expr;
fn f() {
    let made = E::Var("x".into(), span);
}
''', [6]),
    }
    bare_type_alias = 'use crate::ast::Expr;\ntype E = Expr;\nfn f() { let made = E::Var("x".into(), span); }'
    chained_type_alias = 'use crate::ast::Expr;\ntype First = Expr;\ntype E = First;\nfn f() { let made = E::Var("x".into(), span); }'
    with tempfile.TemporaryDirectory(prefix="check-synth-boundary-") as temporary:
        root = Path(temporary)
        fixtures = {
            "src/frontend/core/sema/patterns.rs": negative,
            "src/frontend/core/types/patterns.rs": negative,
            "src/middle/lower/query.rs": negative,
            "src/backends/query.rs": negative,
            "src/std/query.rs": negative,
            "src/middle/lower/synth.rs": positive,
            "src/middle/lower/tests/fixture.rs": positive,
            "src/middle/lower/illegal.rs": positive,
            "src/frontend/core/sema/illegal.rs": positive,
            "src/backends/illegal.rs": positive,
            "src/std/illegal.rs": positive,
            "src/middle/lower/block_arm.rs": block_arm,
            "src/middle/lower/glob_import.rs": glob_import,
            "src/middle/lower/module_alias.rs": module_alias,
            "src/middle/lower/type_alias.rs": type_alias,
            "src/middle/lower/named_constructor.rs": named_constructor,
            "src/middle/lower/qualified_constructor.rs": qualified_constructor,
            "src/frontend/core/sema/alias_patterns.rs": alias_negative,
            "src/middle/lower/if_else_arm.rs": if_else_arm,
            "src/middle/lower/module_alias_type_expr.rs": module_alias_type_expr,
            "src/middle/lower/module_alias_type_stmt.rs": module_alias_type_stmt,
            "src/middle/lower/reimport_expr.rs": reimport_expr,
            "src/middle/lower/reimport_expr_as.rs": reimport_expr_as,
            "src/middle/lower/self_module_alias.rs": self_module_alias,
            "src/middle/lower/bare_type_alias.rs": bare_type_alias,
            "src/middle/lower/chained_type_alias.rs": chained_type_alias,
        }
        fixtures.update(
            {
                f"src/middle/lower/{name}.rs": source
                for name, (source, _) in alias_import_matrix.items()
            }
        )
        for relative, source in fixtures.items():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(source, encoding="utf-8")
        _, violations = scan(root)

    expected_files = {
        "src/middle/lower/illegal.rs",
        "src/frontend/core/sema/illegal.rs",
        "src/backends/illegal.rs",
        "src/std/illegal.rs",
        "src/middle/lower/block_arm.rs",
        "src/middle/lower/glob_import.rs",
        "src/middle/lower/module_alias.rs",
        "src/middle/lower/type_alias.rs",
        "src/middle/lower/named_constructor.rs",
        "src/middle/lower/qualified_constructor.rs",
        "src/middle/lower/if_else_arm.rs",
        "src/middle/lower/module_alias_type_expr.rs",
        "src/middle/lower/module_alias_type_stmt.rs",
        "src/middle/lower/reimport_expr.rs",
        "src/middle/lower/reimport_expr_as.rs",
        "src/middle/lower/self_module_alias.rs",
        "src/middle/lower/bare_type_alias.rs",
        "src/middle/lower/chained_type_alias.rs",
    }
    expected_files.update(
        f"src/middle/lower/{name}.rs" for name in alias_import_matrix
    )
    forbidden = [
        item for item in violations if item.split(":", 1)[0] in expected_files
    ]
    unexpected = [
        item for item in violations if item.split(":", 1)[0] not in expected_files
    ]
    observed_files = {item.split(":", 1)[0] for item in forbidden}
    failures: list[str] = []
    if observed_files != expected_files:
        failures.append(f"真构造未全部判红：observed={sorted(observed_files)}")
    if unexpected:
        failures.append(f"非构造/允许面误报：{unexpected}")
    expected_alias_lines = {
        "src/middle/lower/module_alias_type_expr.rs": [4],
        "src/middle/lower/module_alias_type_stmt.rs": [5, 6],
        "src/middle/lower/reimport_expr.rs": [4],
        "src/middle/lower/reimport_expr_as.rs": [4],
        "src/middle/lower/self_module_alias.rs": [3],
    }
    expected_alias_lines.update(
        {
            f"src/middle/lower/{name}.rs": lines
            for name, (_, lines) in alias_import_matrix.items()
        }
    )
    for relative, expected_lines in expected_alias_lines.items():
        observed_lines = [
            int(item.split(":", 2)[1])
            for item in violations
            if item.split(":", 1)[0] == relative
        ]
        if observed_lines != expected_lines:
            failures.append(
                f"合法别名构造判据错误：{relative} "
                f"expected={expected_lines}, observed={observed_lines}"
            )
    if failures:
        for failure in failures:
            print(f"SELFTEST FAIL: {failure}", file=sys.stderr)
        return 1
    print("check-synth-boundary selftest: PASS")
    print(f"  真构造判红：{len(forbidden)} 处，覆盖 middle/sema/backends/std")
    print("  非构造零误报：match/if-let/while-let/let-else/matches!/查询/注释/字符串/cfg(test)/tests 路径")
    print(f"  唯一允许文件：{SYNTH_FILE}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selftest", action="store_true", help="运行正反向门禁自检")
    args = parser.parse_args()
    if args.selftest:
        return selftest()

    count, violations = scan(ROOT)
    print(f"check-synth-boundary: 扫描 {count} 个 L3/L4 Rust 文件")
    print(f"唯一 AST 合成例外：{SYNTH_FILE}")
    if violations:
        print("AST 合成越界：", file=sys.stderr)
        for violation in violations:
            print(f"  {violation}", file=sys.stderr)
        return 1
    print("check-synth-boundary: PASS（生产 AST 合成全部位于 synth.rs）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
