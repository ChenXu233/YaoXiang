# YaoXiang 高级绑定特性与编译器实现

> 版本：v1.0.0
> 状态：草案
> 作者：晨煦
> 日期：2025-01-04

---

## 目录

1. [模式匹配与元组解构绑定](#一模式匹配与元组解构绑定)
2. [多返回值绑定](#二多返回值绑定)
3. [多位置联合绑定](#三多位置联合绑定)
4. [占位符与跳过参数](#四占位符与跳过参数)
5. [动态位置绑定](#五动态位置绑定)
6. [编译器实现细节](#六编译器实现细节)
7. [类型检查规则](#七类型检查规则)
8. [边缘情况处理](#八边缘情况处理)
9. [完整语法定义](#九完整语法定义)
10. [实际应用示例](#十实际应用示例)

---

## 一、模式匹配与元组解构绑定

### 1.1 模式解构绑定

YaoXiang 支持将函数绑定到接收元组参数的函数，自动进行解构：

```yaoxiang
# === 基础示例 ===

# 函数接收元组参数
process_coordinates((Float, Float)) -> String = (coord) => {
    match coord {
        (0.0, 0.0) -> "origin"
        (x, 0.0) -> "on x-axis at ${x}"
        (0.0, y) -> "on y-axis at ${y}"
        (x, y) -> "point at (${x}, ${y})"
    }
}

# 绑定到元组类型
type Coord = Coord(x: Float, y: Float)

# 自动解构绑定：Coord -> (Float, Float)
Coord.describe = process_coordinates[1]

# === 使用示例 ===

use Coord

c = Coord(3.0, 0.0)
result = c.describe()  # → process_coordinates((3.0, 0.0))
# 输出: "on x-axis at 3.0"
```

### 1.2 嵌套解构

```yaoxiang
# === 复杂示例 ===

# 嵌套元组函数
complex_match((Int, (String, Bool))) -> String = (data) => {
    (id, (name, flag)) = data
    "ID: ${id}, Name: ${name}, Flag: ${flag}"
}

# 类型定义
type Record = Record(id: Int, info: Info)
type Info = Info(name: String, flag: Bool)

# 绑定：Record -> (Int, (String, Bool))
Record.describe = complex_match[1]

# 使用
r = Record(42, Info("test", true))
r.describe()  # → complex_match((42, ("test", true)))
# 输出: "ID: 42, Name: test, Flag: true"
```

---

## 二、多返回值绑定

### 2.1 基础多返回值

```yaoxiang
# === 列表统计示例 ===

# 函数返回元组
min_max(List[Int]) -> (Int, Int) = (list) => {
    min = list.reduce(Int.MAX, (a, b) => if a < b then a else b)
    max = list.reduce(Int.MIN, (a, b) => if a > b then a else b)
    (min, max)
}

# 绑定到列表类型
List[Int].range = min_max[1]

# === 使用示例 ===

use List

numbers = [1, 5, 3, 8, 2]
(min_val, max_val) = numbers.range()  # → min_max([1, 5, 3, 8, 2])
# min_val = 1, max_val = 8
```

### 2.2 多返回值与解构

```yaoxiang
# === 统计函数示例 ===

# 返回多个统计值
stats(List[Float]) -> (mean: Float, std: Float, count: Int) = (list) => {
    sum = list.reduce(0.0, (a, b) => a + b)
    mean = sum / list.length
    variance = list.map(x => (x - mean) * (x - mean)).reduce(0.0, _ + _) / list.length
    (mean, variance.sqrt(), list.length)
}

# 绑定
List[Float].statistics = stats[1]

# 使用
data = [1.0, 2.0, 3.0, 4.0, 5.0]
(avg, std, n) = data.statistics()
# avg = 3.0, std ≈ 1.414, n = 5
```

---

## 三、多位置联合绑定

### 3.1 基础多位置绑定

```yaoxiang
# === 计算函数示例 ===

# 原函数
calculate(scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = (s, p1, p2, x, y) => {
    dx = (p1.x - p2.x) * s
    dy = (p1.y - p2.y) * s
    (dx * dx + dy * dy).sqrt() + x + y
}

# === Point 模块 ===

type Point = Point(x: Float, y: Float)

# 绑定策略
Point.calc1 = calculate[1, 2]    # 绑定 scale 和 point1
Point.calc2 = calculate[1, 3]    # 绑定 scale 和 point2
Point.calc3 = calculate[2, 3]    # 绑定 point1 和 point2
Point.calc4 = calculate[1, 2, 4] # 绑定 scale, point1, x

# === 使用示例 ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# 1. 绑定[1,2] - 剩余3,4,5
f1 = p1.calc1(2.0)  # 绑定 scale=2.0, point1=p1
# f1 需要: point2, x, y
result1 = f1(p2, 10.0, 20.0)  # → calculate(2.0, p1, p2, 10.0, 20.0)

# 2. 绑定[1,3] - 剩余2,4,5
f2 = p2.calc2(2.0)  # 绑定 scale=2.0, point2=p2
# f2 需要: point1, x, y
result2 = f2(p1, 10.0, 20.0)  # → calculate(2.0, p1, p2, 10.0, 20.0)

# 3. 绑定[2,3] - 剩余1,4,5
f3 = p1.calc3(p2)  # 绑定 point1=p1, point2=p2
# f3 需要: scale, x, y
result3 = f3(2.0, 10.0, 20.0)  # → calculate(2.0, p1, p2, 10.0, 20.0)

# 4. 绑定[1,2,4] - 剩余3,5
f4 = p1.calc4(2.0, 10.0)  # 绑定 scale=2.0, point1=p1, x=10.0
# f4 需要: point2, y
result4 = f4(p2, 20.0)  # → calculate(2.0, p1, p2, 10.0, 20.0)
```

### 3.2 剩余参数填入规则

**核心规则**：剩余参数按**原始函数参数顺序**填入，跳过已绑定位置。

```yaoxiang
# === 原函数参数顺序 ===
calculate(scale: Float, a: Point, b: Point, x: Float, y: Float)
# 参数位置: 0        1        2        3       4

# === 绑定示例 ===

# 绑定位置 [1, 3]
Point.method = calculate[1, 3]
# 已绑定: 位置1 (a), 位置3 (x)
# 剩余位置: 0 (scale), 2 (b), 4 (y)

# 调用时
method(scale_val, b_val, y_val)
# 填入: scale_val -> 位置0, b_val -> 位置2, y_val -> 位置4

# 最终: calculate(scale_val, p1, b_val, x_bound, y_val)
# 其中 p1 是绑定的调用者，x_bound 是绑定的值
```

### 3.3 参数顺序可视化

```yaoxiang
# === 可视化说明 ===

func: (p0, p1, p2, p3, p4) -> Result

# 绑定 [1, 3]
Type.method = func[1, 3]

# 调用: obj.method(p0_val, p2_val, p4_val)
# 映射过程:
#
# 原始参数: [p0, p1, p2, p3, p4]
# 绑定:     [  -, obj,  -,  -,  -]  (位置1, 3是obj)
# 用户输入: [p0,  -, p2,  -, p4]    (剩余位置按顺序)
# 结果:     [p0, obj, p2, obj, p4]  (合并)
#
# 注意：位置3也被obj绑定，显示为obj
# 但如果位置3需要不同的绑定值，需要使用占位符
```

---

## 四、占位符与跳过参数

### 4.1 基础占位符

```yaoxiang
# === 原函数 ===
func(Int, String, Bool, Float) -> String = (a, b, c, d) => {
    "${a}: ${b} (${c}) -> ${d}"
}

# === 绑定策略 ===

# 只绑定第1和第4参数
Type.partial = func[1, _, _, 4]
# 或者使用命名占位符
Type.partial2 = func[1, @skip, @skip, 4]

# === 使用示例 ===

# 调用时，第2和第3参数由用户提供
obj.partial("hello", true)  # → func(obj, "hello", true, _)
# 需要继续填充第4参数

# 或者一次性提供所有剩余参数
obj.partial("hello", true, 3.14)  # → func(obj, "hello", true, 3.14)
```

### 4.2 占位符在多位置绑定中的使用

```yaoxiang
# === 复杂函数 ===
complex(
    scale: Float,
    data: (Int, String),
    flag: Bool,
    extra: Float
) -> String = (s, (id, name), f, e) => {
    "${name} (${id}): scale=${s}, flag=${f}, extra=${e}"
}

# === 类型定义 ===
type Info = Info(id: Int, name: String)

# === 绑定策略 ===

# 绑定第1和第2参数，但第2参数是元组，需要解构
# 情况1：绑定整个元组
Info.method1 = complex[1, 2]
# 使用：info.method1(2.0, true, 3.14) → complex(2.0, (info.id, info.name), true, 3.14)

# 情况2：使用占位符跳过某些位置
Info.method2 = complex[_, 2, _, 4]
# 使用：info.method2(2.0, true) → complex(2.0, (info.id, info.name), true, _)
```

### 4.3 批量跳过

```yaoxiang
# === 场景：数据库查询 ===

# 函数参数很多
query(
    db: DBConnection,
    sql: String,
    params: (String, Any),
    limit: Int,
    offset: Int,
    timeout: Duration
) -> List[Record] = ...

# 绑定：只关心 db 和 sql
DB.query = query[1, 2]
# 使用：db.query("SELECT * FROM users") 
# → query(db, "SELECT * FROM users", _, _, _, _)
# 需要继续提供 params, limit, offset, timeout
```

---

## 五、动态位置绑定

### 5.1 编译时计算

```yaoxiang
# === 配置驱动绑定 ===

const BINDING_MODE = 1  # 或 2

# 根据配置选择绑定方式
if BINDING_MODE == 1 {
    Point.distance = distance[1]  # 绑定到第1参数
} else {
    Point.distance = distance[0]  # 绑定到第0参数（如果函数定义不同）
}

# === 使用 ===

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)
d = p1.distance(p2)  # 根据配置，自动选择正确的绑定方式
```

### 5.2 条件绑定

```yaoxiang
# === 特性开关 ===

const FEATURE_ENABLED = true

# 根据特性选择绑定
if FEATURE_ENABLED {
    # 新版本：更多参数
    DB.query = query_new[1, 2, 3]
} else {
    # 旧版本：较少参数
    DB.query = query_old[1, 2]
}

# === 使用 ===

if FEATURE_ENABLED {
    result = db.query(sql, params, timeout)  # 新版本
} else {
    result = db.query(sql, params)           # 旧版本
}
```

### 5.3 泛型绑定

```yaoxiang
# === 泛型函数绑定 ===

# 泛型函数
compare<T: Ord>(T, T) -> Ordering = (a, b) => {
    if a < b { less } else if a > b { greater } else { equal }
}

# 泛型类型绑定
List[T].compare = compare[1]

# === 使用 ===

use List

list1 = [1, 2, 3]
list2 = [1, 2, 4]

# 类型检查确保 T: Ord
list1.compare(list2)  # → compare(list1, list2)
```

---

## 六、编译器实现细节

### 6.1 绑定解析算法

```rust
// 伪代码：解析绑定声明
fn parse_binding(expr: Expr) -> Result<Binding, ParseError> {
    // 语法: Type.method = function[positions]
    
    // 解析左侧
    let (type_name, method_name) = match &expr.left {
        Expr::Binary {
            left: Expr::Identifier(type_name),
            op: Token::Dot,
            right: Expr::Identifier(method_name),
        } => (type_name, method_name),
        _ => return Err("Invalid left side"),
    };
    
    // 解析右侧
    let (function_name, positions) = match &expr.right {
        Expr::Binary {
            left: Expr::Identifier(func_name),
            op: Token::Equal,
            right: Expr::Index {
                base: Expr::Identifier(target_func),
                index,
            },
        } => {
            let positions = parse_positions(index)?;
            (target_func, positions)
        }
        _ => return Err("Invalid right side"),
    };
    
    Ok(Binding {
        type_name: type_name.clone(),
        method_name: method_name.clone(),
        function_name: function_name.clone(),
        positions,
    })
}

fn parse_positions(index: &Expr) -> Result<Vec<Position>, ParseError> {
    // 支持多种格式:
    // [1] -> vec![1]
    // [1, 2, 3] -> vec![1, 2, 3]
    // [1, _, 3] -> vec![1, Placeholder, 3]
    // [1..3] -> vec![1, 2]  (范围)
    
    match index {
        Expr::Integer(n) => Ok(vec![*n]),
        Expr::Tuple(elems) => {
            elems.iter().map(|e| match e {
                Expr::Integer(n) => Ok(Position::Exact(*n)),
                Expr::Identifier(name) if name == "_" => Ok(Position::Placeholder),
                Expr::Range(start, end) => {
                    Ok(Position::Range(*start, *end))
                }
                _ => Err("Invalid position format"),
            }).collect()
        }
        _ => Err("Invalid index expression"),
    }
}
```

### 6.2 调用代码生成

```rust
// 伪代码：生成方法调用代码
fn generate_method_call(
    &mut self,
    obj_type: &Type,
    method_name: &str,
    args: &[Expr],
) -> Expr {
    // 查找绑定定义
    let binding = self.lookup_binding(obj_type, method_name)
        .unwrap_or_else(|| {
            // 尝试自动绑定
            self.generate_auto_binding(obj_type, method_name)
        });
    
    // 原函数签名
    let func_type = self.lookup_function_type(&binding.function_name);
    let param_count = func_type.param_types.len();
    
    // 构建参数数组
    let mut final_args = vec![None; param_count];
    
    // 1. 填入绑定的调用者到指定位置
    for &pos in &binding.positions {
        if pos < param_count {
            final_args[pos] = Some(Expr::This(obj_type.clone()));
        } else {
            self.error("Binding position out of bounds");
        }
    }
    
    // 2. 填入用户提供的参数到剩余位置
    let mut arg_idx = 0;
    for pos in 0..param_count {
        if final_args[pos].is_none() {
            if arg_idx < args.len() {
                final_args[pos] = Some(args[arg_idx].clone());
                arg_idx += 1;
            } else {
                // 参数不足，需要柯里化
                return self.generate_curried_call(
                    binding.function_name,
                    final_args,
                    arg_idx,
                );
            }
        }
    }
    
    // 3. 生成函数调用
    Expr::Call(
        Box::new(Expr::Identifier(binding.function_name)),
        final_args.into_iter().map(|a| a.unwrap()).collect(),
    )
}
```

### 6.3 柯里化处理

```rust
// 伪代码：生成柯里化调用
fn generate_curried_call(
    &mut self,
    func_name: String,
    partial_args: Vec<Option<Expr>>,
    start_from: usize,
) -> Expr {
    // 生成一个闭包，接收剩余参数
    let remaining_params: Vec<_> = partial_args.iter()
        .enumerate()
        .filter(|(_, arg)| arg.is_none())
        .map(|(idx, _)| {
            let param_name = format!("arg{}", idx);
            (param_name, self.get_param_type(func_name, idx))
        })
        .collect();
    
    // 闭包体
    let closure_body = Expr::Call(
        Box::new(Expr::Identifier(func_name)),
        partial_args.into_iter()
            .map(|a| a.unwrap_or_else(|| {
                // 使用参数名
                Expr::Identifier(remaining_params[0].0.clone())
            }))
            .collect(),
    );
    
    Expr::Lambda {
        params: remaining_params,
        body: Box::new(closure_body),
    }
}
```

---

## 七、类型检查规则

### 7.1 绑定声明检查

```yaoxiang
# === 类型检查伪代码 ===

检查绑定 Type.method = func[positions]：

1. 验证 positions 中的每个 index < func 的参数个数
   - 示例: func[5] 绑定到 3 参数函数 → 错误

2. 对于每个 position = i:
   - 获取 func 第 i 个参数的类型 T_i
   - 验证 Type 可以赋值给 T_i（类型兼容）
   - 示例: Point.distance = compare[1]
     # compare(T, T) -> Ordering
     # 位置1参数类型: T
     # 绑定类型: Point
     # 验证: Point 是否满足 T 的约束

3. 检查剩余参数（未绑定的）与 method 调用的参数匹配
   - 计算剩余参数数量
   - 验证调用时提供的参数数量 >= 剩余参数
   - 如果不足，必须支持柯里化

4. 检查返回类型一致性
   - func 的返回类型应与 method 的返回类型匹配
   - 或支持隐式转换
```

### 7.2 调用时类型检查

```yaoxiang
# === 调用检查伪代码 ===

检查 obj.method(arg1, arg2, ...)

1. 查找绑定定义
   - 如果是自动绑定，检查函数是否 pub
   - 如果是显式绑定，查表获取绑定信息

2. 根据绑定位置计算参数映射
   - 已绑定位置: obj 填入
   - 剩余位置: 由 args 按顺序填入

3. 类型匹配检查
   - 每个位置填入的值类型必须匹配对应参数类型
   - 自动绑定: 检查 arg1 类型是否匹配 func 的第2个参数

4. 柯里化检查
   - 如果 args 数量 < 剩余参数数量
   - 返回一个函数类型，接收剩余参数
   - 示例: obj.method(1, 2) 剩余3参数 → (T3, T4, T5) -> Result
```

### 7.3 类型约束检查

```yaoxiang
# === 泛型绑定检查 ===

# 泛型函数
compare<T: Ord>(T, T) -> Ordering = ...

# 泛型绑定
List[T].compare = compare[1]

# 类型检查：
1. 从绑定声明推断 T = List[T]
2. 检查 List[T]: Ord
3. 需要 List 实现 Ord trait
4. 需要 T 实现 Ord trait

# 使用时
list1.compare(list2)
# 检查 list1 和 list2 类型相同
# 检查元素类型 T 满足 Ord 约束
```

---

## 八、边缘情况处理

### 8.1 参数不足

```yaoxiang
# === 场景 1：绑定部分参数 ===

func(Int, String, Float) -> String = (a, b, c) => "${a}: ${b} -> ${c}"

# 绑定1个参数
Type.method = func[1]
# 使用：obj.method("hello", 3.14) → func(obj, "hello", 3.14)

# 绑定2个参数
Type.method2 = func[1, 2]
# 使用：obj.method2(3.14) → func(obj, "hello", 3.14)
# 等待用户补充第2参数

# === 场景 2：需要柯里化 ===

Type.method3 = func[1]
# 使用：obj.method3() → 返回函数 (String, Float) -> String
# 然后：obj.method3()("hello", 3.14) → func(obj, "hello", 3.14)
```

### 8.2 重复绑定位置

```yaoxiang
# === 允许重复绑定（如果类型兼容）===

func(Point, Point, Point) -> Bool = (p1, p2, p3) => {
    p1 == p2 && p2 == p3
}

# 绑定到所有位置
Point.same_all = func[0, 1, 2]
# 使用：p.same_all() → func(p, p, p)

# === 类型检查 ===

# 如果函数参数类型不同
func(Point, Int, String) -> Bool = ...

# 绑定
Point.method = func[0, 1, 2]
# 使用：p.method(42, "test") → func(p, 42, "test")
# 这里只绑定了位置0，位置1和2由用户提供
```

### 8.3 泛型函数绑定

```yaoxiang
# === 泛型函数 ===

identity<T>(T) -> T = (x) => x

# 绑定泛型类型
List[T].identity = identity[1]
# 类型检查：
# 1. T 是泛型参数
# 2. 绑定到 List[T]
# 3. 需要 List[T] 满足 T 的约束
# 4. 如果没有约束，则自动满足

# 使用
list = [1, 2, 3]
result = list.identity()  # → identity(list)
# 类型: List[Int] -> List[Int]
```

### 8.4 默认参数

```yaoxiang
# === 带默认参数的函数 ===

greet(name: String, greeting: String = "Hello") -> String = (n, g) => {
    "${g}, ${n}!"
}

# 绑定
Person.greet = greet[1]  # 绑定 name

# 使用
person = Person("Alice")
person.greet()           # → greet("Alice", "Hello")  # 使用默认值
person.greet("Hi")       # → greet("Alice", "Hi")     # 覆盖默认值
```

### 8.5 重载与歧义

```yaoxiang
# === 潜在歧义场景 ===

# 函数1
distance(Point, Point) -> Float = ...

# 函数2  
distance(Point, Float) -> Float = (p, scale) => { ... }

# 如果绑定
Point.distance = distance[1]

# 问题：哪个 distance？
# 解决方案：
# 1. 要求函数名唯一（不支持重载）
# 2. 或使用全限定名
Point.distance = distance_point_point[1]
```

---

## 九、完整语法定义

### 9.1 BNF 语法

```
绑定声明 ::= 类型 '.' 标识符 '=' 函数名 '[' 位置列表 ']'

类型 ::= 标识符
       | 标识符 '<' 泛型参数列表 '>'

函数名 ::= 标识符

位置列表 ::= 位置 (',' 位置)*
           | 空

位置 ::= 整数                      # 具体位置
       | '_'                       # 占位符
       | 标识符                    # 命名位置（未来扩展）
       | 整数 '..' 整数           # 范围（未来扩展）
       | '@' 标识符               # 命名占位符

泛型参数列表 ::= 泛型参数 (',' 泛型参数)*
泛型参数 ::= 标识符 [':' trait_bound]

整数 ::= 0 | 1 | 2 | ...
标识符 ::= [a-zA-Z_][a-zA-Z0-9_]*
```

### 9.2 语义规则

**规则 1：位置范围**
- 位置从 0 开始计数
- 最大位置必须小于函数参数个数
- 负数位置无效

**规则 2：绑定类型兼容**
- 绑定的类型必须是函数参数类型的子类型
- 对于泛型函数，需要满足类型约束

**规则 3：参数完整性**
- 绑定后，剩余参数数量 <= 2（否则推荐使用部分应用）
- 支持任意长度的柯里化链

**规则 4：返回类型**
- 方法的返回类型 = 函数的返回类型
- 不进行隐式类型转换

### 9.3 支持的格式示例

```yaoxiang
# ✅ 合法格式
Point.distance = distance[1]
Point.calc = calculate[1, 2, 3]
Point.partial = func[1, _, 3]
Point.range = func[1..4]

# ❌ 非法格式
Point.wrong = func[1, 2, 3, 4, 5]  # 位置太多（超过阈值）
Point.wrong = func[-1]            # 负数位置
Point.wrong = func[0, 0]          # 重复位置（如果不允许）
Point.wrong = func[1.5]           # 非整数位置
```

---

## 十、实际应用示例

### 10.1 数据库查询构建器

```yaoxiang
# === 数据库库 ===

# 原始查询函数
query(
    conn: Connection,
    sql: String,
    params: List[Any],
    limit: Int,
    offset: Int,
    timeout: Duration
) -> List[Record] = (conn, sql, params, limit, offset, timeout) => {
    # 执行查询
    # ...
}

# === 绑定到连接类型 ===

type Connection = Connection(url: String)

# 简单查询（绑定连接和SQL）
Connection.simple_query = query[1, 2]

# 带参数查询（绑定连接、SQL、参数）
Connection.param_query = query[1, 2, 3]

# 完整查询（绑定所有）
Connection.full_query = query[1, 2, 3, 4, 5]

# === 使用示例 ===

use Connection

db = Connection("postgres://localhost/db")

# 简单查询
users = db.simple_query("SELECT * FROM users")

# 带参数查询
user = db.param_query("SELECT * FROM users WHERE id = $1", [1])

# 完整查询
data = db.full_query(
    "SELECT * FROM users",
    [],
    10,     # limit
    0,      # offset
    5000ms  # timeout
)

# 柯里化：预设部分参数
quick_query = db.param_query("SELECT * FROM users WHERE active = $1")
active_users = quick_query([true])
```

### 10.2 管道处理

```yaoxiang
# === 数据处理管道 ===

# 处理函数
process(
    data: List[Int],
    transform: (Int) -> Int,
    filter: (Int) -> Bool,
    aggregator: (List[Int]) -> Int
) -> Int = (data, transform, filter, aggregator) => {
    data.map(transform).filter(filter).aggregator()
}

# === 绑定策略 ===

type Pipeline = Pipeline(data: List[Int])

# 绑定数据
Pipeline.process = process[1]

# === 使用示例 ===

use Pipeline

pipe = Pipeline([1, 2, 3, 4, 5])

# 链式处理
result = pipe.process(
    x => x * 2,          # transform
    x => x > 5,          # filter
    list => list.sum()   # aggregator
)
# → process([1,2,3,4,5], x=>x*2, x=>x>5, sum)
# 结果: (2,4,6,8,10).filter(>5).sum() = 14

# 预设变换函数
double_then_sum = pipe.process(_, _, sum)
# 使用时：double_then_sum(x => x * 2, x => x > 5)
```

### 10.3 配置验证

```yaoxiang
# === 配置系统 ===

# 验证函数
validate(
    config: Config,
    rules: List[Rule>,
    strict: Bool,
    error_handler: (Error) -> Result
) -> ValidationResult = (config, rules, strict, error_handler) => {
    # 验证逻辑
}

# === 类型定义 ===

type Config = Config(values: Dict<String, Any>)
type Rule = Rule(field: String, validator: (Any) -> Bool)

# === 绑定 ===

Config.validate = validate[1]

# === 使用 ===

config = Config(...)

# 标准验证
result = config.validate(rules, true, handle_error)

# 快速验证（使用默认错误处理）
quick_validate = config.validate(rules, false, default_handler)
result = quick_validate()
```

### 10.4 数学计算库

```yaoxiang
# === 数学库 ===

# 多参数函数
solve(
    a: Float,
    b: Float,
    c: Float,
    method: (Float, Float, Float) -> Float
) -> Float = (a, b, c, method) => {
    method(a, b, c)
}

# === 绑定策略 ===

type Quadratic = Quadratic(a: Float, b: Float, c: Float)

# 绑定系数
Quadratic.solve = solve[1, 2, 3]

# === 使用 ===

# 求解 x^2 + 2x + 1 = 0
eq = Quadratic(1.0, 2.0, 1.0)

# 使用默认方法
result1 = eq.solve(quadratic_formula)

# 或预设方法
solve_eq = eq.solve(quadratic_formula)
result2 = solve_eq()

# 柯里化：先绑定系数，再选择方法
partial_eq = solve(1.0, 2.0, 1.0, _)
result3 = partial_eq(quadratic_formula)
```

---

## 总结

YaoXiang 的高级绑定特性提供了：

1. **精准控制**：可以绑定到任意参数位置
2. **模式匹配友好**：支持元组解构绑定
3. **灵活柯里化**：支持多位置联合绑定和占位符
4. **类型安全**：编译时验证所有绑定
5. **零运行时开销**：所有绑定在编译时解决
6. **纯函数式**：保持函数式编程的纯粹性

这些特性让 YaoXiang 的绑定系统既强大又优雅，完美解决了柯里化绑定的所有挑战！
