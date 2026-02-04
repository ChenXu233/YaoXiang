//! DCE 测试

use std::collections::HashMap;

use crate::middle::passes::mono::dce::{DceConfig, DcePass, DceResult};
use crate::middle::passes::mono::instance::FunctionId;
use crate::frontend::typecheck::MonoType;
use crate::middle::core::ir::FunctionIR;
use crate::middle::core::ir::ModuleIR;

#[test]
fn test_dce_pass() {
    let config = DceConfig::default();
    let mut dce = DcePass::new(config);

    let mut instantiated_functions = HashMap::new();
    instantiated_functions.insert(
        FunctionId::new("main".to_string(), vec![]),
        FunctionIR {
            name: "main".to_string(),
            params: vec![],
            return_type: MonoType::Void,
            is_async: false,
            locals: vec![],
            blocks: vec![],
            entry: 0,
        },
    );

    let result = dce.run_on_module(
        &ModuleIR::default(),
        &instantiated_functions,
        &HashMap::new(),
        &[FunctionId::new("main".to_string(), vec![])],
    );

    assert_eq!(result.kept_functions.len(), 1);
}

#[test]
fn test_dce_result() {
    let result = DceResult::new();
    assert!(!result.has_eliminated_functions());
}

#[test]
fn test_dce_config() {
    let dev_config = DceConfig::development();
    assert!(!dev_config.enable_bloat_control);
    assert!(dev_config.print_stats);

    let release_config = DceConfig::release();
    assert!(release_config.enable_bloat_control);
    assert!(!release_config.print_stats);
}
