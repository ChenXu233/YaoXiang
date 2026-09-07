//! RFC-011a §6 动态分发——typecheck 层单测
//!
//! 覆盖：
//! - 存在类型强制点收集（existential_coercions：span 键 + 接口名）
//! - 字面量逐元素成员检查（主 unify 只作用首个元素的缺口由走查兜住）
//! - E1101（具体类型未实现接口）在字面量 / append 实参两个决策点
//!
//! 参照 docs/src/design/rfc/accepted/011a-interface-implementation.md §6

use super::rfc011a::check_source_with_checker;

const ANIMAL_DOG_CAT: &str = r#"
    Animal: (Self: Type) -> Type = {
        speak: (self: Self) -> String,
    }
    Dog: Type = {
        name: String,
        Animal(Dog),
    }
    Dog.speak: (self: Dog) -> String = { return "Woof" }
    Cat: Type = {
        lives: Int,
        Animal(Cat),
    }
    Cat.speak: (self: Cat) -> String = { return "Meow" }
    Stone: Type = {
        weight: Int,
    }
"#;

#[test]
fn test_rfc011a_dispatch_coercions_collected() {
    let source = format!(
        "{}\n
    main = {{
        animals: List(Animal) = [Dog(\"Rex\"), Cat(9)]
        x: Animal = Dog(\"Bella\")
    }}",
        ANIMAL_DOG_CAT
    );
    let (result, _checker) = check_source_with_checker(&source);
    assert!(
        result.diagnostics.is_empty(),
        "expect no diagnostics, got: {:?}",
        result.diagnostics
    );
    // 字面量 2 个元素 + 标量赋值 1 处 = 3 个包装点
    assert_eq!(
        result.existential_coercions.len(),
        3,
        "coercions: {:?}",
        result.existential_coercions
    );
    assert!(result
        .existential_coercions
        .iter()
        .all(|c| c.interface == "Animal"));
    // span 两两不同（ir_gen 查表键）
    let mut spans: Vec<_> = result
        .existential_coercions
        .iter()
        .map(|c| c.span)
        .collect();
    spans.dedup();
    assert_eq!(spans.len(), 3);
}

#[test]
fn test_rfc011a_dispatch_literal_elem_violation_e1101() {
    let source = format!(
        "{}\n
    main = {{
        animals: List(Animal) = [Dog(\"Rex\"), Stone(2)]
    }}",
        ANIMAL_DOG_CAT
    );
    let (result, _checker) = check_source_with_checker(&source);
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1101"),
        "expect E1101 for non-implementing literal element, got: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.code)
            .collect::<Vec<_>>()
    );
    // 违规时不产生任何包装点
    assert!(result.existential_coercions.is_empty());
}

#[test]
fn test_rfc011a_dispatch_append_arg_violation_e1101() {
    let source = format!(
        "{}\n
    use std.list

    main = {{
        animals: List(Animal) = [Dog(\"Rex\")]
        animals2 = list.append(animals, Stone(2))
    }}",
        ANIMAL_DOG_CAT
    );
    let (result, _checker) = check_source_with_checker(&source);
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1101"),
        "expect E1101 for non-implementing append argument, got: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_rfc011a_dispatch_fn_param_coercion() {
    let source = format!(
        "{}\n
    describe: (a: Animal) -> String = {{
        return a.speak()
    }}
    main = {{
        r1 = describe(Dog(\"Rex\"))
        r2 = describe(Cat(9))
    }}",
        ANIMAL_DOG_CAT
    );
    let (result, _checker) = check_source_with_checker(&source);
    assert!(
        result.diagnostics.is_empty(),
        "expect no diagnostics, got: {:?}",
        result.diagnostics
    );
    // 两处调用实参 = 2 个包装点
    assert_eq!(
        result.existential_coercions.len(),
        2,
        "coercions: {:?}",
        result.existential_coercions
    );
}

#[test]
fn test_rfc011a_dispatch_return_position_coercion() {
    let source = format!(
        "{}\n
    make_dog: () -> Animal = {{
        return Dog(\"Rex\")
    }}",
        ANIMAL_DOG_CAT
    );
    let (result, _checker) = check_source_with_checker(&source);
    assert!(
        result.diagnostics.is_empty(),
        "expect no diagnostics, got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.existential_coercions.len(), 1);
    assert_eq!(result.existential_coercions[0].interface, "Animal");
}
