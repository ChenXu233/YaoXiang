//! Monomorphize Instance 单元测试
//!
//! 测试单态化过程中的实例化请求、缓存键和函数实例

use crate::frontend::typecheck::MonoType;
use crate::middle::monomorphize::instance::{
    FunctionId, FunctionInstance, GenericFunctionId, InstantiationRequest, SpecializationKey,
};
use crate::util::span::Span;

#[cfg(test)]
mod specialization_key_tests {
    use super::*;

    #[test]
    fn test_specialization_key_no_args() {
        let key = SpecializationKey::new("main".to_string(), vec![]);
        assert_eq!(key.to_string(), "main");
    }

    #[test]
    fn test_specialization_key_with_one_arg() {
        let key = SpecializationKey::new(
            "identity".to_string(),
            vec![MonoType::Int(64)],
        );
        assert_eq!(key.to_string(), "identity<int64>");
    }

    #[test]
    fn test_specialization_key_with_multiple_args() {
        let key = SpecializationKey::new(
            "map".to_string(),
            vec![MonoType::Int(32), MonoType::Float(64)],
        );
        assert_eq!(key.to_string(), "map<int32,float64>");
    }

    #[test]
    fn test_specialization_key_with_string_type() {
        let key = SpecializationKey::new(
            "print".to_string(),
            vec![MonoType::String],
        );
        assert_eq!(key.to_string(), "print<string>");
    }

    #[test]
    fn test_specialization_key_with_list_type() {
        let key = SpecializationKey::new(
            "len".to_string(),
            vec![MonoType::List(Box::new(MonoType::Int(32)))],
        );
        assert_eq!(key.to_string(), "len<list<int32>>");
    }

    #[test]
    fn test_specialization_key_partial_eq() {
        let key1 = SpecializationKey::new("func".to_string(), vec![MonoType::Int(64)]);
        let key2 = SpecializationKey::new("func".to_string(), vec![MonoType::Int(64)]);
        let key3 = SpecializationKey::new("func".to_string(), vec![MonoType::Float(64)]);
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_specialization_key_display() {
        let key = SpecializationKey::new("test".to_string(), vec![]);
        let display = format!("{}", key);
        assert_eq!(display, "test");
    }

    #[test]
    fn test_specialization_key_clone() {
        let key = SpecializationKey::new("test".to_string(), vec![MonoType::Bool]);
        let cloned = key.clone();
        assert_eq!(key.to_string(), cloned.to_string());
    }
}

#[cfg(test)]
mod generic_function_id_tests {
    use super::*;

    #[test]
    fn test_generic_function_id_no_params() {
        let id = GenericFunctionId::new("main".to_string(), vec![]);
        assert_eq!(id.name(), "main");
        assert!(id.type_params().is_empty());
        assert_eq!(id.signature(), "main");
    }

    #[test]
    fn test_generic_function_id_with_one_param() {
        let id = GenericFunctionId::new(
            "identity".to_string(),
            vec!["T".to_string()],
        );
        assert_eq!(id.name(), "identity");
        assert_eq!(id.type_params(), vec!["T"]);
        assert_eq!(id.signature(), "identity<T>");
    }

    #[test]
    fn test_generic_function_id_with_multiple_params() {
        let id = GenericFunctionId::new(
            "pair".to_string(),
            vec!["T".to_string(), "U".to_string()],
        );
        assert_eq!(id.signature(), "pair<T, U>");
    }

    #[test]
    fn test_generic_function_id_display() {
        let id = GenericFunctionId::new("test".to_string(), vec!["T".to_string()]);
        let display = format!("{}", id);
        assert_eq!(display, "test<T>");
    }

    #[test]
    fn test_generic_function_id_partial_eq() {
        let id1 = GenericFunctionId::new("func".to_string(), vec!["T".to_string()]);
        let id2 = GenericFunctionId::new("func".to_string(), vec!["T".to_string()]);
        let id3 = GenericFunctionId::new("func".to_string(), vec!["U".to_string()]);
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_generic_function_id_clone() {
        let id = GenericFunctionId::new("test".to_string(), vec!["T".to_string()]);
        let cloned = id.clone();
        assert_eq!(id.signature(), cloned.signature());
    }
}

#[cfg(test)]
mod function_id_tests {
    use super::*;

    #[test]
    fn test_function_id_no_args() {
        let id = FunctionId::new("main".to_string(), vec![]);
        assert_eq!(id.name(), "main");
        assert_eq!(id.specialized_name(), "main");
    }

    #[test]
    fn test_function_id_with_one_arg() {
        let id = FunctionId::new(
            "identity".to_string(),
            vec![MonoType::Int(64)],
        );
        assert_eq!(id.specialized_name(), "identity_int64");
    }

    #[test]
    fn test_function_id_with_multiple_args() {
        let id = FunctionId::new(
            "combine".to_string(),
            vec![MonoType::Int(32), MonoType::Float(64)],
        );
        assert_eq!(id.specialized_name(), "combine_int32_float64");
    }

    #[test]
    fn test_function_id_display() {
        let id = FunctionId::new("test".to_string(), vec![]);
        let display = format!("{}", id);
        assert_eq!(display, "test");
    }

    #[test]
    fn test_function_id_partial_eq() {
        let id1 = FunctionId::new("func".to_string(), vec![MonoType::Int(64)]);
        let id2 = FunctionId::new("func".to_string(), vec![MonoType::Int(64)]);
        let id3 = FunctionId::new("func".to_string(), vec![MonoType::Float(64)]);
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_function_id_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hash;
        
        let id1 = FunctionId::new("func".to_string(), vec![MonoType::Int(64)]);
        let id2 = FunctionId::new("func".to_string(), vec![MonoType::Int(64)]);
        
        let mut hasher1 = DefaultHasher::new();
        id1.hash(&mut hasher1);
        let hash1 = hasher1.finish();
        
        let mut hasher2 = DefaultHasher::new();
        id2.hash(&mut hasher2);
        let hash2 = hasher2.finish();
        
        assert_eq!(hash1, hash2);
    }
}

#[cfg(test)]
mod instantiation_request_tests {
    use super::*;

    #[test]
    fn test_instantiation_request_basic() {
        let generic_id = GenericFunctionId::new(
            "identity".to_string(),
            vec!["T".to_string()],
        );
        let type_args = vec![MonoType::Int(64)];
        let span = Span::dummy();
        let request = InstantiationRequest::new(generic_id.clone(), type_args.clone(), span);
        
        assert_eq!(request.generic_id(), &generic_id);
        assert_eq!(request.type_args().len(), 1);
        assert!(matches!(request.type_args()[0], MonoType::Int(64)));
    }

    #[test]
    fn test_instantiation_request_specialization_key() {
        let generic_id = GenericFunctionId::new(
            "identity".to_string(),
            vec!["T".to_string()],
        );
        let type_args = vec![MonoType::Int(64)];
        let span = Span::dummy();
        let request = InstantiationRequest::new(generic_id, type_args, span);
        
        let key = request.specialization_key();
        assert_eq!(key.to_string(), "identity<int64>");
    }

    #[test]
    fn test_instantiation_request_multiple_type_args() {
        let generic_id = GenericFunctionId::new(
            "map".to_string(),
            vec!["T".to_string(), "U".to_string()],
        );
        let type_args = vec![MonoType::Int(32), MonoType::Float(64)];
        let span = Span::new(
            crate::util::span::Position::new(1, 1),
            crate::util::span::Position::new(1, 10),
        );
        let request = InstantiationRequest::new(generic_id, type_args, span);
        
        assert_eq!(request.type_args().len(), 2);
    }

    #[test]
    fn test_instantiation_request_clone() {
        let generic_id = GenericFunctionId::new("test".to_string(), vec![]);
        let type_args = vec![MonoType::Bool];
        let span = Span::dummy();
        let request = InstantiationRequest::new(generic_id, type_args, span);
        
        let cloned = request.clone();
        assert_eq!(request.generic_id().name(), cloned.generic_id().name());
    }
}

#[cfg(test)]
mod function_instance_tests {
    use super::*;

    #[test]
    fn test_function_instance_creation() {
        let generic_id = GenericFunctionId::new(
            "identity".to_string(),
            vec!["T".to_string()],
        );
        let type_args = vec![MonoType::Int(64)];
        let id = FunctionId::new("identity_int64".to_string(), type_args.clone());
        
        let instance = FunctionInstance::new(id, generic_id, type_args);
        assert!(instance.ir.is_none());
    }

    #[test]
    fn test_function_instance_set_ir() {
        let generic_id = GenericFunctionId::new("test".to_string(), vec![]);
        let id = FunctionId::new("test".to_string(), vec![]);
        let mut instance = FunctionInstance::new(id, generic_id, vec![]);
        
        let ir = crate::middle::ir::FunctionIR {
            name: "test".to_string(),
            params: vec![],
            return_type: crate::frontend::typecheck::MonoType::Void,
            locals: vec![],
            instructions: vec![],
            spans: vec![],
        };
        
        instance.set_ir(ir);
        assert!(instance.ir.is_some());
    }

    #[test]
    fn test_function_instance_get_ir() {
        let generic_id = GenericFunctionId::new("test".to_string(), vec![]);
        let id = FunctionId::new("test".to_string(), vec![]);
        let mut instance = FunctionInstance::new(id, generic_id, vec![]);
        
        assert!(instance.get_ir().is_none());
        
        let ir = crate::middle::ir::FunctionIR {
            name: "test".to_string(),
            params: vec![],
            return_type: crate::frontend::typecheck::MonoType::Void,
            locals: vec![],
            instructions: vec![],
            spans: vec![],
        };
        
        instance.set_ir(ir);
        assert!(instance.get_ir().is_some());
        assert_eq!(instance.get_ir().unwrap().name, "test");
    }

    #[test]
    fn test_function_instance_clone() {
        let generic_id = GenericFunctionId::new("test".to_string(), vec![]);
        let id = FunctionId::new("test".to_string(), vec![]);
        let instance = FunctionInstance::new(id, generic_id, vec![]);
        
        let cloned = instance.clone();
        assert!(cloned.ir.is_none());
    }
}
