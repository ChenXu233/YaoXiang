//! Monomorphize Instance 单元测试
//!
//! 测试单态化过程中的实例化请求、缓存键和函数实例

use crate::frontend::typecheck::MonoType;
use crate::middle::monomorphize::instance::{
    FunctionId, FunctionInstance, GenericFunctionId, GenericTypeId, InstantiationRequest,
    SpecializationKey, TypeId, TypeInstance,
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
        assert_eq!(key.to_string(), "len<List<int32>>");
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
        use std::hash::{Hash, Hasher};

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
            is_async: false,
            blocks: vec![],
            entry: 0,
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
            is_async: false,
            blocks: vec![],
            entry: 0,
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

    #[test]
    fn test_function_instance_with_type_args() {
        let generic_id = GenericFunctionId::new(
            "map".to_string(),
            vec!["T".to_string(), "U".to_string()],
        );
        let type_args = vec![MonoType::Int(64), MonoType::String];
        let id = FunctionId::new("map_int64_string".to_string(), type_args.clone());
        let instance = FunctionInstance::new(id.clone(), generic_id.clone(), type_args.clone());

        assert_eq!(instance.id.name(), "map_int64_string");
        assert_eq!(instance.type_args.len(), 2);
        assert!(instance.ir.is_none());
    }

    #[test]
    fn test_function_instance_multiple_increments() {
        let generic_id = GenericFunctionId::new("inc".to_string(), vec!["T".to_string()]);

        let id1 = FunctionId::new("inc_int64".to_string(), vec![MonoType::Int(64)]);
        let instance1 = FunctionInstance::new(id1, generic_id.clone(), vec![MonoType::Int(64)]);

        let id2 = FunctionId::new("inc_float64".to_string(), vec![MonoType::Float(64)]);
        let instance2 = FunctionInstance::new(id2, generic_id.clone(), vec![MonoType::Float(64)]);

        assert_ne!(instance1.id.specialized_name(), instance2.id.specialized_name());
    }
}

#[cfg(test)]
mod type_id_tests {
    use super::*;

    #[test]
    fn test_type_id_no_args() {
        let id = TypeId::new("MyStruct".to_string(), vec![]);
        assert_eq!(id.name(), "MyStruct");
        assert_eq!(id.specialized_name(), "MyStruct");
    }

    #[test]
    fn test_type_id_with_one_arg() {
        let id = TypeId::new("Option".to_string(), vec![MonoType::Int(64)]);
        assert_eq!(id.specialized_name(), "Option_int64");
    }

    #[test]
    fn test_type_id_with_multiple_args() {
        let id = TypeId::new(
            "Map".to_string(),
            vec![MonoType::String, MonoType::Int(32)],
        );
        assert_eq!(id.specialized_name(), "Map_string_int32");
    }

    #[test]
    fn test_type_id_partial_eq() {
        let id1 = TypeId::new("Test".to_string(), vec![MonoType::Bool]);
        let id2 = TypeId::new("Test".to_string(), vec![MonoType::Bool]);
        let id3 = TypeId::new("Test".to_string(), vec![MonoType::Int(64)]);
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_type_id_clone() {
        let id = TypeId::new("Clone".to_string(), vec![MonoType::Float(32)]);
        let cloned = id.clone();
        assert_eq!(id.specialized_name(), cloned.specialized_name());
    }

    #[test]
    fn test_type_id_display() {
        let id = TypeId::new("Display".to_string(), vec![]);
        let display = format!("{}", id);
        assert_eq!(display, "Display");
    }

    #[test]
    fn test_type_id_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let id1 = TypeId::new("Hashable".to_string(), vec![MonoType::String]);
        let id2 = TypeId::new("Hashable".to_string(), vec![MonoType::String]);

        let mut hasher1 = DefaultHasher::new();
        id1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        id2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_type_id_in_hashmap() {
        use std::collections::HashMap;

        let mut map: HashMap<TypeId, String> = HashMap::new();

        let id1 = TypeId::new("Key".to_string(), vec![MonoType::Int(64)]);
        let id2 = TypeId::new("Key".to_string(), vec![MonoType::Int(64)]);
        let id3 = TypeId::new("Key".to_string(), vec![MonoType::String]);

        map.insert(id1.clone(), "value1".to_string());
        assert_eq!(map.get(&id1), Some(&"value1".to_string()));
        assert_eq!(map.get(&id2), Some(&"value1".to_string())); // Same key
        assert_eq!(map.get(&id3), None); // Different key
    }
}

#[cfg(test)]
mod type_instance_tests {
    use super::*;

    #[test]
    fn test_type_instance_creation() {
        let generic_id = GenericTypeId::new("Option".to_string(), vec!["T".to_string()]);
        let type_args = vec![MonoType::Int(64)];
        let id = TypeId::new("Option_int64".to_string(), type_args.clone());

        let instance = TypeInstance::new(id.clone(), generic_id, type_args.clone());
        assert!(instance.mono_type.is_none());
    }

    #[test]
    fn test_type_instance_set_mono_type() {
        let generic_id = GenericTypeId::new("List".to_string(), vec!["T".to_string()]);
        let id = TypeId::new("List_int64".to_string(), vec![MonoType::Int(64)]);
        let mut instance = TypeInstance::new(id, generic_id, vec![MonoType::Int(64)]);

        let mono_type = MonoType::List(Box::new(MonoType::Int(64)));
        instance.set_mono_type(mono_type.clone());

        assert!(instance.mono_type.is_some());
        assert_eq!(instance.get_mono_type(), Some(&mono_type));
    }

    #[test]
    fn test_type_instance_get_mono_type() {
        let generic_id = GenericTypeId::new("Box".to_string(), vec!["T".to_string()]);
        let id = TypeId::new("Box_string".to_string(), vec![MonoType::String]);
        let mut instance = TypeInstance::new(id, generic_id, vec![MonoType::String]);

        assert!(instance.get_mono_type().is_none());

        let mono_type = MonoType::Arc(Box::new(MonoType::String));
        instance.set_mono_type(mono_type);
        assert!(instance.get_mono_type().is_some());
    }

    #[test]
    fn test_type_instance_clone() {
        let generic_id = GenericTypeId::new("Ref".to_string(), vec!["T".to_string()]);
        let id = TypeId::new("Ref_bool".to_string(), vec![MonoType::Bool]);
        let instance = TypeInstance::new(id, generic_id, vec![MonoType::Bool]);

        let cloned = instance.clone();
        assert!(cloned.mono_type.is_none());
    }
}

#[cfg(test)]
mod generic_type_id_tests {
    use super::*;

    #[test]
    fn test_generic_type_id_no_params() {
        let id = GenericTypeId::new("Simple".to_string(), vec![]);
        assert_eq!(id.name(), "Simple");
        assert!(id.type_params().is_empty());
    }

    #[test]
    fn test_generic_type_id_with_params() {
        let id = GenericTypeId::new(
            "Map".to_string(),
            vec!["K".to_string(), "V".to_string()],
        );
        assert_eq!(id.name(), "Map");
        assert_eq!(id.type_params().len(), 2);
        assert_eq!(id.type_params()[0], "K");
        assert_eq!(id.type_params()[1], "V");
    }

    #[test]
    fn test_generic_type_id_display() {
        let id = GenericTypeId::new("Generic".to_string(), vec!["T".to_string()]);
        let display = format!("{}", id);
        assert_eq!(display, "Generic<T>");
    }

    #[test]
    fn test_generic_type_id_partial_eq() {
        let id1 = GenericTypeId::new("Eq".to_string(), vec!["T".to_string()]);
        let id2 = GenericTypeId::new("Eq".to_string(), vec!["T".to_string()]);
        let id3 = GenericTypeId::new("Eq".to_string(), vec!["U".to_string()]);
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_generic_type_id_clone() {
        let id = GenericTypeId::new("Clone".to_string(), vec!["T".to_string()]);
        let cloned = id.clone();
        assert_eq!(id.name(), cloned.name());
    }
}

#[cfg(test)]
mod specialization_key_edge_cases {
    use super::*;

    #[test]
    fn test_specialization_key_with_tuple() {
        let key = SpecializationKey::new(
            "tuple_func".to_string(),
            vec![MonoType::Tuple(vec![MonoType::Int(32), MonoType::String])],
        );
        assert!(key.to_string().contains("tuple"));
    }

    #[test]
    fn test_specialization_key_with_dict() {
        let key = SpecializationKey::new(
            "dict_func".to_string(),
            vec![MonoType::Dict(
                Box::new(MonoType::String),
                Box::new(MonoType::Int(64)),
            )],
        );
        let key_str = key.to_string();
        assert!(key_str.contains("dict"));
    }

    #[test]
    fn test_specialization_key_with_set() {
        let key = SpecializationKey::new(
            "set_func".to_string(),
            vec![MonoType::Set(Box::new(MonoType::Int(32)))],
        );
        let key_str = key.to_string();
        assert!(key_str.contains("set"));
    }

    #[test]
    fn test_specialization_key_with_fn_type() {
        let key = SpecializationKey::new(
            "callback".to_string(),
            vec![MonoType::Fn {
                is_async: false,
                params: vec![MonoType::Int(64)],
                return_type: Box::new(MonoType::Bool),
            }],
        );
        let key_str = key.to_string();
        assert!(key_str.contains("fn"));
    }

    #[test]
    fn test_specialization_key_with_range() {
        let key = SpecializationKey::new(
            "range_func".to_string(),
            vec![MonoType::Range {
                elem_type: Box::new(MonoType::Int(32)),
            }],
        );
        let key_str = key.to_string();
        assert!(key_str.contains("range"));
    }

    #[test]
    fn test_specialization_key_equality_with_same_args() {
        let key1 = SpecializationKey::new(
            "compare".to_string(),
            vec![MonoType::Int(64), MonoType::String],
        );
        let key2 = SpecializationKey::new(
            "compare".to_string(),
            vec![MonoType::Int(64), MonoType::String],
        );
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_specialization_key_inequality_different_order() {
        let key1 = SpecializationKey::new(
            "order".to_string(),
            vec![MonoType::Int(64), MonoType::String],
        );
        let key2 = SpecializationKey::new(
            "order".to_string(),
            vec![MonoType::String, MonoType::Int(64)],
        );
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_specialization_key_inequality_different_name() {
        let key1 = SpecializationKey::new("name1".to_string(), vec![MonoType::Int(64)]);
        let key2 = SpecializationKey::new("name2".to_string(), vec![MonoType::Int(64)]);
        assert_ne!(key1, key2);
    }
}
