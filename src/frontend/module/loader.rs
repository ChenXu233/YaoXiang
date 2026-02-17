//! 模块加载器
//!
//! 负责从文件系统加载用户模块，管理模块缓存，并检测循环依赖。
//! 支持解析 `.yx` 源文件，自动提取导出项（`pub` 函数、类型定义等）。

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::ast::{Module as AstModule, StmtKind, Type as AstType};
use crate::frontend::core::parser::parse;

use super::resolver::{ModuleLocation, ModuleResolver};
use super::{Export, ExportKind, ModuleError, ModuleInfo, ModuleSource};

/// 加载状态（用于循环依赖检测）
#[derive(Debug, Clone, PartialEq, Eq)]
enum LoadState {
    /// 正在加载
    Loading,
    /// 加载完成
    Loaded,
}

/// 模块加载器
///
/// 从文件系统加载模块，支持缓存和循环依赖检测。
#[derive(Debug)]
pub struct ModuleLoader {
    /// 模块解析器
    resolver: ModuleResolver,
    /// 模块缓存（path -> ModuleInfo）
    cache: HashMap<String, ModuleInfo>,
    /// 加载状态追踪（用于循环依赖检测）
    load_states: HashMap<String, LoadState>,
    /// 当前加载栈（用于报告循环路径）
    load_stack: Vec<String>,
}

impl ModuleLoader {
    /// 创建新的加载器
    pub fn new(
        project_root: PathBuf,
        current_file: PathBuf,
    ) -> Self {
        Self {
            resolver: ModuleResolver::new(project_root, current_file),
            cache: HashMap::new(),
            load_states: HashMap::new(),
            load_stack: Vec::new(),
        }
    }

    /// 加载模块
    ///
    /// 根据模块路径加载模块。如果模块已缓存则直接返回。
    /// 如果检测到循环依赖则返回错误。
    pub fn load(
        &mut self,
        module_path: &str,
    ) -> Result<ModuleInfo, ModuleError> {
        // 检查缓存
        if let Some(cached) = self.cache.get(module_path) {
            return Ok(cached.clone());
        }

        // 检查循环依赖
        if self.load_states.get(module_path) == Some(&LoadState::Loading) {
            let cycle = self.format_cycle(module_path);
            return Err(ModuleError::CyclicDependency { cycle });
        }

        // 标记开始加载
        self.load_states
            .insert(module_path.to_string(), LoadState::Loading);
        self.load_stack.push(module_path.to_string());

        // 解析模块位置
        let location = self.resolver.resolve(module_path)?;

        let module = match location {
            ModuleLocation::Std(_) => {
                // std 模块不在此处加载，应该已经在 registry 中注册
                return Err(ModuleError::NotFound {
                    path: module_path.to_string(),
                    searched_paths: vec!["std modules should be accessed via registry".to_string()],
                });
            }
            ModuleLocation::File(path) => self.load_from_file(module_path, &path)?,
            ModuleLocation::Vendor(path) => self.load_from_file(module_path, &path)?,
        };

        // 标记加载完成
        self.load_states
            .insert(module_path.to_string(), LoadState::Loaded);
        self.load_stack.pop();

        // 缓存
        self.cache.insert(module_path.to_string(), module.clone());

        Ok(module)
    }

    /// 从文件加载模块
    ///
    /// 读取 `.yx` 文件，使用词法分析器和解析器提取导出项。
    ///
    /// 导出规则：
    /// - `pub` 修饰的函数定义 → `ExportKind::Function`
    /// - 类型定义（`Name: Type = ...`） → `ExportKind::Type`
    /// - 顶层不可变变量绑定 → `ExportKind::Constant`
    fn load_from_file(
        &mut self,
        module_path: &str,
        file_path: &PathBuf,
    ) -> Result<ModuleInfo, ModuleError> {
        // 读取文件内容
        let source = std::fs::read_to_string(file_path).map_err(|_| ModuleError::NotFound {
            path: module_path.to_string(),
            searched_paths: vec![file_path.display().to_string()],
        })?;

        // 词法分析
        let tokens = tokenize(&source).map_err(|e| ModuleError::InvalidPath {
            path: format!("{}: {}", file_path.display(), e),
        })?;

        // 语法分析
        let ast = parse(&tokens).map_err(|e| ModuleError::InvalidPath {
            path: format!("{}: {}", file_path.display(), e),
        })?;

        // 提取导出项
        let module = Self::extract_exports(module_path, &ast, &ModuleSource::User);

        Ok(module)
    }

    /// 从 AST 中提取模块导出项
    ///
    /// 遍历模块顶层语句，根据以下规则提取导出项：
    /// - `pub fn_name: (...) -> ... = ...` → 公开函数
    /// - `Name: Type = ...` → 类型定义（始终导出）
    /// - `name = expr`（不可变绑定） → 常量
    fn extract_exports(
        module_path: &str,
        ast: &AstModule,
        source: &ModuleSource,
    ) -> ModuleInfo {
        let mut module = ModuleInfo::new(module_path.to_string(), source.clone());

        for stmt in &ast.items {
            match &stmt.kind {
                // pub 函数 → 导出
                StmtKind::Fn {
                    name,
                    type_annotation,
                    is_pub,
                    ..
                } => {
                    if *is_pub {
                        let signature = type_annotation
                            .as_ref()
                            .map(|t| format_type(t))
                            .unwrap_or_else(|| "(...) -> Any".to_string());
                        module.add_export(Export {
                            name: name.clone(),
                            full_path: format!("{}.{}", module_path, name),
                            kind: ExportKind::Function,
                            signature,
                        });
                    }
                }

                // 类型定义始终导出
                StmtKind::TypeDef { name, .. } => {
                    module.add_export(Export {
                        name: name.clone(),
                        full_path: format!("{}.{}", module_path, name),
                        kind: ExportKind::Type,
                        signature: "Type".to_string(),
                    });
                }

                // 顶层不可变绑定 → 常量导出
                StmtKind::Var {
                    name,
                    is_mut,
                    type_annotation,
                    ..
                } => {
                    if !is_mut {
                        let signature = type_annotation
                            .as_ref()
                            .map(|t| format_type(t))
                            .unwrap_or_else(|| "Any".to_string());
                        module.add_export(Export {
                            name: name.clone(),
                            full_path: format!("{}.{}", module_path, name),
                            kind: ExportKind::Constant,
                            signature,
                        });
                    }
                }

                // use/方法绑定/其他语句不产生导出
                _ => {}
            }
        }

        module
    }

    /// 加载 vendor 模块
    ///
    /// 直接从指定文件路径加载模块，标记为 Vendor 来源。
    /// 用于 VendorBridge 发现的依赖模块。
    pub fn load_vendor_module(
        &mut self,
        module_name: &str,
        file_path: &PathBuf,
    ) -> Result<ModuleInfo, ModuleError> {
        let mut module = self.load_from_file(module_name, file_path)?;
        module.source = ModuleSource::Vendor;
        Ok(module)
    }

    /// 格式化循环依赖路径
    fn format_cycle(
        &self,
        current: &str,
    ) -> String {
        let mut cycle_parts = Vec::new();
        let mut found = false;
        for path in &self.load_stack {
            if path == current {
                found = true;
            }
            if found {
                cycle_parts.push(path.as_str());
            }
        }
        cycle_parts.push(current);
        cycle_parts.join(" -> ")
    }

    /// 检测依赖图中的循环
    ///
    /// 使用 Kahn 算法进行拓扑排序，如果排序失败则说明存在循环依赖。
    pub fn detect_cycles(
        dependencies: &HashMap<String, Vec<String>>
    ) -> Result<Vec<String>, ModuleError> {
        // 构建入度表
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut all_modules: HashSet<&str> = HashSet::new();

        for (module, deps) in dependencies {
            all_modules.insert(module);
            in_degree.entry(module).or_insert(0);
            for dep in deps {
                all_modules.insert(dep);
                *in_degree.entry(dep).or_insert(0) += 1;
            }
        }

        // Kahn 算法
        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&name, _)| name)
            .collect();

        let mut sorted = Vec::new();

        while let Some(module) = queue.pop() {
            sorted.push(module.to_string());

            if let Some(deps) = dependencies.get(module) {
                for dep in deps {
                    if let Some(deg) = in_degree.get_mut(dep.as_str()) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(dep);
                        }
                    }
                }
            }
        }

        if sorted.len() < all_modules.len() {
            // 存在循环，找出参与循环的模块
            let sorted_set: HashSet<&str> = sorted.iter().map(|s| s.as_str()).collect();
            let cycle_modules: Vec<String> = all_modules
                .iter()
                .filter(|m| !sorted_set.contains(**m))
                .map(|m| m.to_string())
                .collect();

            Err(ModuleError::CyclicDependency {
                cycle: cycle_modules.join(" -> "),
            })
        } else {
            Ok(sorted)
        }
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.load_states.clear();
    }
}

/// 将 AST 类型格式化为可读签名字符串
///
/// 用于生成模块导出项的签名描述。
fn format_type(ty: &AstType) -> String {
    match ty {
        AstType::Name(name) => name.clone(),
        AstType::Int(bits) => format!("Int{}", bits),
        AstType::Float(bits) => format!("Float{}", bits),
        AstType::Char => "Char".to_string(),
        AstType::String => "String".to_string(),
        AstType::Bytes => "Bytes".to_string(),
        AstType::Bool => "Bool".to_string(),
        AstType::Void => "Void".to_string(),
        AstType::Fn {
            params,
            return_type,
        } => {
            let param_str: Vec<String> = params.iter().map(|p| format_type(p)).collect();
            format!("({}) -> {}", param_str.join(", "), format_type(return_type))
        }
        AstType::Option(inner) => format!("{}?", format_type(inner)),
        AstType::Result(ok, err) => {
            format!("Result[{}, {}]", format_type(ok), format_type(err))
        }
        AstType::Generic { name, args } => {
            let args_str: Vec<String> = args.iter().map(|a| format_type(a)).collect();
            format!("{}[{}]", name, args_str.join(", "))
        }
        AstType::Tuple(types) => {
            let parts: Vec<String> = types.iter().map(|t| format_type(t)).collect();
            format!("({})", parts.join(", "))
        }
        AstType::Ptr(inner) => format!("*{}", format_type(inner)),
        _ => "Any".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_no_cycles() {
        let mut deps = HashMap::new();
        deps.insert("a".to_string(), vec!["b".to_string()]);
        deps.insert("b".to_string(), vec!["c".to_string()]);
        deps.insert("c".to_string(), vec![]);

        let result = ModuleLoader::detect_cycles(&deps);
        assert!(result.is_ok());
    }

    #[test]
    fn test_detect_direct_cycle() {
        let mut deps = HashMap::new();
        deps.insert("a".to_string(), vec!["b".to_string()]);
        deps.insert("b".to_string(), vec!["a".to_string()]);

        let result = ModuleLoader::detect_cycles(&deps);
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_indirect_cycle() {
        let mut deps = HashMap::new();
        deps.insert("a".to_string(), vec!["b".to_string()]);
        deps.insert("b".to_string(), vec!["c".to_string()]);
        deps.insert("c".to_string(), vec!["a".to_string()]);

        let result = ModuleLoader::detect_cycles(&deps);
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_self_reference() {
        let mut deps = HashMap::new();
        deps.insert("a".to_string(), vec!["a".to_string()]);

        let result = ModuleLoader::detect_cycles(&deps);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_exports_pub_fn() {
        let source = r#"
pub add: (a: Int, b: Int) -> Int = (a, b) => {
    a + b
}

helper: (x: Int) -> Int = (x) => {
    x * 2
}
"#;
        let tokens = tokenize(source).unwrap();
        let ast = parse(&tokens).unwrap();
        let module = ModuleLoader::extract_exports("my_module", &ast, &ModuleSource::User);

        // pub 函数应该被导出
        assert!(module.has_export("add"));
        assert_eq!(module.get_export("add").unwrap().kind, ExportKind::Function);
        assert_eq!(module.get_export("add").unwrap().full_path, "my_module.add");

        // 非 pub 函数不导出
        assert!(!module.has_export("helper"));
    }

    #[test]
    fn test_extract_exports_typedef() {
        let source = r#"
Point: Type = {
    x: Int
    y: Int
}
"#;
        let tokens = tokenize(source).unwrap();
        let ast = parse(&tokens).unwrap();
        let module = ModuleLoader::extract_exports("shapes", &ast, &ModuleSource::User);

        // 类型定义始终导出
        assert!(module.has_export("Point"));
        assert_eq!(module.get_export("Point").unwrap().kind, ExportKind::Type);
    }

    #[test]
    fn test_extract_exports_constant() {
        let source = r#"
MAX_SIZE = 100
mut counter = 0
"#;
        let tokens = tokenize(source).unwrap();
        let ast = parse(&tokens).unwrap();
        let module = ModuleLoader::extract_exports("config", &ast, &ModuleSource::User);

        // 不可变绑定导出为常量
        assert!(module.has_export("MAX_SIZE"));
        assert_eq!(
            module.get_export("MAX_SIZE").unwrap().kind,
            ExportKind::Constant
        );

        // 可变绑定不导出
        assert!(!module.has_export("counter"));
    }

    #[test]
    fn test_format_type_fn() {
        let ty = AstType::Fn {
            params: vec![
                AstType::Name("Int".to_string()),
                AstType::Name("Int".to_string()),
            ],
            return_type: Box::new(AstType::Name("Bool".to_string())),
        };
        assert_eq!(format_type(&ty), "(Int, Int) -> Bool");
    }

    #[test]
    fn test_format_type_generic() {
        let ty = AstType::Generic {
            name: "List".to_string(),
            args: vec![AstType::Name("Int".to_string())],
        };
        assert_eq!(format_type(&ty), "List[Int]");
    }
}
