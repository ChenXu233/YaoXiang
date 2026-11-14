//! Parser state and error handling

use crate::frontend::core::lexer::tokens::*;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::plan_id::PlanId;
use crate::util::span::Span;
use std::collections::HashSet;

/// Temporary: wrap string as E0012 diagnostic（#324：挂当前 token 位置）
pub fn parse_msg(
    state: &ParserState<'_>,
    msg: impl Into<String>,
) -> Diagnostic {
    ErrorCodeDefinition::invalid_syntax(&msg.into())
        .at(state.span())
        .build()
}

/// 解析递归深度上限（issue #540）。
///
/// 依据（登记）：实测（debug 构建 / 1MB 主线程栈）既有**编译通过**形态的
/// 解析递归深度上限由括号嵌套决定——600 层括号通过（解析递归 ~602 层，
/// AST 折叠后无深度、可编译），崩溃起点 ~650；其余形态最深通过为一元链
/// ~255（含语句/块包装计 ~259 层）。下限取 ~640 保全部既有通过区，
/// 上限为大栈线程（64MB）崩溃阈值 ~8500 层的 ~1/8——1024 位于窗口
/// [640, 8499] 内，是裁决登记值：解析递归 ≥1025 层的形态（修前在 1MB
/// 栈上全部崩溃或已被 E3014 拒绝）在此得到 E0019，≤1024 层的既有通过
/// 程序逐字不变。深度按**语句 + 表达式两汇点合计**计数（解析真实递归
/// 栈），扁平二元链不经本闸（parser 迭代处理），由 IR 层既有 E3014
/// 寄存器上限拒绝。
pub(crate) const MAX_PARSE_DEPTH: usize = 1024;

pub struct ParserState<'a> {
    tokens: &'a [Token],
    pos: usize,
    errors: Vec<Diagnostic>,
    /// 注入的**类型环境探针**（RFC-039 6.4 / 03 §5.4）：约束名、编译期谓词名、
    /// 内置名定种的唯一判定口。parser 属 L2，不得反向依赖 L3 的类型环境，
    /// 故以 trait 注入；无环境入口传 [`NullProbe`](super::probe::NullProbe)，
    /// 生产入口传编译器内置封闭集探针或 L3 语义探针。
    probe: &'a dyn super::probe::TypeEnvProbe,
    /// match 臂守卫模式（RFC-010b）：为真时 `=>` 不作为 lambda 中缀被
    /// 表达式循环消费——守卫表达式 `n if n > 3 => ...` 要完整解析 `n > 3`
    /// 并停在 `=>`。save/restore 语义，嵌套（守卫里再写 match）安全。
    pub(crate) no_fat_arrow: bool,
    /// RFC-027：本模块已解析的**谓词名**（返回 `Type` 的声明）。
    ///
    /// 用于判定类型标注里的类型应用是「编译期谓词应用」（实参位是**值**）
    /// 还是「const 泛型应用」（实参位是类型引用）。parser 不持有类型环境，
    /// 只能靠本遍已解析的声明累积——**谓词须先声明后使用**。
    predicate_names: HashSet<String>,
    /// 释放计划身份的单调计数器（RFC-039 D20 / WBS 4.5.1）：本编译单元的
    /// 语句身份由此按解析序分配，见 [`Self::alloc_plan_id`]。
    next_plan_id: u32,
    /// 当前解析递归深度（issue #540）：每进入一层嵌套语句/表达式加一。
    /// 唯一的进入/退出点是 [`Self::parse_statement`] 与
    /// [`Self::parse_expression_internal`]——parser 的全部递归都经这两个
    /// 汇点之一（块体递归走 `parse_statement`，pratt 前缀臂/调用实参/
    /// 括号/列表走表达式汇点）。
    parse_depth: usize,
    /// 深度闸拒绝标志：E0019 已在本语句内发射。恢复路径（「Expected
    /// expression after '='」一族）见此标志即跳过追加恢复诊断——否则
    /// 同因双诊断违反「恰 1 条正式诊断」判据。每条语句开始时清零。
    depth_rejected: bool,
    /// 当前语句的起点 token 下标（[`Self::parse_statement`] 进入时记录）：
    /// [`Self::drain_broken_construct`] 据此回扫本语句的括号余额。
    stmt_start_pos: usize,
}

impl<'a> ParserState<'a> {
    pub fn new(
        tokens: &'a [Token],
        probe: &'a dyn super::probe::TypeEnvProbe,
    ) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
            probe,
            no_fat_arrow: false,
            predicate_names: HashSet::new(),
            next_plan_id: 0,
            parse_depth: 0,
            depth_rejected: false,
            stmt_start_pos: 0,
        }
    }

    /// 进入一层嵌套解析（issue #540 深度闸）。
    ///
    /// 深度已达 [`MAX_PARSE_DEPTH`] 时发射 E0019、置拒绝标志并拒绝进入——
    /// 调用方按 `None` 语义向上弹出，整条嵌套链只在拒绝点产生一条诊断
    /// （同语句内的恢复诊断见 [`Self::depth_rejected`] 的消费点）。深度
    /// 计数在拒绝路径不增不减，外层各汇点的 [`Self::exit_nested_parse`]
    /// 照常成对回收。
    pub(crate) fn enter_nested_parse(&mut self) -> bool {
        if self.parse_depth >= MAX_PARSE_DEPTH {
            // 一次拒绝只发一条 E0019：同一语句上下文内的后续拒绝（闸值
            // 包裹仍在执行、计数器停留在闸值，其内所有语句都超限）静默
            // 拒绝——否则残骸逐语句再拒会产生 E0019 级联。标志在每条
            // 语句开始时复位。
            if !self.depth_rejected {
                self.error(
                    ErrorCodeDefinition::expression_too_deep(MAX_PARSE_DEPTH)
                        .at(self.span())
                        .build(),
                );
                self.depth_rejected = true;
            }
            return false;
        }
        self.parse_depth += 1;
        true
    }

    /// 退出一层嵌套解析（与 [`Self::enter_nested_parse`] 成对）。
    pub(crate) fn exit_nested_parse(&mut self) {
        self.parse_depth -= 1;
    }

    /// 深度闸拒绝标志读取（恢复路径消费点：expressions/declarations.rs 的
    /// 「Expected expression after …」一族——标志为真时跳过追加恢复诊断）。
    pub(crate) fn is_depth_rejected(&self) -> bool {
        self.depth_rejected
    }

    /// f-string 内嵌表达式另起解析器实例（`ParserState::new`，nud.rs），
    /// 深度从外层继承——否则「1024 层外层 + 1024 层内嵌」两计数都看自己，
    /// 真实栈深翻倍而两闸都拦不住（issue #540 测绘发现的边界）。
    pub(crate) fn inherit_parse_depth_from(
        &mut self,
        outer: &Self,
    ) {
        self.parse_depth = outer.parse_depth;
    }

    /// 类型环境探针：约束名 / 谓词名 / 内置名定种一律经此判定。
    pub fn probe(&self) -> &'a dyn super::probe::TypeEnvProbe {
        self.probe
    }

    /// 名字定种：本遍已声明的谓词优先，其余交注入探针。
    ///
    /// parser 只能看到**本遍已解析**的声明（[`Self::declare_predicate`]），
    /// 环境侧的谓词名由探针提供——两者都不是「类型层」本身的定义。
    pub fn kind_of(
        &self,
        name: &str,
    ) -> super::probe::NameKind {
        if self.predicate_names.contains(name) {
            super::probe::NameKind::Predicate
        } else {
            self.probe.kind_of(name)
        }
    }

    /// 分配下一个释放计划身份（RFC-039 D20 / WBS 4.5.1）。
    ///
    /// 解析期单调递增、编译单元内唯一。**唯一分配点**：语句身份必须随 AST
    /// 节点走（`Stmt::plan_id`），故由 [`Self::parse_statement`] 统一发放——
    /// 各 `parse_*_stmt` 只构造语法节点，不各自编号（避免同一个身份两处来源）。
    /// 解析器之外的合成语句用 [`PlanId::UNPLANNED`]，不构成释放计划义务。
    pub fn alloc_plan_id(&mut self) -> PlanId {
        let id = PlanId::from_ordinal(self.next_plan_id);
        self.next_plan_id += 1;
        id
    }

    /// RFC-027：登记一个谓词名（返回 `Type` 的声明）。
    pub(crate) fn declare_predicate(
        &mut self,
        name: &str,
    ) {
        self.predicate_names.insert(name.to_string());
    }

    pub fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
            || matches!(self.current().map(|t| &t.kind), Some(TokenKind::Eof))
    }
    pub fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }
    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1)
    }
    pub fn peek_nth(
        &self,
        n: usize,
    ) -> Option<&Token> {
        self.tokens.get(self.pos + n)
    }
    pub fn span(&self) -> Span {
        self.current().map(|t| t.span).unwrap_or(Span::dummy())
    }
    pub fn bump(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned()?;
        self.pos += 1;
        Some(t)
    }
    pub fn at(
        &self,
        kind: &TokenKind,
    ) -> bool {
        self.current().is_some_and(|t| &t.kind == kind)
    }
    pub fn skip(
        &mut self,
        kind: &TokenKind,
    ) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }
    pub fn expect(
        &mut self,
        kind: &TokenKind,
    ) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            // 深度闸拒绝后的弹出期（issue #540）：残骸已由 drain 清理，
            // 各层 expect(RBrace/RParen) 的「缺失闭合」恢复诊断是同因噪声
            // ——静默失败，保「恰 1 条正式诊断」判据。标志在每条语句
            // 开始时复位，不影响后续语句的正常恢复。
            if !self.depth_rejected {
                let found = self
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof);
                self.errors.push(
                    ErrorCodeDefinition::expected_token(
                        &format!("{:?}", kind),
                        &format!("{:?}", found),
                    )
                    .at(self.span())
                    .build(),
                );
            }
            false
        }
    }
    pub fn error(
        &mut self,
        mut error: Diagnostic,
    ) {
        // #324：parser 错误缺 span 时自动挂当前 token 位置（咽喉点，覆盖全部站点）
        if error.span.is_none() {
            error.span = Some(self.span());
        }
        self.errors.push(error);
    }
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    pub fn first_error(&self) -> Option<&Diagnostic> {
        self.errors.first()
    }
    pub fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }
    pub fn take_errors(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.errors)
    }
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }
    pub fn save_position(&self) -> usize {
        self.pos
    }
    pub fn restore_position(
        &mut self,
        pos: usize,
    ) {
        self.pos = pos;
    }
    pub fn truncate_errors(
        &mut self,
        len: usize,
    ) {
        self.errors.truncate(len);
    }
    pub fn can_start_stmt(&self) -> bool {
        !self.at_end() && !self.at(&TokenKind::Semicolon)
    }
    pub fn synchronize(&mut self) {
        while !self.at_end() {
            if self.at(&TokenKind::Semicolon) {
                self.bump();
                break;
            }
            if let Some(TokenKind::Identifier(_)) = self.current().map(|t| &t.kind) {
                if matches!(
                    self.peek().map(|t| &t.kind),
                    Some(TokenKind::Colon | TokenKind::Eq | TokenKind::LParen | TokenKind::LBrace)
                ) {
                    break;
                }
            }
            self.bump();
        }
    }
    /// 解析一条语句。
    ///
    /// 释放计划身份（`Stmt::plan_id`）在此**统一发放**——语句构造在
    /// `parse_*_stmt` 里完成、语法入不了编号，故身份的单一来源是这里：
    /// 「每条经解析器产出的语句都有一个编译单元内唯一的身份」（RFC-039 D20）。
    pub fn parse_statement(&mut self) -> Option<crate::ast::Stmt> {
        // 解析递归深度闸的语句汇点（issue #540）：块体（control_flow.rs 与
        // functions.rs 两个 parse_block_body）与 pratt 前缀臂的块递归都经
        // 本函数回入；超限发射 E0019 并以 `None` 弹出。同语句的恢复诊断
        // 上下文在此复位。
        self.depth_rejected = false;
        self.stmt_start_pos = self.pos;
        let result = if self.enter_nested_parse() {
            let nested = self.parse_statement_nested();
            self.exit_nested_parse();
            nested
        } else {
            // 入口即拒绝（外层包裹仍在执行、计数器停留在闸值）：拒绝点
            // 的残骸 token 必须就地清理——否则外层循环的 synchronize()
            // 「标识符 + `(`」启发式零推进（残骸起点恰是它的语句起点
            // 形态），实测 block-1024 在闸值上无限打转（58877 轮拒绝）。
            None
        };
        if result.is_none() && self.depth_rejected {
            // 深度闸拒绝：拒绝点位于嵌套构造中部，`None` 弹出留下未消费
            // 的残骸 token。残骸若交给外层循环的 synchronize()，其
            // 「标识符 + `(`」语句起点启发式会把残骸重新当语句解析——
            // 实测 1100 层调用嵌套产生 511 条 E0011 级联（违反恰 1 条
            // 判据）。此处静默跳到构造闭合/语句边界，四个月体恢复循环
            // （两个 parse_block_body、parse_block、parse_lambda_block）
            // 无需各自处理。
            self.drain_broken_construct();
        }
        result
    }

    /// 深度闸拒绝后的残骸清理（issue #540）。
    ///
    /// 先回扫本语句已消费的 token 计出括号余额（负值 = 拒绝点仍悬在
    /// `|balance|` 个未闭合构造内部——它们的闭合 `)` 属于本语句的残骸，
    /// 必须一并吃掉）；再前跳静默消费，直到：余额归零后遇到语句分界
    /// （`;`）、外层构造的闭合括号（不得吞，那是调用方的）、或 RFC-038
    /// 行语义的语句边界（换行）。构造内部的换行不停（跨行深构造一并
    /// 清理）。
    fn drain_broken_construct(&mut self) {
        let mut balance: i64 = 0;
        for tok in &self.tokens[self.stmt_start_pos..self.pos] {
            match tok.kind {
                TokenKind::LParen | TokenKind::LBrace | TokenKind::LBracket => balance += 1,
                TokenKind::RParen | TokenKind::RBrace | TokenKind::RBracket => balance -= 1,
                _ => {}
            }
        }
        let mut prev_line = self.current().map(|t| t.span.start.line);
        while let Some(tok) = self.current() {
            let line = tok.span.start.line;
            if balance <= 0 {
                match tok.kind {
                    TokenKind::Semicolon => {
                        self.bump();
                        return;
                    }
                    TokenKind::RParen | TokenKind::RBrace | TokenKind::RBracket => {
                        if balance < 0 {
                            // 未闭合构造的闭合——属于本语句残骸，吃掉
                            balance += 1;
                        } else {
                            // 余额归零后的闭合——那是外层构造的，不吞，
                            // 留给调用方的循环收尾
                            return;
                        }
                    }
                    // 余额归零后的首个换行 = 下一条语句的起点（RFC-038）
                    _ if prev_line.is_some_and(|pl| line > pl) => return,
                    _ => {}
                }
            }
            match tok.kind {
                TokenKind::LParen | TokenKind::LBrace | TokenKind::LBracket => balance += 1,
                TokenKind::RParen | TokenKind::RBrace | TokenKind::RBracket => balance -= 1,
                _ => {}
            }
            prev_line = Some(line);
            self.bump();
        }
    }

    /// [`Self::parse_statement`] 的闸内本体（原实现逐字保留）。
    fn parse_statement_nested(&mut self) -> Option<crate::ast::Stmt> {
        use crate::frontend::core::parser::statements::*;
        let ss = self.span();
        let stmt = match self.current().map(|t| &t.kind) {
            Some(TokenKind::KwUse) => parse_use_stmt(self, ss),
            Some(TokenKind::KwReturn) => parse_return_stmt(self, ss),
            Some(TokenKind::KwBreak) => parse_break_stmt(self, ss),
            Some(TokenKind::KwContinue) => parse_continue_stmt(self, ss),
            Some(TokenKind::KwFor) => parse_for_stmt(self, ss),
            Some(TokenKind::KwWhile) => parse_while_stmt(self, ss),
            Some(TokenKind::KwIf) => parse_if_stmt(self, ss),
            Some(TokenKind::LBrace) => parse_block_stmt(self, ss),
            // `unsafe { ... }` 是表达式（nud.rs:parse_unsafe）。语句位置必须
            // 走表达式语句，否则落入下方「关键字误用」分支报 E0018（#347）。
            Some(TokenKind::KwUnsafe) => parse_expr_stmt(self, ss),
            Some(TokenKind::KwMut) => parse_var_stmt(self, ss),
            Some(TokenKind::Identifier(_)) => parse_identifier_stmt(self, ss),
            Some(TokenKind::LParen) => parse_paren_destructure_stmt(self, ss),
            Some(TokenKind::Eof) | None => None,
            Some(TokenKind::At) => {
                self.error(ErrorCodeDefinition::unexpected_token("@").at(ss).build());
                None
            }
            Some(kw @ TokenKind::KwRef)
            | Some(kw @ TokenKind::KwElse)
            | Some(kw @ TokenKind::KwIn)
            | Some(kw @ TokenKind::KwAs) => {
                let kw = match kw {
                    TokenKind::KwRef => "ref",
                    TokenKind::KwElse => "else",
                    TokenKind::KwIn => "in",
                    TokenKind::KwAs => "as",
                    _ => "keyword",
                };
                self.error(ErrorCodeDefinition::keyword_as_name(kw).at(ss).build());
                self.bump();
                None
            }
            Some(_) => parse_expr_stmt(self, ss),
        };
        stmt.map(|mut stmt| {
            stmt.plan_id = self.alloc_plan_id();
            stmt
        })
    }
    pub fn parse_expression(
        &mut self,
        min_bp: u8,
    ) -> Option<crate::ast::Expr> {
        crate::frontend::core::parser::pratt::parse_expression_impl(self, min_bp)
    }
}
