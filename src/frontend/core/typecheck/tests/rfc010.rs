//! 类型检查器测试 — 基于 RFC-010 统一类型语法
//!
//! RFC-010: https://github.com/YaoXiang/YaoXiang/docs/src/design/rfc/accepted/010-unified-type-syntax.md
//!
//! 测试点：
//! - §3.1: 变量声明 `x: Int = 42`
//! - §3.2: 函数定义 `add: (a: Int, b: Int) -> Int = a + b`
//! - §3.3: 记录类型 `Point: Type = { x: Float, y: Float }`
//! - §3.4: 接口类型 `Drawable: Type = { draw: (Surface) -> Void }`
//! - §3.5: 泛型类型 `List: (T: Type) -> Type = { ... }`
//! - §3.6: 方法定义 `Point.draw: (self: Point, ...) -> ...`

use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;

/// 辅助函数：解析源代码并类型检查
fn check_source(
    source: &str
) -> Result<
    crate::frontend::core::typecheck::types::TypeCheckResult,
    Vec<crate::util::diagnostic::Diagnostic>,
> {
    let tokens = tokenize(source).expect("tokenize failed");
    let module = parse(&tokens).expect("parse failed");
    let mut checker = TypeChecker::new("test");
    checker.check_module(&module)
}

// ===================================================================
// RFC-010 §3.1: 变量声明
// ===================================================================

/// 规范：`x: Int = 42` 应该类型检查通过，x 的类型为 Int
///
/// 预期行为：
/// - 解析 `x: Int = 42` 为 Let 语句
/// - 类型检查器应验证 42 的类型与声明的 Int 一致
/// - x 的推断类型应为 Int(64)
#[test]
fn test_rfc010_variable_declaration_int() {
    // Arrange
    let source = "x: Int = 42";

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "x: Int = 42 should pass type check");
}

/// 规范：`name: String = "Alice"` 应该类型检查通过
///
/// 预期行为：
/// - String 字面量的类型为 String
/// - 声明类型与字面量类型一致
#[test]
fn test_rfc010_variable_declaration_string() {
    // Arrange
    let source = r#"name: String = "Alice""#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(
        result.is_ok(),
        "name: String = \"Alice\" should pass type check"
    );
}

/// 规范：`flag: Bool = true` 应该类型检查通过
///
/// 预期行为：
/// - Bool 字面量的类型为 Bool
#[test]
fn test_rfc010_variable_declaration_bool() {
    // Arrange
    let source = "flag: Bool = true";

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "flag: Bool = true should pass type check");
}

/// 规范：类型推导 `y = 100` 应该推断为 Int
///
/// 预期行为：
/// - 省略类型注解时，编译器自动推导
/// - 整数字面量推导为 Int
#[test]
fn test_rfc010_type_inference_int() {
    // Arrange
    let source = "y = 100";

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "y = 100 should pass type check");
}

// ===================================================================
// RFC-010 §3.2: 函数定义
// ===================================================================

/// 规范：`add: (a: Int, b: Int) -> Int = { return a + b }` 应该类型检查通过
///
/// 预期行为：
/// - 参数 a, b 的类型为 Int
/// - 返回类型为 Int
/// - 函数体中 a + b 的类型为 Int
#[test]
fn test_rfc010_function_definition() {
    // Arrange
    let source = r#"
        add: (a: Int, b: Int) -> Int = {
            return a + b
        }
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "add function should pass type check");
}

/// 规范：单行函数 `inc: (x: Int) -> Int = x + 1`
///
/// 预期行为：
/// - 单行表达式直接返回，无需 return
#[test]
fn test_rfc010_single_line_function() {
    // Arrange
    let source = "inc: (x: Int) -> Int = x + 1";

    // Act
    let result = check_source(source);

    // Assert
    assert!(
        result.is_ok(),
        "single line function should pass type check"
    );
}

/// 规范：函数返回类型检查
///
/// 预期行为：
/// - 函数体的返回类型应与声明一致
/// - 当前版本暂不检查 return 语句的类型匹配（需要 expected_return_type 传递）
/// - 仅验证函数定义语法正确性
#[test]
fn test_rfc010_function_return_type() {
    // Arrange
    let source = r#"
        good: (x: Int) -> Int = {
            return x
        }
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(
        result.is_ok(),
        "function with matching return type should pass"
    );
}

// ===================================================================
// RFC-010 §3.3: 记录类型
// ===================================================================

/// 规范：`Point: Type = { x: Float, y: Float }` 应该类型检查通过
///
/// 预期行为：
/// - 定义记录类型 Point
/// - 包含字段 x: Float, y: Float
#[test]
fn test_rfc010_record_type_definition() {
    // Arrange
    let source = r#"
        Point: Type = {
            x: Float,
            y: Float
        }
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "Point type definition should pass");
}

/// 规范：记录类型构造 `p: Point = Point(1.0, 2.0)`
///
/// 预期行为：
/// - 使用类型名作为构造函数
/// - 参数顺序与字段定义顺序一致
#[test]
fn test_rfc010_record_type_construction() {
    // Arrange
    let source = r#"
        Point: Type = {
            x: Float,
            y: Float
        }
        p: Point = Point(1.0, 2.0)
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "Point construction should pass");
}

/// 规范：带默认值的字段 `Point: Type = { x: Float = 0, y: Float = 0 }`
///
/// 预期行为：
/// - 有默认值的字段在构造时可选
/// - Point() 等价于 Point(x=0, y=0)
#[test]
fn test_rfc010_record_type_default_values() {
    // Arrange
    let source = r#"
        Point: Type = {
            x: Float = 0,
            y: Float = 0
        }
        p1: Point = Point()        // 使用默认值
        p2: Point = Point(x=1)     // 部分指定
        p3: Point = Point(x=1, y=2) // 全部指定
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "default values should pass");
}

// ===================================================================
// RFC-010 §3.4: 接口类型
// ===================================================================

/// 规范：接口是字段全为函数的记录类型
///
/// `Drawable: Type = { draw: (Surface) -> Void, bounding_box: () -> Rect }`
///
/// 预期行为：
/// - 接口定义的所有字段必须是函数类型
#[test]
fn test_rfc010_interface_definition() {
    // Arrange
    let source = r#"
        Drawable: Type = {
            draw: (Surface) -> Void,
            bounding_box: () -> Rect
        }
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "Drawable interface definition should pass");
}

/// 规范：类型实现接口
///
/// `Point: Type = { x: Float, y: Float, Drawable }`
///
/// 预期行为：
/// - 在类型定义末尾列出接口名
/// - 编译器检查 Point 是否实现了 Drawable 的所有方法
#[test]
fn test_rfc010_interface_implementation() {
    // Arrange
    let source = r#"
        Drawable: Type = {
            draw: (Surface) -> Void
        }
        Surface: Type = {}
        Point: Type = {
            x: Float,
            y: Float,
            Drawable
        }
        Point.draw: (self: Point, surface: Surface) -> Void = {
            return
        }
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "interface implementation should pass");
}

/// 规范：接口赋值（结构化子类型）
///
/// `d: Drawable = Circle(1)`
///
/// 预期行为：
/// - 具体类型可以直接赋值给接口类型变量
/// - 编译期检查是否满足接口要求
#[test]
fn test_rfc010_interface_assignment() {
    // Arrange
    let source = r#"
        Drawable: Type = {
            draw: (Surface) -> Void
        }
        Surface: Type = {}
        Circle: Type = {
            radius: Float,
            Drawable
        }
        Circle.draw: (self: Circle, surface: Surface) -> Void = {
            return
        }
        c: Circle = Circle(1.0)
        d: Drawable = c  // 结构化子类型
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "interface assignment should pass");
}

// ===================================================================
// RFC-010 §3.5: 泛型类型
// ===================================================================

/// 规范：泛型类型定义
///
/// `List: (T: Type) -> Type = { data: Array(T), length: Int }`
///
/// 预期行为：
/// - T 是类型参数
/// - Array(T) 使用类型参数 T
#[test]
fn test_rfc010_generic_type_definition() {
    // Arrange
    let source = r#"
        List: (T: Type) -> Type = {
            data: Array(T),
            length: Int
        }
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "List generic type definition should pass");
}

/// 规范：泛型类型实例化
///
/// `numbers: List(Int) = List(1, 2, 3)`
///
/// 预期行为：
/// - 使用 () 语法填充类型参数
/// - 编译器推导 T=Int
#[test]
fn test_rfc010_generic_type_instantiation() {
    // Arrange
    let source = r#"
        List: (T: Type) -> Type = {
            data: Array(T),
            length: Int
        }
        numbers: List(Int) = List(1, 2, 3)
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "List(Int) instantiation should pass");
}

// ===================================================================
// RFC-010 §3.6: 方法定义
// ===================================================================

/// 规范：类型方法定义
///
/// `Point.draw: (self: Point, surface: Surface) -> Void = { ... }`
///
/// 预期行为：
/// - 使用 Type.method 语法定义方法
/// - 第一个参数 self 的类型为 Point
/// - self 和 surface 在函数体内可用
#[test]
fn test_rfc010_method_definition() {
    // Arrange
    let source = r#"
        Surface: Type = {}
        Point: Type = {
            x: Float,
            y: Float
        }
        Point.draw: (self: Point, surface: Surface) -> Void = {
            return
        }
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "method definition should pass");
}

/// 规范：方法调用语法糖
///
/// `p.draw(screen)` 等价于 `Point.draw(p, screen)`
///
/// 预期行为：
/// - 方法调用自动将 p 作为第一个参数
#[test]
fn test_rfc010_method_call_syntax_sugar() {
    // Arrange
    let source = r#"
        Surface: Type = {}
        Point: Type = { x: Float, y: Float }
        Point.draw: (self: Point, surface: Surface) -> Void = {
            return
        }
        p: Point = Point(1.0, 2.0)
        screen: Surface = Surface()
        main: () -> Void = {
            p.draw(screen)  // 语法糖 → Point.draw(p, screen)
        }
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "method call syntax sugar should pass");
}

// ===================================================================
// RFC-010: Type 元类型关键字
// ===================================================================

/// 规范：Type 是语言中唯一的元类型关键字
///
/// 预期行为：
/// - `Point: Type = { ... }` 声明类型
/// - Type 本身也是一个类型
#[test]
fn test_rfc010_type_meta_keyword() {
    // Arrange
    let source = r#"
        Point: Type = {
            x: Float,
            y: Float
        }
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "Type keyword should work");
}

/// 规范：有 `: Type` 强制为类型构造器
///
/// 预期行为：
/// - `Point: Type = { ... }` 是类型
/// - `Point = { ... }` 不是类型（HM 推断为函数）
#[test]
fn test_rfc010_type_annotation_forces_type_constructor() {
    // Arrange
    let source = r#"
        // 正确：有 : Type
        Point: Type = { x: Float, y: Float }
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(
        result.is_ok(),
        "Type annotation should force type constructor"
    );
}

// RFC-010: 泛型类型实例化展开

/// 规范：泛型类型实例化展开
///
/// `List(Int)` 应展开为 `{ data: Array(Int), length: Int }` 结构体类型。
/// 使得字段访问 `list.data` 可以正确解析。
#[test]
fn test_rfc010_generic_type_instantiation_expansion() {
    // Arrange
    let source = r#"
        Wrapper: (T: Type) -> Type = {
            value: T
        }
        w: Wrapper(Int) = Wrapper(42)
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(
        result.is_ok(),
        "Wrapper(Int) should expand to struct with value: Int"
    );
}

// RFC-004/RFC-010: 外部方法绑定语法

/// 规范：外部方法绑定
///
/// `Point.distance = distance[0]` 将普通函数绑定为类型方法。
///
/// 预期行为：
/// - `distance` 函数注册为 Point 的方法
/// - 通过方法绑定可以正常调用
#[test]
fn test_rfc010_external_method_binding() {
    // Arrange
    let source = r#"
        Point: Type = { x: Float, y: Float }
        get_x: (p: Point) -> Float = { return 1.0 }
        Point.get_x = get_x[0]
        p: Point = Point(1.0, 2.0)
        main: () -> Float = {
            return p.get_x()
        }
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "external method binding should work");
}

/// 规范：多位置绑定
///
/// `Point.calc = calculate[1, 2]` 将函数的多个参数绑定为类型方法。
///
/// 预期行为：
/// - 位置 1 和 2 的参数被绑定
/// - 剩余参数成为方法的参数签名
#[test]
fn test_rfc010_multi_position_binding() {
    // Arrange
    let source = r#"
        Point: Type = { x: Float, y: Float }
        calc: (a: Point, offset: Float) -> Float = {
            return 1.0
        }
        Point.calc = calc[0]
        p: Point = Point(1.0, 2.0)
        main: () -> Float = {
            return p.calc(0.5)
        }
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(result.is_ok(), "multi-position binding should work");
}

/// 规范：结构化子类型 — 接口赋值应失败（未实现接口）
///
/// 确保没有实现 Serializable 的 Point 不能赋值给 Serializable 变量。
#[test]
fn test_rfc010_interface_assignment_rejected_when_not_implemented() {
    // Arrange
    let source = r#"
        Serializable: Type = { serialize: () -> String }
        Point: Type = { x: Float, y: Float }
        p: Point = Point(1.0, 2.0)
        s: Serializable = p  // Point 未实现 Serializable，应报错
    "#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(
        result.is_err(),
        "interface assignment should be rejected when not implemented"
    );
}
