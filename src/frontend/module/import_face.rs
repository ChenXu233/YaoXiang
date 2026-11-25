//! 导入面的**类型候选提供方表**（WBS 4.9.9 修复轮 4）。
//!
//! 唯一生产：候选注入面（`driver::arms::visible_method_overloads`）与消费侧同源判据
//! （`middle::lower::query`）都从本表读「某个定义侧类型名的候选提供模块」，避免两条
//! 规则各自演化（禁令三：一处生产、一处消费）。
//!
//! 规则（4.9.7–4.9.9 的重载候选收窄，逐字迁移自注入面）：
//! ① 只认**花括号项**形态（`use M.{Type}`）；整体导入 `use M` 不在此表；
//! ② 键一律是**定义侧类型名**（内联别名的 target）——候选键前缀就是它；
//! ③ **提供方优先级**：裸名提供方（`local == target`）优先；裸名侧缺席时才用别名
//!    提供方；同一级内被多个模块带进作用域 = 该名字在名字层已歧义 ⇒ 整名跳过
//!    （裸名侧冲突时别名也不得补位）。

use std::collections::{HashMap, HashSet};

use crate::ast;
use crate::frontend::core::sema::passes::dead_code::DeadCodeAnalyzer;
use crate::frontend::module::registry::ModuleRegistry;

/// 本单元导入面里，每个**定义侧类型名**的候选提供模块（按来源分级登记）。
#[derive(Debug, Default, Clone)]
pub struct TypeCandidateProviders {
    /// 裸名提供方：`use M.{Type}`（本地名 == 目标名）
    bare: HashMap<String, String>,
    /// 别名提供方：`use M.{Type as P}`（键仍取目标名）
    aliased: HashMap<String, String>,
    /// 裸名侧被 ≥2 个模块带进作用域的名字（名字层歧义 ⇒ 整名跳过）
    conflicted_bare: HashSet<String>,
    /// 别名侧被 ≥2 个模块带进作用域的名字
    conflicted_alias: HashSet<String>,
}

impl TypeCandidateProviders {
    /// 扫一个单元的 `use` 面，构建提供方表。
    pub fn scan(
        ast: &ast::Module,
        registry: &ModuleRegistry,
    ) -> Self {
        let names = DeadCodeAnalyzer::collect_module_names(ast);
        let mut providers = Self::default();
        for (local, target, use_path) in &names.imported {
            // use 路径必须命中注册表模块、且该模块确实导出这个名字（未解析导入 /
            // 子模块名都不构成候选提供方）；查法与 checker 的 use 处理同表。
            let Some(exports) = registry.get_exports(use_path) else {
                continue;
            };
            if !exports.contains_key(target) {
                continue;
            }
            // 本地别名不参与候选键：键的前缀是接收者的已解析类型名 = 定义侧名字。
            // 这里只决定**提供方优先级**：裸名提供方优先。
            let (table, conflicted) = if local == target {
                (&mut providers.bare, &mut providers.conflicted_bare)
            } else {
                (&mut providers.aliased, &mut providers.conflicted_alias)
            };
            match table.get(target) {
                Some(existing) if existing != use_path => {
                    conflicted.insert(target.clone());
                }
                Some(_) => {}
                None => {
                    table.insert(target.clone(), use_path.clone());
                }
            }
        }
        providers
    }

    /// 候选注入面实际采用的「类型名 → 提供模块」逐对列表（升序）。
    ///
    /// 裸名提供方优先；裸名侧有冲突（多模块同名）⇒ 该名字整名跳过、别名不得补位；
    /// 别名只在裸名侧完全没有该名字时补位。
    pub fn injected_pairs(&self) -> Vec<(String, String)> {
        let mut pairs: Vec<(String, String)> = self
            .bare
            .iter()
            .filter(|(name, _)| !self.conflicted_bare.contains(*name))
            .map(|(name, module)| (name.clone(), module.clone()))
            .collect();
        pairs.extend(
            self.aliased
                .iter()
                .filter(|(name, _)| {
                    !self.bare.contains_key(*name) && !self.conflicted_alias.contains(*name)
                })
                .map(|(name, module)| (name.clone(), module.clone())),
        );
        pairs.sort();
        pairs
    }

    /// 某个定义侧类型名的候选**提供模块**（候选注入面选用的那一个）；
    /// 该名字无候选（未导入 / 名字层歧义）时为 None。
    pub fn winner(
        &self,
        type_name: &str,
    ) -> Option<&str> {
        if let Some(module) = self.bare.get(type_name) {
            if self.conflicted_bare.contains(type_name) {
                return None;
            }
            return Some(module.as_str());
        }
        if self.conflicted_alias.contains(type_name) {
            return None;
        }
        self.aliased.get(type_name).map(String::as_str)
    }

    /// 该类型名在本单元导入面里的全部提供模块（去重；裸名提供方在前）。
    pub fn providers(
        &self,
        type_name: &str,
    ) -> Vec<&str> {
        let mut modules: Vec<&str> = Vec::new();
        for module in [self.bare.get(type_name), self.aliased.get(type_name)]
            .into_iter()
            .flatten()
        {
            if !modules.contains(&module.as_str()) {
                modules.push(module.as_str());
            }
        }
        modules
    }

    /// 该类型名的提供模块数（≥2 = 名字层歧义）。
    pub fn provider_count(
        &self,
        type_name: &str,
    ) -> usize {
        self.providers(type_name).len()
    }
}
