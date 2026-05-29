# RFC-010 统一类型语法 - 待实现功能文档

> **创建日期**: 2026-02-03
> **状态**: 待实现
> **基于 RFC**: RFC-010 统一类型语法

## 概述

本文档描述 RFC-010 统一类型语法设计中尚未实现或实现不完整的部分，作为后续开发的参考指南。

---

## 1. 方法绑定语法解析

### 1.1 问题描述

RFC-010 设计了 `Type.method: (Type, ...) -> ReturnType = ...` 方法定义语法，但解析器目前缺乏对这种语法的支持。

**期望语法**：
```yaoxiang
# 类型方法定义
Point.draw: (Point, Surface) -> Void = (self, surface) => {
    surface.plot(self.x, self.y)
}

Point.serialize: (Point) -> String = (self) => {
    "Point(${self.x}, ${self.y})"
}
```

**当前状态**：
- AST 有 `MethodBind` 节点定义 (`src/frontend/core/parser/ast.rs:184-195`)
- 解析器 `declarations.rs` 缺乏对应的语法解析逻辑

### 1.2 需要的修改

#### 1.2.1 修改 `parse_type_annotation` 或新增解析函数

在 `src/frontend/core/parser/statements/declarations.rs` 中添加方法绑定语法识别：

```rust
/// 检测是否是方法绑定语法: `Type.method: (Params) -> ReturnType`
fn is_method_bind_syntax(state: &mut ParserState<'_>) -> bool {
    let saved = state.save_position();

    // 检查是否有点号分隔的类型名和方法名
    // 例如: Point.draw: (Point, Surface) -> Void = ...
    let has_dot_method = if let Some(TokenKind::Identifier(_)) = state.current().map(|t| &t.kind) {
        state.bump();
        state.at(&TokenKind::Dot)
    } else {
        false
    };

    state.restore_position(saved);
    has_dot_method
}
```

#### 1.2.2 新增方法绑定解析函数

```rust
/// Parse method binding: `Type.method: (Params) -> ReturnType = (params) => body`
pub fn parse_method_bind_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    // 解析类型名
    let type_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => return None,
    };
    state.bump();

    // 消费点号
    state.expect(&TokenKind::Dot)?;

    // 解析方法名
    let method_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => return None,
    };
    state.bump();

    // 消费冒号
    state.expect(&TokenKind::Colon)?;

    // 解析方法类型
    let method_type = parse_type_annotation(state)?;

    // 消费等号
    state.expect(&TokenKind::Eq)?;

    // 解析方法体
    let body = parse_fn_body(state)?;

    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::MethodBind {
            type_name,
            method_name,
            method_type,
            params: body.0,
            body: body.1,
        },
        span,
    })
}
```

### 1.3 测试用例

```rust
#[test]
fn test_method_bind_parsing() {
    let code = r#"
        Point.draw: (Point, Surface) -> Void = (self, surface) => {
            surface.plot(self.x, self.y)
        }
    "#;

    let ast = parse(code).unwrap();
    assert!(matches!(
        ast.items[0].kind,
        StmtKind::MethodBind {
            type_name: ref n,
            method_name: ref m,
            ..
        } if n == "Point" && m == "draw"
    ));
}
```

---

## 2. pub 自动绑定机制

### 2.1 问题描述

RFC-010 设计了 `pub` 自动绑定机制：当函数使用 `pub` 声明时，编译器应自动将其绑定到同文件定义的类型。

**期望行为**：
```yaoxiang
# 使用 pub 声明，编译器自动绑定到 Point.distance
pub distance: (Point, Point) -> Float = (p1, p2) => {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# 等价于：
Point.distance = distance[0]

# 调用方式
d1 = distance(p1, p2)      # 函数式
d2 = p1.distance(p2)       # OOP 语法糖
```

**当前状态**：无相关实现

### 2.2 需要的修改

#### 2.2.1 修改解析器识别 pub 函数

在 `src/frontend/core/parser/statements/declarations.rs` 的 `parse_identifier_stmt` 函数中：

```rust
/// Parse statement starting with identifier
pub fn parse_identifier_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    // 检查是否是 pub 声明
    let is_pub = state.skip(&TokenKind::KwPub);

    // 后续逻辑...

    // 返回时标记 pub 状态
    Some(Stmt {
        kind: StmtKind::Fn {
            name,
            type_annotation,
            params,
            body,
            is_pub,  // 新增字段
        },
        span,
    })
}
```

#### 2.2.2 新增 AST 字段

修改 `src/frontend/core/parser/ast.rs` 中的 `StmtKind::Fn`：

```rust
/// Function definition: `name: Type = (params) => body`
pub struct FnStmt {
    pub name: String,
    pub type_annotation: Option<Type>,
    pub params: Vec<Param>,
    pub body: (Vec<Stmt>, Option<Box<Expr>>),
    pub is_pub: bool,  // 新增：是否自动绑定到类型
    pub auto_bind_type: Option<String>,  // 新增：自动绑定的目标类型
}
```

#### 2.2.3 类型检查阶段实现自动绑定

在 `src/frontend/typecheck/inference/statements.rs` 中：

```rust
/// 处理函数定义，支持 pub 自动绑定
fn infer_fn_stmt(
    &mut self,
    stmt: &Stmt,
    env: &mut TypeEnvironment,
) -> TypeResult<MonoType> {
    match &stmt.kind {
        StmtKind::Fn { name, params, return_type, body, is_pub, .. } => {
            // 构建函数类型
            let fn_type = self.infer_fn_type(params, return_type.as_ref())?;

            if *is_pub {
                // 尝试自动绑定到同文件定义的类型
                if let Some(target_type) = self.find_target_type_for_pub(name, params) {
                    self.bind_method_to_type(&target_type, name, &fn_type)?;
                }
            }

            // 注册到环境
            env.add_var(name.clone(), PolyType::mono(fn_type));

            Ok(MonoType::Void)
        }
        _ => unreachable!(),
    }
}

/// 查找 pub 函数应该绑定的目标类型
fn find_target_type_for_pub(
    &self,
    fn_name: &str,
    params: &[Param],
) -> Option<String> {
    // 规则：第一个参数的类型名作为绑定目标
    // 例如：distance: (Point, Point) -> Float 绑定到 Point
    if let Some(first_param) = params.first() {
        if let Some(ref ty) = first_param.ty {
            return Some(self.type_to_string(ty));
        }
    }
    None
}
```

### 2.3 测试用例

```rust
#[test]
fn test_pub_auto_bind() {
    let code = r#"
        type Point = {
            x: Float,
            y: Float
        }

        pub distance: (Point, Point) -> Float = (p1, p2) => {
            dx = p1.x - p2.x
            dy = p1.y - p2.y
            (dx * dx + dy * dy).sqrt()
        }
    "#;

    let type_env = typecheck(code).unwrap();

    // 检查 Point.distance 方法是否被绑定
    let point_type = type_env.get_type("Point").unwrap();
    assert!(point_type.methods.contains_key("distance"));
}
```

---

## 3. 泛型约束语法解析

### 3.1 问题描述

RFC-010 设计与 RFC-011 泛型系统集成，支持 `[T: Constraint]` 约束语法。

**期望语法**：
```yaoxiang
# 泛型函数带约束
clone: [T: Clone](value: T) -> T = value.clone()

# 多重约束（暂不支持 & 语法）
# process: [T: Drawable & Serializable](item: T) -> String = { ... }

# 尖括号语法
identity: <T: Clone>(value: T) -> T = value
```

**当前状态**：✅ 已实现

### 3.2 需要的修改

#### 3.2.1 修改泛型参数解析

在 `src/frontend/core/parser/statements/declarations.rs` 中：

```rust
/// 泛型参数结构
pub struct GenericParam {
    pub name: String,
    pub constraints: Vec<MonoType>,  // 约束列表
}

/// Parse generic parameters: `[T, U]` or `[T: Clone, U: Serializable]`
pub fn parse_generic_params_with_constraints(
    state: &mut ParserState<'_>,
) -> Option<Vec<GenericParam>> {
    let open = if state.at(&TokenKind::LBracket) {
        state.bump();
        TokenKind::RBracket
    } else if state.at(&TokenKind::Lt) {
        state.bump();
        TokenKind::Gt
    } else {
        return Some(Vec::new());
    };

    let mut params = Vec::new();

    while !state.at(&open) && !state.at_end() {
        // 解析参数名
        let name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => break,
        };
        state.bump();

        // 解析约束
        let mut constraints = Vec::new();
        if state.skip(&TokenKind::Colon) {
            loop {
                let constraint = parse_type_annotation(state)?;
                constraints.push(constraint);

                if !state.skip(&TokenKind::Amp) {
                    break;
                }
            }
        }

        params.push(GenericParam { name, constraints });
        state.skip(&TokenKind::Comma);
    }

    if !state.expect(&open) {
        return None;
    }

    Some(params)
}
```

#### 3.2.2 修改类型定义和函数定义

在 `StmtKind::Fn` 中添加泛型参数：

```rust
/// Function definition with generic params
pub struct FnStmt {
    pub name: String,
    pub generic_params: Vec<GenericParam>,  // 新增
    pub type_annotation: Option<Type>,
    pub params: Vec<Param>,
    pub body: (Vec<Stmt>, Option<Box<Expr>>),
}
```

#### 3.2.3 在类型检查中实现约束验证

在 `src/frontend/typecheck/checking/bounds.rs` 中添加：

```rust
/// 检查泛型参数是否满足约束
pub fn check_generic_param_constraints(
    &self,
    param: &GenericParam,
    arg_type: &MonoType,
) -> Result<(), TypeError> {
    for constraint in &param.constraints {
        if !self.check_constraint(arg_type, constraint)? {
            return Err(TypeError::ConstraintNotSatisfied {
                param_name: param.name.clone(),
                constraint_name: constraint.type_name(),
                arg_type: arg_type.type_name(),
            });
        }
    }
    Ok(())
}
```

### 3.3 测试用例

```rust
#[test]
fn test_generic_constraint_parsing() {
    let code = r#"
        clone: [T: Clone](value: T) -> T = value.clone()
    "#;

    let ast = parse(code).unwrap();
    match &ast.items[0].kind {
        StmtKind::Fn { generic_params, .. } => {
            assert_eq!(generic_params.len(), 1);
            assert_eq!(generic_params[0].name, "T");
            assert_eq!(generic_params[0].constraints.len(), 1);
        }
        _ => panic!("Expected function definition"),
    }
}

#[test]
fn test_generic_constraint_checking() {
    let code = r#"
        type Point = { x: Float, y: Float }

        # Point 未实现 Clone，应该报错
        clone: [T: Clone](value: T) -> T = value.clone()
    "#;

    let result = typecheck(code);
    assert!(result.is_err());
}
```

---

## 4. 完整实现优先级

| 优先级 | 功能 | 影响范围 | 状态 |
|--------|------|----------|------|
| **P0** | 方法绑定语法解析 | 解析器 | 待实现 |
| **P1** | pub 自动绑定机制 | 解析器 + 类型检查 | 待实现 |
| **P2** | 泛型约束语法 | 解析器 + 类型检查 | ✅ 已完成 |

---

## 5. 相关文件清单

### 5.1 需要修改的文件

| 文件路径 | 修改内容 |
|----------|----------|
| `src/frontend/core/parser/ast.rs` | 添加 `GenericParam` 结构体、`StmtKind::Fn` 新增 `generic_params` 字段 |
| `src/frontend/core/parser/statements/declarations.rs` | 添加 `parse_generic_params_with_constraints`、修改 `parse_var_stmt`、扩展 `parse_type_annotation` |
| `src/frontend/typecheck/checking/mod.rs` | 添加 `generic_params` 字段匹配 |
| `src/frontend/typecheck/inference/statements.rs` | 添加 `generic_params` 字段匹配 |
| `src/frontend/typecheck/inference/expressions.rs` | 添加 `generic_params` 字段匹配 |
| `src/middle/core/ir_gen.rs` | 添加 `generic_params` 字段匹配 |

### 5.2 需要新增的文件

| 文件路径 | 说明 |
|----------|------|
| `src/frontend/core/parser/statements/method_bind.rs` | 方法绑定解析逻辑（待实现） |
| `src/frontend/typecheck/checking/auto_bind.rs` | 自动绑定检查逻辑（待实现） |

---

## 6. 验收标准

### 6.1 方法绑定
- [ ] 能解析 `Type.method: (Params) -> ReturnType = ...` 语法
- [ ] AST 正确生成 `MethodBind` 节点
- [ ] 类型检查能正确绑定方法到类型

### 6.2 pub 自动绑定
- [ ] `pub fn` 能被正确识别
- [ ] 能自动绑定到第一个参数的类型
- [ ] 支持 `p.method()` 语法糖调用

### 6.3 泛型约束
- [x] 能解析 `[T: Clone]` 语法
- [ ] 类型检查能验证约束是否满足（待实现）
- [ ] 错误信息清晰指出缺失的约束（待实现）
