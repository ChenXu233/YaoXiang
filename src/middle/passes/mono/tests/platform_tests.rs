//! 平台特化功能测试
//!
//! 测试 RFC-011 平台特化功能
//!
//! 使用示例：
//! ```yaoxiang
//! # 通用实现（所有平台可用）
//! sum: [T: Add](arr: Array[T]) -> T = { ... }
//!
//! # 平台特化：P 是预定义泛型参数，代表当前平台
//! sum: [P: X86_64](arr: Array[Float]) -> Float = {
//!     return avx2_sum(arr.data, arr.length)
//! }
//!
//! sum: [P: AArch64](arr: Array[Float]) -> Float = {
//!     return neon_sum(arr.data, arr.length)
//! }
//! ```

#[cfg(test)]
mod platform_info_tests {
    use crate::middle::passes::mono::platform_info::{PlatformDetector, TargetPlatform};

    #[test]
    fn test_detect_x86_64() {
        let info = PlatformDetector::detect_from_target("x86_64-unknown-linux-gnu");
        assert_eq!(info.target(), &TargetPlatform::X86_64);
        // 使用 TargetPlatform 的方法
        assert!(info.target().is_64bit());
        assert!(info.target().is_x86());
    }

    #[test]
    fn test_detect_aarch64() {
        let info = PlatformDetector::detect_from_target("aarch64-apple-darwin");
        assert_eq!(info.target(), &TargetPlatform::AArch64);
        assert!(info.target().is_64bit());
        assert!(info.target().is_arm());
    }

    #[test]
    fn test_detect_wasm32() {
        let info = PlatformDetector::detect_from_target("wasm32-unknown-unknown");
        assert_eq!(info.target(), &TargetPlatform::Wasm32);
        assert!(info.target().is_32bit());
    }

    #[test]
    fn test_platform_display() {
        assert_eq!(TargetPlatform::X86_64.to_string(), "X86_64");
        assert_eq!(TargetPlatform::AArch64.to_string(), "AArch64");
        assert_eq!(TargetPlatform::RiscV64.to_string(), "RISC_V64");
    }

    #[test]
    fn test_is_platform_param() {
        use crate::middle::passes::mono::platform_info::is_platform_param;
        assert!(is_platform_param("P"));
        assert!(!is_platform_param("T"));
        assert!(!is_platform_param("X86_64"));
    }

    #[test]
    fn test_platform_type_name() {
        let info = PlatformDetector::detect_from_target("x86_64-pc-windows-msvc");
        assert_eq!(info.platform_type_name(), "X86_64");
    }

    #[test]
    fn test_cpu_features() {
        let info = PlatformDetector::detect_from_target("x86_64-linux-gnu");
        assert!(!info.has_cpu_feature("avx2"));
        let mut info = info.clone();
        info.add_cpu_feature("avx2".to_string());
        assert!(info.has_cpu_feature("avx2"));
    }
}

#[cfg(test)]
mod platform_specializer_tests {
    use crate::middle::passes::mono::platform_info::{PlatformDetector, TargetPlatform};
    use crate::middle::passes::mono::platform_specializer::{
        FunctionPlatformInfo, PlatformConstraint, PlatformSpecializer, SpecializationDecider,
    };

    #[test]
    fn test_platform_constraint_matches() {
        let constraint = PlatformConstraint::new("X86_64".to_string());
        assert!(constraint.matches(&TargetPlatform::X86_64));
        assert!(!constraint.matches(&TargetPlatform::AArch64));
    }

    #[test]
    fn test_wildcard_constraint() {
        let constraint = PlatformConstraint::any();
        assert!(constraint.is_any());
        assert!(constraint.matches(&TargetPlatform::X86_64));
        assert!(constraint.matches(&TargetPlatform::AArch64));
    }

    #[test]
    fn test_select_specialization_x86_64() {
        let platform_info = PlatformDetector::detect_from_target("x86_64-unknown-linux-gnu");
        let mut specializer = PlatformSpecializer::new(platform_info);

        // 注册两个特化版本
        specializer.register_function(FunctionPlatformInfo::new(
            "sum".to_string(),
            vec![],
            Some(PlatformConstraint::new("X86_64".to_string())),
        ));

        specializer.register_function(FunctionPlatformInfo::new(
            "sum".to_string(),
            vec![],
            Some(PlatformConstraint::new("AArch64".to_string())),
        ));

        // 在 X86_64 平台上，应该选择 X86_64 版本
        let selected = specializer.select_specialization("sum", &[]);
        assert!(selected.is_some());
        assert_eq!(
            selected
                .unwrap()
                .platform_constraint()
                .unwrap()
                .platform_type(),
            "X86_64"
        );
    }

    #[test]
    fn test_select_specialization_aarch64() {
        let platform_info = PlatformDetector::detect_from_target("aarch64-apple-darwin");
        let mut specializer = PlatformSpecializer::new(platform_info);

        // 注册两个特化版本
        specializer.register_function(FunctionPlatformInfo::new(
            "sum".to_string(),
            vec![],
            Some(PlatformConstraint::new("X86_64".to_string())),
        ));

        specializer.register_function(FunctionPlatformInfo::new(
            "sum".to_string(),
            vec![],
            Some(PlatformConstraint::new("AArch64".to_string())),
        ));

        // 在 AArch64 平台上，应该选择 AArch64 版本
        let selected = specializer.select_specialization("sum", &[]);
        assert!(selected.is_some());
        assert_eq!(
            selected
                .unwrap()
                .platform_constraint()
                .unwrap()
                .platform_type(),
            "AArch64"
        );
    }

    #[test]
    fn test_specialization_decider() {
        let platform_info = PlatformDetector::detect_from_target("x86_64-unknown-linux-gnu");
        let decider = SpecializationDecider::new(&platform_info);

        let x86_constraint = PlatformConstraint::new("X86_64".to_string());
        let arm_constraint = PlatformConstraint::new("AArch64".to_string());

        let x86_decision = decider.decide(&x86_constraint);
        assert!(x86_decision.should_specialize());

        let arm_decision = decider.decide(&arm_constraint);
        assert!(!arm_decision.should_specialize());
    }

    #[test]
    fn test_function_without_platform_constraint() {
        let platform_info = PlatformDetector::detect_from_target("x86_64-unknown-linux-gnu");
        let mut specializer = PlatformSpecializer::new(platform_info);

        // 注册无平台约束的通用版本
        specializer.register_function(FunctionPlatformInfo::new(
            "generic_sum".to_string(),
            vec![],
            None,
        ));

        // 通用版本应该始终匹配
        let selected = specializer.select_specialization("generic_sum", &[]);
        assert!(selected.is_some());
        assert!(selected.unwrap().platform_constraint().is_none());
    }
}

#[cfg(test)]
mod parser_platform_param_tests {
    use crate::frontend::core::parser::ast::{GenericParam, GenericParamKind};
    use crate::middle::passes::mono::platform_specializer::PlatformSpecializer;

    #[test]
    fn test_extract_platform_constraint_with_name() {
        let params = vec![GenericParam {
            name: "P".to_string(),
            kind: GenericParamKind::Platform,
            constraints: vec![crate::frontend::core::parser::ast::Type::Name(
                "X86_64".to_string(),
            )],
        }];

        let (constraint, filtered) = PlatformSpecializer::extract_platform_constraint(&params);

        assert!(constraint.is_some());
        assert_eq!(constraint.unwrap().platform_type(), "X86_64");
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_extract_platform_constraint_mixed_params() {
        use crate::frontend::core::parser::ast::Type;

        let params = vec![
            GenericParam {
                name: "T".to_string(),
                kind: GenericParamKind::Type,
                constraints: vec![Type::Name("Clone".to_string())],
            },
            GenericParam {
                name: "P".to_string(),
                kind: GenericParamKind::Platform,
                constraints: vec![Type::Name("AArch64".to_string())],
            },
        ];

        let (constraint, filtered) = PlatformSpecializer::extract_platform_constraint(&params);

        assert!(constraint.is_some());
        assert_eq!(constraint.unwrap().platform_type(), "AArch64");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "T");
    }

    #[test]
    fn test_no_platform_constraint() {
        let params = vec![GenericParam {
            name: "T".to_string(),
            kind: GenericParamKind::Type,
            constraints: vec![],
        }];

        let (constraint, filtered) = PlatformSpecializer::extract_platform_constraint(&params);

        assert!(constraint.is_none());
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "T");
    }
}
