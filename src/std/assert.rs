//! assert 标准库模块
//!
//! 同时导出类型族（类型宇宙）和 native 函数（值宇宙）：
//! - IsTrue: (b: Bool) -> Type = match b { true => Void, false => Never }
//! - Assert: (cond: Bool) -> Type = IsTrue(cond)
//! - assert: (cond: Bool, ?msg: String) -> Void — 运行时断言，false 时 panic

use crate::backends::common::value::RuntimeValue;
use crate::ExecutorError;
use crate::frontend::core::types::eval::dependent_types::AssociatedTypeDef;
use crate::frontend::core::types::mono::MonoType;
use crate::std::{NativeContext, NativeExport, StdModule, TypeFamilyExport};

pub struct AssertModule;

impl Default for AssertModule {
    fn default() -> Self {
        Self
    }
}

impl StdModule for AssertModule {
    fn module_path(&self) -> &str {
        "std.assert"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![NativeExport::new(
            "assert",
            "std.assert.assert",
            "(cond: Bool, ?msg: String) -> Void",
            native_assert,
        )]
    }

    fn type_families(&self) -> Vec<TypeFamilyExport> {
        vec![
            TypeFamilyExport::new(
                "IsTrue",
                vec!["b"],
                AssociatedTypeDef::Match {
                    arg_index: 0,
                    arms: vec![
                        (MonoType::TypeRef("true".into()), MonoType::Void),
                        (MonoType::TypeRef("false".into()), MonoType::Never),
                    ],
                },
            ),
            TypeFamilyExport::new(
                "Assert",
                vec!["cond"],
                AssociatedTypeDef::Direct(MonoType::TypeRef("IsTrue(cond)".into())),
            ),
        ]
    }

    fn effect_specs(&self) -> Vec<crate::std::EffectSpec> {
        vec![crate::std::EffectSpec::new(
            "assert",
            vec![crate::std::Effect::GammaAssume { predicate_arg: 0 }],
            true,
        )]
    }
}

/// native assert 函数：cond 为 false 时 panic
fn native_assert(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::runtime_only(
            "assert expects at least 1 argument (cond: Bool)",
        ));
    }

    let cond = match &args[0] {
        RuntimeValue::Bool(b) => *b,
        _ => {
            return Err(ExecutorError::runtime_only(
                "assert expects first argument to be Bool",
            ))
        }
    };

    if cond {
        Ok(RuntimeValue::Unit)
    } else {
        let msg = if args.len() >= 2 {
            match &args[1] {
                RuntimeValue::String(s) => format!("Assertion failed: {}", s),
                _ => "Assertion failed".to_string(),
            }
        } else {
            "Assertion failed".to_string()
        };
        Err(ExecutorError::runtime_only(msg))
    }
}
