//! 接口实现测试 — 基于 RFC-011a 接口实现与动态分发设计（#307）
//!
//! RFC-011a: docs/src/design/rfc/accepted/011a-interface-implementation.md
//!
//! 测试点：
//! - §1: 接口声明（参数化类型）与类型体实例化（Self 替换展开）
//! - §1: 完整性检查五步流程 → ImplementationProof（§5.3）
//! - §1.2: 字段/方法命名空间冲突
//! - §3: 同签名重复实现（覆盖）禁止
//! - 接口继承：Self 延迟替换递归展开
//!
//! 接口机制 = 类型内自动柯里化：`Animal(Dog)` 实例化后成员
//! `speak: (self: Self) -> String` 变为 `speak: (self: Dog) -> String`，
//! 实例占住参数位、剩余参数柯里化（RFC-004 位置绑定语义）。

use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;

/// 辅助函数：解析源代码并类型检查，同时返回 checker 以检查实现证明
pub(crate) fn check_source_with_checker(
    source: &str
) -> (
    crate::frontend::core::typecheck::types::TypeCheckResult,
    TypeChecker,
) {
    let tokens = tokenize(source).expect("tokenize failed");
    let result = parse(&tokens);
    assert!(!result.has_errors, "parse failed: {:?}", result.errors);
    let module = result.module;
    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);
    (result, checker)
}

/// 辅助函数：只关心诊断结果
fn check_source(source: &str) -> crate::frontend::core::typecheck::types::TypeCheckResult {
    check_source_with_checker(source).0
}

const ANIMAL_DOG_FULL: &str = r#"
    Animal: (Self: Type) -> Type = {
        speak: (self: Self) -> String,
        age: (self: Self) -> Int,
    }
    Dog: Type = {
        name: String,
        Animal(Dog),
    }
    Dog.speak: (self: Dog) -> String = {
        return "Woof"
    }
    Dog.age: (self: Dog) -> Int = {
        return 3
    }
"#;

// RFC-011a §1: 接口实例化与实现证明

/// 规范：接口实例化成功 → 生成 ImplementationProof
///
/// 预期行为：
/// - Self ↦ Dog 替换展开接口体，签名匹配通过
/// - checker 记录 {Dog, Animal, [speak, age]} 实现证明
#[test]
fn test_rfc011a_interface_instantiation_generates_proof() {
    let (result, checker) = check_source_with_checker(ANIMAL_DOG_FULL);
    assert!(
        result.diagnostics.is_empty(),
        "complete implementation should pass: {:?}",
        result.diagnostics
    );
    let proofs = checker.implementation_proofs();
    let proof = proofs
        .iter()
        .find(|p| p.type_name == "Dog" && p.interface_name == "Animal")
        .expect("Dog should have an Animal implementation proof");
    assert_eq!(proof.methods.len(), 2, "both members proven");
    assert!(proof.methods.contains(&"speak".to_string()));
    assert!(proof.methods.contains(&"age".to_string()));
}

/// 规范：接口方法未实现 → E1098（完整性检查失败）
#[test]
fn test_rfc011a_missing_method_reports_e1098() {
    let source = r#"
        Animal: (Self: Type) -> Type = {
            speak: (self: Self) -> String,
        }
        Dog: Type = {
            name: String,
            Animal(Dog),
        }
    "#;
    let result = check_source(source);
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1098"),
        "missing implementation should report E1098: {:?}",
        result.diagnostics
    );
}

/// 规范：接口方法签名不匹配 → E1099
#[test]
fn test_rfc011a_signature_mismatch_reports_e1099() {
    let source = r#"
        Animal: (Self: Type) -> Type = {
            speak: (self: Self) -> String,
        }
        Dog: Type = {
            name: String,
            Animal(Dog),
        }
        Dog.speak: (self: Dog) -> Int = {
            return 42
        }
    "#;
    let result = check_source(source);
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1099"),
        "signature mismatch should report E1099: {:?}",
        result.diagnostics
    );
}

// RFC-011a §1.2: 命名空间共享

/// 规范：接口方法名与类型字段同名 → E1097
#[test]
fn test_rfc011a_member_field_conflict_reports_e1097() {
    let source = r#"
        Animal: (Self: Type) -> Type = {
            name: (self: Self) -> String,
        }
        Dog: Type = {
            name: String,
            Animal(Dog),
        }
    "#;
    let result = check_source(source);
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1097"),
        "field/method namespace conflict should report E1097: {:?}",
        result.diagnostics
    );
}

// RFC-011a §3: 覆盖禁止

/// 规范：同签名方法重复声明 → E1100
#[test]
fn test_rfc011a_duplicate_impl_reports_e1100() {
    let source = r#"
        Animal: (Self: Type) -> Type = {
            speak: (self: Self) -> String,
        }
        Dog: Type = {
            name: String,
            Animal(Dog),
        }
        Dog.speak: (self: Dog) -> String = {
            return "Woof"
        }
        Dog.speak: (self: Dog) -> String = {
            return "Bark"
        }
    "#;
    let result = check_source(source);
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1100"),
        "same-signature duplicate declaration should report E1100: {:?}",
        result.diagnostics
    );
}

/// 规范：不同签名同名方法 = 重载 → 放行（§3）
#[test]
fn test_rfc011a_overload_different_signature_allowed() {
    let source = r#"
        Dog: Type = {
            name: String,
        }
        Dog.speak: (self: Dog) -> String = {
            return "Woof"
        }
        Dog.speak: (self: Dog, times: Int) -> String = {
            return "Woof"
        }
    "#;
    let result = check_source(source);
    let has_e1100 = result.diagnostics.iter().any(|d| d.code == "E1100");
    assert!(
        !has_e1100,
        "different signatures are overloads, not overrides: {:?}",
        result.diagnostics
    );
}

// 接口继承：Self 延迟替换

/// 规范：接口继承 Pet: Animal(Self) → Pet(Dog) 递归展开为 Animal(Dog)
///
/// 预期行为：
/// - Dog 须同时实现 speak（继承自 Animal）与 fetch（Pet 自有）
/// - 实现证明登记为 {Dog, Pet}
#[test]
fn test_rfc011a_inheritance_expands_recursively() {
    let source = r#"
        Animal: (Self: Type) -> Type = {
            speak: (self: Self) -> String,
        }
        Pet: (Self: Type) -> Type = {
            Animal(Self),
            fetch: (self: Self) -> String,
        }
        Dog: Type = {
            name: String,
            Pet(Dog),
        }
        Dog.speak: (self: Dog) -> String = {
            return "Woof"
        }
        Dog.fetch: (self: Dog) -> String = {
            return self.name + " fetches"
        }
    "#;
    let (result, checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "inherited contract should pass: {:?}",
        result.diagnostics
    );
    let proof = checker
        .implementation_proofs()
        .iter()
        .find(|p| p.type_name == "Dog" && p.interface_name == "Pet")
        .expect("Dog should have a Pet proof via recursive expansion");
    assert_eq!(proof.methods.len(), 2, "inherited + own members proven");
}

/// 规范：继承链成员缺失同样报完整性错误
#[test]
fn test_rfc011a_inherited_member_missing_reports_e1098() {
    let source = r#"
        Animal: (Self: Type) -> Type = {
            speak: (self: Self) -> String,
        }
        Pet: (Self: Type) -> Type = {
            Animal(Self),
            fetch: (self: Self) -> String,
        }
        Dog: Type = {
            name: String,
            Pet(Dog),
        }
        Dog.fetch: (self: Dog) -> String = {
            return "ball"
        }
    "#;
    let result = check_source(source);
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1098"),
        "missing inherited member should report E1098: {:?}",
        result.diagnostics
    );
}

/// 规范：继承链内引用未注册类型 → E1095
#[test]
fn test_rfc011a_unknown_interface_in_chain_reports_e1095() {
    let source = r#"
        Pet: (Self: Type) -> Type = {
            Ghost(Self),
            fetch: (self: Self) -> String,
        }
        Dog: Type = {
            name: String,
            Pet(Dog),
        }
        Dog.fetch: (self: Dog) -> String = {
            return "ball"
        }
    "#;
    let result = check_source(source);
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1095"),
        "unknown interface in inheritance chain should report E1095: {:?}",
        result.diagnostics
    );
}

/// 规范：不完整实现不产生证明
#[test]
fn test_rfc011a_no_proof_on_failed_check() {
    let source = r#"
        Animal: (Self: Type) -> Type = {
            speak: (self: Self) -> String,
        }
        Dog: Type = {
            name: String,
            Animal(Dog),
        }
    "#;
    let (_result, checker) = check_source_with_checker(source);
    assert!(
        checker.implementation_proofs().is_empty(),
        "failed completeness check must not generate proof"
    );
}
