//! Symbols 模块测试
//!
//! 测试符号验证器相关功能，包括：
//! - 绑定语法验证
//! - 泛型参数验证
//! - 类型系统验证

use crate::frontend::core::lexer::symbols::{
    BindingValidator, GenericValidator, TypeSystemValidator,
};

#[test]
fn test_binding_validator() {
    let validator = BindingValidator::new(10);

    // Valid binding
    assert!(validator.validate_binding_syntax("func[0, 1, 2]").is_ok());

    // Invalid binding - missing brackets
    assert!(validator.validate_binding_syntax("func").is_err());

    // Invalid binding - negative position
    assert!(validator.validate_binding_syntax("func[-1, 0]").is_err());

    // Invalid binding - position too large
    assert!(validator.validate_binding_syntax("func[0, 15]").is_err());
}

#[test]
fn test_generic_validator() {
    let validator = GenericValidator::new(10, 5);

    // Valid generic params
    assert!(validator
        .validate_generic_params(&["T".to_string()])
        .is_ok());
    assert!(validator
        .validate_generic_params(&["T".to_string(), "U".to_string()])
        .is_ok());

    // Valid constraints
    assert!(validator.validate_constraint("T: Clone").is_ok());
    assert!(validator.validate_constraint("T: Clone + Add").is_ok());

    // Invalid constraints
    assert!(validator.validate_constraint("Clone").is_err());
}

#[test]
fn test_type_system_validator() {
    let validator = TypeSystemValidator::new(10, 5);

    // Valid type
    assert!(validator.validate_type_complexity("Vec<T>").is_ok());

    // Invalid type - too deep (11 levels, exceeds max of 10)
    let deep_type = "Vec<Vec<Vec<Vec<Vec<Vec<Vec<Vec<Vec<Vec<Vec<T>>>>>>>>>>>>";
    assert!(validator.validate_type_complexity(deep_type).is_err());

    // Valid constraints
    assert!(validator
        .validate_constraint_count(&["Clone".to_string()])
        .is_ok());

    // Invalid constraints - too many
    let many_constraints = vec!["Clone".to_string(); 6];
    assert!(validator
        .validate_constraint_count(&many_constraints)
        .is_err());
}
