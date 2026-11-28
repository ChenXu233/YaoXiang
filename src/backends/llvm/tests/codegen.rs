//! 发射层类型映射的回归测试（批次 4.3）。
//!
//! 性质：`ScalarType` 的五个变体里，只有 `Void` 没有 LLVM 值形态，其余四个
//! （`Int`/`Bool`/`Float`/`String`）都有值形态。分析层只放行这五种标量，非 Void 的
//! 槽位在入口块拿到 alloca、全局槽位由 `declare_globals` 物化指针——两处「缺指针」
//! 因此只可能是 Void。该性质是「不静默物化 Void」这条判据的证明前提，任何新增标量
//! 变体只要没有值形态就会让本测试（枚举式 `match`）编译失败。

use crate::backends::llvm::analysis::ScalarType;
use crate::backends::llvm::codegen::llvm_type;

#[test]
fn test_only_void_scalar_has_no_llvm_form() {
    // Arrange：五种标量逐个显式列出，新增变体时本 match 会要求同步。
    let context = inkwell::context::Context::create();
    let variants = [
        (ScalarType::Void, "Void"),
        (ScalarType::Int, "Int"),
        (ScalarType::Bool, "Bool"),
        (ScalarType::Float, "Float"),
        (ScalarType::String, "String"),
    ];

    // Act & Assert
    for (ty, name) in variants {
        let form = match ty {
            ScalarType::Void => llvm_type(&context, ty),
            ScalarType::Int | ScalarType::Bool | ScalarType::Float | ScalarType::String => {
                llvm_type(&context, ty)
            }
        };
        if ty == ScalarType::Void {
            assert!(
                form.is_none(),
                "{name} must have no LLVM value form, got {form:?}"
            );
        } else {
            assert!(
                form.is_some(),
                "{name} must have an LLVM value form so its slots can be materialized"
            );
        }
        if ty == ScalarType::Float {
            // Float 必须落成 LLVM 的浮点类型（f64），不能是整数形态。
            assert!(
                matches!(form, Some(inkwell::types::BasicTypeEnum::FloatType(_))),
                "Float must map to an LLVM float type, got {form:?}"
            );
        }
    }
}
