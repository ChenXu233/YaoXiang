//! Generics 推断模块测试
//!
//! 测试泛型函数推断功能

use crate::frontend::core::typecheck::inference::generics::GenericInferrer;
use crate::frontend::core::types::MonoType;

#[test]
fn test_infer_generic_function_creates_fresh_vars() {
    let mut inferrer = GenericInferrer::new();

    let t1 = inferrer
        .infer_generic_function("f", &["T".to_string()])
        .unwrap();
    let t2 = inferrer
        .infer_generic_function("g", &["U".to_string()])
        .unwrap();

    match (t1, t2) {
        (MonoType::TypeVar(v1), MonoType::TypeVar(v2)) => {
            assert_ne!(v1, v2);
        }
        _ => panic!("Expected type variables"),
    }
}
