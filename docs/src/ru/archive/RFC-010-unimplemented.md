# RFC-010 Унифицированный синтаксис типов — документация функций, ожидающих реализации

> **Дата создания**: 2026-02-03
> **Статус**: Ожидает реализации
> **На основе RFC**: RFC-010 Унифицированный синтаксис типов

## Обзор

В данном документе описаны части дизайна унифицированного синтаксиса типов из RFC-010, которые ещё не реализованы или реализованы не полностью. Документ служит справочным руководством для дальнейшей разработки.

---

## 1. Синтаксический анализ привязки методов

### 1.1 Описание проблемы

В RFC-010 разработан синтаксис определения методов `Type.method: (Type, ...) -> ReturnType = ...`, однако парсер в настоящее время не поддерживает этот синтаксис.

**Ожидаемый синтаксис**:
```yaoxiang
# Определение метода типа
Point.draw: (Point, Surface) -> Void = (self, surface) => {
    surface.plot(self.x, self.y)
}

Point.serialize: (Point) -> String = (self) => {
    "Point(${self.x}, ${self.y})"
}
```

**Текущее состояние**:
- Узел `MethodBind` определён в AST (`src/frontend/core/parser/ast.rs:184-195`)
- В парсере `declarations.rs` отсутствует соответствующая логика синтаксического анализа

### 1.2 Необходимые изменения

#### 1.2.1 Изменение `parse_type_annotation` или добавление новой функции разбора

В `src/frontend/core/parser/statements/declarations.rs` добавить распознавание синтаксиса привязки методов:

```rust
/// Проверка, является ли синтаксис привязкой метода: `Type.method: (Params) -> ReturnType`
fn is_method_bind_syntax(state: &mut ParserState<'_>) -> bool {
    let saved = state.save_position();

    // Проверка наличия разделённого точкой имени типа и имени метода
    // Например: Point.draw: (Point, Surface) -> Void = ...
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

#### 1.2.2 Добавление функции разбора привязки методов

```rust
/// Разбор привязки метода: `Type.method: (Params) -> ReturnType = (params) => body`
pub fn parse_method_bind_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    // Разбор имени типа
    let type_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => return None,
    };
    state.bump();

    // Потребление точки
    state.expect(&TokenKind::Dot)?;

    // Разбор имени метода
    let method_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => return None,
    };
    state.bump();

    // Потребление двоеточия
    state.expect(&TokenKind::Colon)?;

    // Разбор типа метода
    let method_type = parse_type_annotation(state)?;

    // Потребление знака равенства
    state.expect(&TokenKind::Eq)?;

    // Разбор тела метода
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

### 1.3 Тестовые случаи

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

## 2. Механизм автоматической привязки pub

### 2.1 Описание проблемы

В RFC-010 разработан механизм автоматической привязки `pub`: когда функция объявляется с `pub`, компилятор должен автоматически привязать её к типу, определённому в том же файле.

**Ожидаемое поведение**:
```yaoxiang
# Объявление с pub, компилятор автоматически привязывает к Point.distance
pub distance: (Point, Point) -> Float = (p1, p2) => {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# Эквивалентно:
Point.distance = distance[0]

# Способы вызова
d1 = distance(p1, p2)      # Функциональный
d2 = p1.distance(p2)       # ООП-синтаксис
```

**Текущее состояние**: Соответствующая реализация отсутствует

### 2.2 Необходимые изменения

#### 2.2.1 Изменение парсера для распознавания функций pub

В функции `parse_identifier_stmt` в `src/frontend/core/parser/statements/declarations.rs`:

```rust
/// Разбор оператора, начинающегося с идентификатора
pub fn parse_identifier_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    // Проверка, является ли это объявлением pub
    let is_pub = state.skip(&TokenKind::KwPub);

    // Продолжение логики...

    // При возврате пометить статус pub
    Some(Stmt {
        kind: StmtKind::Fn {
            name,
            type_annotation,
            params,
            body,
            is_pub,  // Новое поле
        },
        span,
    })
}
```

#### 2.2.2 Добавление новых полей в AST

Изменить `StmtKind::Fn` в `src/frontend/core/parser/ast.rs`:

```rust
/// Определение функции: `name: Type = (params) => body`
pub struct FnStmt {
    pub name: String,
    pub type_annotation: Option<Type>,
    pub params: Vec<Param>,
    pub body: (Vec<Stmt>, Option<Box<Expr>>),
    pub is_pub: bool,  // Новое: автоматическая привязка к типу
    pub auto_bind_type: Option<String>,  // Новое: целевой тип для автоматической привязки
}
```

#### 2.2.3 Реализация автоматической привязки на этапе проверки типов

В `src/frontend/typecheck/inference/statements.rs`:

```rust
/// Обработка определения функции с автоматической привязкой pub
fn infer_fn_stmt(
    &mut self,
    stmt: &Stmt,
    env: &mut TypeEnvironment,
) -> TypeResult<MonoType> {
    match &stmt.kind {
        StmtKind::Fn { name, params, return_type, body, is_pub, .. } => {
            // Построение типа функции
            let fn_type = self.infer_fn_type(params, return_type.as_ref())?;

            if *is_pub {
                // Попытка автоматической привязки к типу, определённому в том же файле
                if let Some(target_type) = self.find_target_type_for_pub(name, params) {
                    self.bind_method_to_type(&target_type, name, &fn_type)?;
                }
            }

            // Регистрация в окружении
            env.add_var(name.clone(), PolyType::mono(fn_type));

            Ok(MonoType::Void)
        }
        _ => unreachable!(),
    }
}

/// Поиск целевого типа для привязки функции pub
fn find_target_type_for_pub(
    &self,
    fn_name: &str,
    params: &[Param],
) -> Option<String> {
    // Правило: тип первого параметра используется как цель привязки
    // Например: distance: (Point, Point) -> Float привязывается к Point
    if let Some(first_param) = params.first() {
        if let Some(ref ty) = first_param.ty {
            return Some(self.type_to_string(ty));
        }
    }
    None
}
```

### 2.3 Тестовые случаи

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

    // Проверка, что метод Point.distance привязан
    let point_type = type_env.get_type("Point").unwrap();
    assert!(point_type.methods.contains_key("distance"));
}
```

---

## 3. Синтаксический анализ ограничений обобщённых типов

### 3.1 Описание проблемы

RFC-010 интегрирован с обобщённой системой из RFC-011, поддерживая синтаксис ограничений `[T: Constraint]`.

**Ожидаемый синтаксис**:
```yaoxiang
# Обобщённая функция с ограничением
clone: [T: Clone](value: T) -> T = value.clone()

# Множественные ограничения (пока не поддерживается синтаксис &)
# process: [T: Drawable & Serializable](item: T) -> String = { ... }

# Синтаксис с угловыми скобками
identity: <T: Clone>(value: T) -> T = value
```

**Текущее состояние**: ✅ Реализовано

### 3.2 Необходимые изменения

#### 3.2.1 Изменение разбора обобщённых параметров

В `src/frontend/core/parser/statements/declarations.rs`:

```rust
/// Структура обобщённого параметра
pub struct GenericParam {
    pub name: String,
    pub constraints: Vec<MonoType>,  // Список ограничений
}

/// Разбор обобщённых параметров: `[T, U]` или `[T: Clone, U: Serializable]`
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
        // Разбор имени параметра
        let name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => break,
        };
        state.bump();

        // Разбор ограничений
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

#### 3.2.2 Изменение определения типа и функции

Добавить обобщённые параметры в `StmtKind::Fn`:

```rust
/// Определение функции с обобщёнными параметрами
pub struct FnStmt {
    pub name: String,
    pub generic_params: Vec<GenericParam>,  // Новое
    pub type_annotation: Option<Type>,
    pub params: Vec<Param>,
    pub body: (Vec<Stmt>, Option<Box<Expr>>),
}
```

#### 3.2.3 Реализация проверки ограничений при проверке типов

В `src/frontend/typecheck/checking/bounds.rs` добавить:

```rust
/// Проверка соответствия обобщённого параметра ограничениям
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

### 3.3 Тестовые случаи

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

        # Point не реализует Clone, должна возникнуть ошибка
        clone: [T: Clone](value: T) -> T = value.clone()
    "#;

    let result = typecheck(code);
    assert!(result.is_err());
}
```

---

## 4. Приоритеты полной реализации

| Приоритет | Функция | Область влияния | Статус |
|-----------|---------|-----------------|--------|
| **P0** | Синтаксический анализ привязки методов | Парсер | Ожидает реализации |
| **P1** | Механизм автоматической привязки pub | Парсер + проверка типов | Ожидает реализации |
| **P2** | Синтаксис ограничений обобщённых типов | Парсер + проверка типов | ✅ Выполнено |

---

## 5. Список связанных файлов

### 5.1 Файлы, требующие изменения

| Путь к файлу | Содержание изменений |
|--------------|----------------------|
| `src/frontend/core/parser/ast.rs` | Добавление структуры `GenericParam`, нового поля `generic_params` в `StmtKind::Fn` |
| `src/frontend/core/parser/statements/declarations.rs` | Добавление `parse_generic_params_with_constraints`, изменение `parse_var_stmt`, расширение `parse_type_annotation` |
| `src/frontend/typecheck/checking/mod.rs` | Добавление соответствия полю `generic_params` |
| `src/frontend/typecheck/inference/statements.rs` | Добавление соответствия полю `generic_params` |
| `src/frontend/typecheck/inference/expressions.rs` | Добавление соответствия полю `generic_params` |
| `src/middle/core/ir_gen.rs` | Добавление соответствия полю `generic_params` |

### 5.2 Файлы, требующие создания

| Путь к файлу | Описание |
|--------------|----------|
| `src/frontend/core/parser/statements/method_bind.rs` | Логика разбора привязки методов (ожидает реализации) |
| `src/frontend/typecheck/checking/auto_bind.rs` | Логика проверки автоматической привязки (ожидает реализации) |

---

## 6. Критерии приёмки

### 6.1 Привязка методов
- [ ] Возможность разбора синтаксиса `Type.method: (Params) -> ReturnType = ...`
- [ ] Корректная генерация узла `MethodBind` в AST
- [ ] Проверка типов корректно привязывает методы к типам

### 6.2 Автоматическая привязка pub
- [ ] Функции `pub fn` корректно распознаются
- [ ] Автоматическая привязка к типу первого параметра
- [ ] Поддержка вызова через синтаксис `p.method()`

### 6.3 Ограничения обобщённых типов
- [x] Возможность разбора синтаксиса `[T: Clone]`
- [ ] Проверка типов проверяет выполнение ограничений (ожидает реализации)
- [ ] Сообщения об ошибках чётко указывают на недостающие ограничения (ожидает реализации)