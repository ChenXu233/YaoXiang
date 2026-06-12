//! IR 生成器借用表达式测试
//!
//! 验证 `Expr::Borrow` AST 节点到 IR 的正确转换。
//! 对应 RFC-009 v9 借用令牌系统：Borrow 指令是零大小类型，
//! 运行时等价于 Mov，其存在让借用检查器可以进行流敏感分析。
//!
//! - `Borrow { mutable: false }` 表示不可变借用 `&expr`
//! - `Borrow { mutable: true }` 表示可变借用 `&mut expr`

use crate::frontend::core::lexer::tokens::Literal;
use crate::frontend::core::parser::ast;
use crate::middle::core::ir::{ConstValue, Instruction, Operand};
use crate::middle::core::ir_gen::AstToIrGenerator;
use crate::util::span::Span;

/// Helper: build `&expr` AST node (immutable borrow)
fn make_borrow_imm(inner: ast::Expr) -> ast::Expr {
    ast::Expr::Borrow {
        mutable: false,
        expr: Box::new(inner),
        span: Span::dummy(),
    }
}

/// Helper: build `&mut expr` AST node (mutable borrow)
fn make_borrow_mut(inner: ast::Expr) -> ast::Expr {
    ast::Expr::Borrow {
        mutable: true,
        expr: Box::new(inner),
        span: Span::dummy(),
    }
}

/// Helper: build an int literal expression
fn make_int_lit(n: i128) -> ast::Expr {
    ast::Expr::Lit(Literal::Int(n), Span::dummy())
}

/// Helper (Rule 5.1): set up generator, create expr, and run `generate_expr_ir`.
/// Returns (instructions, constants) for downstream assertions.
fn generate_borrow_ir(
    expr: &ast::Expr,
    result_reg: usize,
) -> (Vec<Instruction>, Vec<ConstValue>) {
    let mut gen = AstToIrGenerator::new();
    let mut instructions = Vec::new();
    let mut constants = Vec::new();
    gen.generate_expr_ir(expr, result_reg, &mut instructions, &mut constants)
        .expect("generate_expr_ir should succeed for Borrow expression");
    (instructions, constants)
}

/// 验证 `&42` 生成 `Borrow { mutable: false }` 指令
///
/// 规格: RFC-009 v9 不可变借用令牌
#[test]
fn borrow_immutable_literal_produces_borrow_instruction_with_mutable_false() {
    // Arrange
    let expr = make_borrow_imm(make_int_lit(42));
    let result_reg = 5; // 使用非零 result_reg 以便区分 dst 和 src

    // Act
    let (instructions, _constants) = generate_borrow_ir(&expr, result_reg);

    // Assert: 内部表达式 (Lit) 生成一条 Load，然后 Borrow 生成一条 Borrow 指令
    assert!(
        instructions.len() >= 2,
        "expected at least 2 instructions (Load + Borrow) for immutable borrow, got {}",
        instructions.len()
    );

    // 最后一条指令必须是 Borrow
    let last = instructions.last().unwrap();
    match last {
        Instruction::Borrow { dst, src, mutable } => {
            assert_eq!(
                *dst,
                Operand::Local(result_reg),
                "Borrow dst should be result_reg={}, got {:?}",
                result_reg,
                dst
            );
            // src 是内部表达式的寄存器（next_temp_reg 从 0 开始分配）
            assert_eq!(
                *src,
                Operand::Local(0),
                "Borrow src should be inner expression register (0), got {:?}",
                src
            );
            assert!(
                !mutable,
                "immutable borrow should have mutable=false, got true"
            );
        }
        other => panic!(
            "expected Instruction::Borrow as last instruction, got {:?}",
            other
        ),
    }
}

/// 验证 `&mut 42` 生成 `Borrow { mutable: true }` 指令
///
/// 规格: RFC-009 v9 可变借用令牌
#[test]
fn borrow_mutable_literal_produces_borrow_instruction_with_mutable_true() {
    // Arrange
    let expr = make_borrow_mut(make_int_lit(42));
    let result_reg = 5;

    // Act
    let (instructions, _constants) = generate_borrow_ir(&expr, result_reg);

    // Assert
    assert!(
        instructions.len() >= 2,
        "expected at least 2 instructions (Load + Borrow) for mutable borrow, got {}",
        instructions.len()
    );

    let last = instructions.last().unwrap();
    match last {
        Instruction::Borrow { dst, src, mutable } => {
            assert_eq!(
                *dst,
                Operand::Local(result_reg),
                "Borrow dst should be result_reg={}, got {:?}",
                result_reg,
                dst
            );
            assert_eq!(
                *src,
                Operand::Local(0),
                "Borrow src should be inner expression register (0), got {:?}",
                src
            );
            assert!(
                *mutable,
                "mutable borrow should have mutable=true, got false"
            );
        }
        other => panic!(
            "expected Instruction::Borrow as last instruction, got {:?}",
            other
        ),
    }
}

/// 验证内部表达式先被求值，Borrow 的 src 使用原始变量（而非临时寄存器）
///
/// 规格: RFC-009 v9 借用令牌的内部表达式求值顺序
/// BorrowChecker 使用 src 追踪冲突，原始变量确保同源借用的冲突检测。
#[test]
fn borrow_inner_expression_is_evaluated_before_borrow_token_is_created() {
    // Arrange: 使用变量引用作为内部表达式，先注册局部变量 "x" 到 local 1
    let mut gen = AstToIrGenerator::new();
    gen.register_local("x", 1);
    let inner = ast::Expr::Var("x".to_string(), Span::dummy());
    let expr = make_borrow_imm(inner);
    let result_reg = 5;
    let mut instructions = Vec::new();
    let mut constants = Vec::new();

    // Act
    gen.generate_expr_ir(&expr, result_reg, &mut instructions, &mut constants)
        .expect("generate_expr_ir should succeed for Borrow with variable inner expression");

    // Assert: 内部表达式 Var("x") 生成 Load { dst: inner_reg, src: Local(1) }
    //         然后 Borrow { dst: result_reg, src: Local(1) }  ← src 是原始变量
    assert!(
        instructions.len() >= 2,
        "expected at least 2 instructions for borrow with variable, got {}",
        instructions.len()
    );

    // 第一条指令：加载变量 x 到 inner_reg (0)
    match &instructions[0] {
        Instruction::Load { dst, src } => {
            assert_eq!(
                *dst,
                Operand::Local(0),
                "inner expr result should go to reg 0, got {:?}",
                dst
            );
            assert_eq!(
                *src,
                Operand::Local(1),
                "should load from local 1 (x), got {:?}",
                src
            );
        }
        other => panic!(
            "expected Load as first instruction for inner variable, got {:?}",
            other
        ),
    }

    // 最后一条指令：Borrow，src 指向原始变量 (Local(1))
    match instructions.last().unwrap() {
        Instruction::Borrow { dst, src, .. } => {
            assert_eq!(
                *dst,
                Operand::Local(result_reg),
                "Borrow dst should be result_reg={}",
                result_reg
            );
            assert_eq!(
                *src,
                Operand::Local(1),
                "Borrow src should reference the original variable (x at Local(1)), got {:?}",
                src
            );
        }
        other => panic!("expected Borrow as last instruction, got {:?}", other),
    }
}
