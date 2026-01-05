# Phase 19: AOT 编译器

> **模块路径**: `src/aot/`
> **状态**: ⏳ 待实现
> **依赖**: P4（CodeGen）+ P15（JIT）

## 概述

AOT（Ahead-Of-Time）编译器将 YaoXiang 代码编译为独立可执行文件，无需运行时即可运行。

## 文件结构

```
phase-19-aot/
├── README.md                    # 本文档
├── task-19-01-emit-native.md    # 原生代码生成
├── task-19-02-linker.md         # 链接器
├── task-19-03-runtime-bundle.md # 运行时打包
└── task-19-04-optimization.md   # 编译优化
```

## 完成状态

| Task | 名称 | 状态 | 依赖 |
|------|------|------|------|
| task-19-01 | 原生代码生成 | ⏳ 待实现 | task-04-04 |
| task-19-02 | 链接器 | ⏳ 待实现 | task-19-01 |
| task-19-03 | 运行时打包 | ⏳ 待实现 | task-19-02 |
| task-19-04 | 编译优化 | ⏳ 待实现 | task-15-02 |

## AOT vs JIT

| 特性 | AOT | JIT |
|------|-----|-----|
| 编译时机 | 运行前 | 运行时 |
| 启动速度 | 快 | 慢 |
| 峰值性能 | 中等 | 高 |
| 可执行文件 | 独立 | 需要运行时 |
| 平台适配 | 编译时确定 | 运行时优化 |
| 二进制大小 | 较大 | 较小（按需） |

## 编译流程

```
┌─────────────────────────────────────────────────────────┐
│                    AOT 编译流程                          │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  source.yx                                               │
│      │                                                   │
│      ▼                                                   │
│  ┌─────────┐                                             │
│  │ Lexer   │  ──►  Token Stream                          │
│  └─────────┘                                             │
│      │                                                   │
│      ▼                                                   │
│  ┌─────────┐                                             │
│  │ Parser  │  ──►  AST                                   │
│  └─────────┘                                             │
│      │                                                   │
│      ▼                                                   │
│  ┌─────────┐                                             │
│  │TypeCheck│  ──►  Typed AST                             │
│  └─────────┘                                             │
│      │                                                   │
│      ▼                                                   │
│  ┌─────────┐                                             │
│  │CodeGen  │  ──►  IR (LLVM/CRanel)                      │
│  └─────────┘                                             │
│      │                                                   │
│      ▼                                                   │
│  ┌─────────┐                                             │
│  │Optimize │  ──►  Optimized IR                          │
│  └─────────┘                                             │
│      │                                                   │
│      ▼                                                   │
│  ┌─────────┐                                             │
│  │Emit     │  ──►  Assembly/Machine Code                 │
│  └─────────┘                                             │
│      │                                                   │
│      ▼                                                   │
│  ┌─────────┐                                             │
│  │Linker   │  ──►  Executable (ELF/PE/Mach-O)            │
│  └─────────┘                                             │
│      │                                                   │
│      ▼                                                   │
│  executable                                               │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## 目标平台

### Tier 1（一级支持）

| 平台 | 架构 | 状态 |
|------|------|------|
| Linux | x86_64 | ⏳ 待实现 |
| macOS | x86_64/aarch64 | ⏳ 待实现 |
| Windows | x86_64 | ⏳ 待实现 |

### Tier 2（二级支持）

| 平台 | 架构 | 状态 |
|------|------|------|
| Linux | aarch64 | ⏳ 待实现 |
| Windows | i686 | ⏳ 待实现 |
| WASM | - | ⏳ 待实现 |

## 代码生成

```rust
/// AOT 代码生成器
struct AotCompiler {
    target: Target,
    codegen: CodeGenerator,
    optimizer: Optimizer,
    linker: Linker,
}

impl AotCompiler {
    /// 编译为可执行文件
    fn compile(&self, input: &Path, output: &Path) -> Result<(), Error> {
        // 1. 解析和类型检查
        let ast = self.parse_and_check(input)?;

        // 2. 生成 IR
        let ir = self.codegen.generate(&ast)?;

        // 3. 优化 IR
        let optimized_ir = self.optimizer.optimize(ir)?;

        // 4. 生成目标代码
        let object = self.codegen.emit(&optimized_ir)?;

        // 5. 链接
        self.linker.link(&object, output)?;

        Ok(())
    }
}
```

## 运行时打包

```rust
/// 运行时打包器
struct RuntimeBundler {
    runtime_path: PathBuf,
    stripped: bool,
}

impl RuntimeBundler {
    /// 打包运行时（嵌入式或动态链接）
    fn bundle(&self, exec_path: &Path, bundle_path: &Path) -> Result<(), Error> {
        if self.stripped {
            // 嵌入式运行时（最小依赖）
            self.bundle_embedded(exec_path, bundle_path)
        } else {
            // 动态链接
            self.bundle_dynamic(exec_path, bundle_path)
        }
    }
}
```

## 使用示例

```bash
# 基本编译
yaoxiangc compile input.yx -o output

# 指定目标平台
yaoxiangc compile input.yx --target x86_64-unknown-linux-gnu -o output

# 带调试信息
yaoxiangc compile input.yx -g -o output

# 优化级别
yaoxiangc compile input.yx -O3 -o output

# 静态链接（无动态依赖）
yaoxiangc compile input.yx --static -o output

# 打包运行时
yaoxiangc compile input.yx --bundle-runtime -o output
```

## 输出格式

### Linux (ELF)

```
output: ELF 64-bit LSB executable, x86-64
```

### macOS (Mach-O)

```
output: Mach-O 64-bit executable x86-64
```

### Windows (PE)

```
output: PE32+ executable (GUI) x86-64
```

## 相关文件

- **mod.rs**: AOT 编译器主模块
- **emitter.rs**: 代码发射器
- **linker.rs**: 链接器
- **bundler.rs**: 运行时打包器

## 相关文档

- [Phase 4: Codegen](../phase-04-codegen/README.md)
- [Phase 15: JIT](../phase-15-jit/README.md)
- [Phase 18: Bootstrap](../phase-18-bootstrap/README.md)
