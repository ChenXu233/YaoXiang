//! 模块加载器
//!
//! 负责从文件系统加载用户模块，管理模块缓存，并检测循环依赖。

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::resolver::{ModuleLocation, ModuleResolver};
use super::{ModuleError, ModuleInfo, ModuleSource};

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
    fn load_from_file(
        &mut self,
        module_path: &str,
        file_path: &PathBuf,
    ) -> Result<ModuleInfo, ModuleError> {
        // 读取文件内容
        let _source = std::fs::read_to_string(file_path).map_err(|_| ModuleError::NotFound {
            path: module_path.to_string(),
            searched_paths: vec![file_path.display().to_string()],
        })?;

        // TODO: 解析源文件，提取导出项
        // 当前返回空模块，后续实现完整的解析
        let module = ModuleInfo::new(module_path.to_string(), ModuleSource::User);

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
}
