//! Monomorphizer DCE 集成测试

use crate::middle::passes::mono::dce::DceConfig;
use crate::middle::passes::mono::{Monomorphizer, instance};
use crate::frontend::typecheck::MonoType;
use crate::middle::core::ir::FunctionIR;
use crate::middle::passes::mono::instance::GenericFunctionId;

#[test]
fn test_dce_integration() {
    // 创建单态化器，启用 DCE
    let mut mono = Monomorphizer::with_dce_config(DceConfig::default());

    // 插入泛型函数
    let map_ir = FunctionIR {
        name: "map".to_string(),
        params: vec![MonoType::List(Box::new(MonoType::Int(32)))],
        return_type: MonoType::List(Box::new(MonoType::Int(32))),
        is_async: false,
        locals: vec![],
        blocks: vec![],
        entry: 0,
    };
    let map_id = GenericFunctionId::new("map".to_string(), vec!["T".to_string()]);
    mono.test_insert_generic_function(map_id.clone(), map_ir);

    // 插入另一个泛型函数（不会被调用）
    let unused_ir = FunctionIR {
        name: "unused_function".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![],
        blocks: vec![],
        entry: 0,
    };
    let unused_id = GenericFunctionId::new("unused_function".to_string(), vec![]);
    mono.test_insert_generic_function(unused_id.clone(), unused_ir);

    // 模拟实例化 map[Int]（通过直接插入）
    let map_int_id = instance::FunctionId::new("map".to_string(), vec![MonoType::Int(32)]);
    let map_int_ir = FunctionIR {
        name: "map_int".to_string(),
        params: vec![MonoType::List(Box::new(MonoType::Int(32)))],
        return_type: MonoType::List(Box::new(MonoType::Int(32))),
        is_async: false,
        locals: vec![],
        blocks: vec![],
        entry: 0,
    };
    mono.instantiated_functions
        .insert(map_int_id.clone(), map_int_ir);

    // 模拟实例化 unused_function（不会被调用）
    let unused_inst_id = instance::FunctionId::new("unused_function".to_string(), vec![]);
    let unused_inst_ir = FunctionIR {
        name: "unused_function".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![],
        blocks: vec![],
        entry: 0,
    };
    mono.instantiated_functions
        .insert(unused_inst_id.clone(), unused_inst_ir);

    // 验证实例化数量
    assert_eq!(mono.instantiated_functions.len(), 2);

    // DCE 统计应该为空（还没运行 DCE）
    assert!(mono.dce_stats().is_none());

    // 验证 DCE 配置
    assert!(mono.dce_config().enabled);
}

#[test]
fn test_dce_with_disabled_config() {
    // 创建单态化器，禁用 DCE
    let mut mono = Monomorphizer::with_dce_config(DceConfig {
        enabled: false,
        ..Default::default()
    });

    // 插入一些函数
    let ir = FunctionIR {
        name: "test".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![],
        blocks: vec![],
        entry: 0,
    };
    let id = instance::FunctionId::new("test".to_string(), vec![]);
    mono.instantiated_functions.insert(id.clone(), ir);

    // 禁用 DCE
    mono.disable_dce();
    assert!(!mono.dce_config().enabled);

    // 启用 DCE
    mono.enable_dce();
    assert!(mono.dce_config().enabled);
}

#[test]
fn test_dce_config_modes() {
    // 开发模式
    let dev_config = DceConfig::development();
    assert!(dev_config.enabled);
    assert!(!dev_config.enable_bloat_control);
    assert!(dev_config.print_stats);

    // 发布模式
    let release_config = DceConfig::release();
    assert!(release_config.enabled);
    assert!(release_config.enable_bloat_control);
    assert!(!release_config.print_stats);
}
