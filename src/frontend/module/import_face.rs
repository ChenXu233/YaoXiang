//! 导入面的**类型候选提供方表**（WBS 4.9.9 修复轮 4–5）。
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
//!
//! 冲突（同一名字被 ≥2 个模块带进作用域）**必须可见**：`providers` 返回全部提供
//! 模块、`provider_count` 反映真实数量、`winner` 返回 None 且 `is_conflicted` 为真。
//! 消费侧据此识别「冲突」而不是「未知」：命名实参响亮拒绝、位置实参走接收者
//! owner 的保守相容判定（见 `middle::lower::query::method_source_mismatch`）。

use std::collections::HashMap;

use crate::ast;
use crate::frontend::core::sema::passes::dead_code::DeadCodeAnalyzer;
use crate::frontend::module::registry::ModuleRegistry;

/// 本单元导入面里，每个**定义侧类型名**的候选提供模块（按来源分级登记）。
///
/// 每个名字保留**全部**提供模块（按 use 面书写序，去重）：冲突数量与顺序都是
/// 可观测事实，不再只留「第一个模块 + 一个冲突标记」（4.9.9 修复轮 4 的前车之鉴：
/// 只存一个模块时 `provider_count` 对冲突返回 1，消费侧误判为「未知」而静默放行）。
#[derive(Debug, Default, Clone)]
pub struct TypeCandidateProviders {
    /// 裸名提供方：`use M.{Type}`（本地名 == 目标名），按书写序
    bare: HashMap<String, Vec<String>>,
    /// 别名提供方：`use M.{Type as P}`（键仍取目标名），按书写序
    aliased: HashMap<String, Vec<String>>,
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
            let table = if local == target {
                &mut providers.bare
            } else {
                &mut providers.aliased
            };
            let modules = table.entry(target.clone()).or_default();
            // 去重但保留书写序：顺序是确定性判据的一部分（不得读 HashMap 迭代序）。
            if !modules.iter().any(|module| module == use_path) {
                modules.push(use_path.clone());
            }
        }
        providers
    }

    /// 候选注入面实际采用的「类型名 → 提供模块」逐对列表（升序）。
    ///
    /// 裸名提供方优先；裸名侧或别名侧冲突（多模块同名）⇒ 该名字整名跳过；
    /// 别名只在裸名侧完全没有该名字时补位。顺序按类型名升序（合并顺序确定）。
    pub fn injected_pairs(&self) -> Vec<(String, String)> {
        let mut pairs: Vec<(String, String)> = self
            .bare
            .iter()
            .filter(|(_, modules)| modules.len() == 1)
            .map(|(name, modules)| (name.clone(), modules[0].clone()))
            .collect();
        pairs.extend(
            self.aliased
                .iter()
                .filter(|(name, modules)| modules.len() == 1 && !self.bare.contains_key(*name))
                .map(|(name, modules)| (name.clone(), modules[0].clone())),
        );
        pairs.sort();
        pairs
    }

    /// 某个定义侧类型名的候选**提供模块**（候选注入面选用的那一个）；
    /// 冲突（同级多模块同名）或未导入时为 None——冲突可用
    /// [`Self::is_conflicted`] 与未导入区分。
    pub fn winner(
        &self,
        type_name: &str,
    ) -> Option<&str> {
        if let Some(modules) = self.bare.get(type_name) {
            return (modules.len() == 1).then(|| modules[0].as_str());
        }
        self.aliased
            .get(type_name)
            .filter(|modules| modules.len() == 1)
            .map(|modules| modules[0].as_str())
    }

    /// 该定义侧类型名在本单元导入面里的**全部**提供模块（去重；裸名在前）。
    pub fn providers(
        &self,
        type_name: &str,
    ) -> Vec<&str> {
        let mut modules: Vec<&str> = Vec::new();
        for module in self
            .bare
            .get(type_name)
            .into_iter()
            .chain(self.aliased.get(type_name))
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

    /// 该类型名是否**冲突**（任一级来源被 ≥2 个模块带进作用域）。
    ///
    /// 两级**取或**：裸名侧多模块、或别名侧多模块，都构成名字层歧义。
    /// （候选注入面在裸名存在时仍按裸名优先注入；本判据只回答「这个名字在名字层
    /// 是否已经不唯一」，供消费侧决定是否按名/按位置拒绝。）
    pub fn is_conflicted(
        &self,
        type_name: &str,
    ) -> bool {
        self.bare
            .get(type_name)
            .is_some_and(|modules| modules.len() > 1)
            || self
                .aliased
                .get(type_name)
                .is_some_and(|modules| modules.len() > 1)
    }
}

#[cfg(test)]
mod tests {
    use super::TypeCandidateProviders;

    /// 直接构造（单元测试不建 AST/注册表）；顺序即构造顺序。
    fn table(
        bare: &[(&str, &str)],
        aliased: &[(&str, &str)],
    ) -> TypeCandidateProviders {
        let mut providers = TypeCandidateProviders::default();
        for (target, module) in bare {
            providers
                .bare
                .entry((*target).to_string())
                .or_default()
                .push((*module).to_string());
        }
        for (target, module) in aliased {
            providers
                .aliased
                .entry((*target).to_string())
                .or_default()
                .push((*module).to_string());
        }
        providers
    }

    #[test]
    fn test_conflicting_alias_providers_list_all_modules() {
        // Arrange：两个别名（不同本地名）从两个模块引入同名类型 Temp
        let providers = table(&[], &[("Temp", "a"), ("Temp", "b")]);

        // Act
        let modules = providers.providers("Temp");
        let count = providers.provider_count("Temp");

        // Assert
        assert!(
            modules == vec!["a", "b"],
            "冲突 alias 的 providers 必须列出全部提供模块（书写序），实际：{modules:?}"
        );
        assert!(
            count >= 2,
            "冲突 alias 的 provider_count 必须 ≥2（修复前只存一个模块、返回 1），实际：{count}"
        );
    }

    #[test]
    fn test_conflicting_alias_has_no_winner_and_is_conflicted() {
        // Arrange
        let providers = table(&[], &[("Temp", "a"), ("Temp", "b")]);

        // Act
        let winner = providers.winner("Temp");
        let conflicted = providers.is_conflicted("Temp");

        // Assert
        assert!(
            winner.is_none(),
            "冲突时没有唯一候选提供模块，winner 必须为 None，实际：{winner:?}"
        );
        assert!(
            conflicted,
            "冲突必须能被消费侧识别为『冲突』而不是『未导入』"
        );
    }

    #[test]
    fn test_conflicting_alias_is_skipped_by_injected_pairs() {
        // Arrange：Temp 冲突（两别名），Other 单提供方
        let providers = table(&[("Other", "b")], &[("Temp", "a"), ("Temp", "b")]);

        // Act
        let pairs = providers.injected_pairs();

        // Assert
        assert!(
            pairs.iter().all(|(name, _)| name != "Temp"),
            "冲突名字必须整名跳过（不注入候选），实际 pairs：{pairs:?}"
        );
        assert!(
            pairs
                .iter()
                .any(|(name, module)| name == "Other" && module == "b"),
            "非冲突的裸名提供方必须照常注入，实际 pairs：{pairs:?}"
        );
    }

    #[test]
    fn test_single_alias_has_winner_and_single_provider() {
        // Arrange
        let providers = table(&[], &[("Temp", "a")]);

        // Act
        let winner = providers.winner("Temp");

        // Assert
        assert!(
            winner == Some("a"),
            "单个别名提供方必须有 winner（4.9.9 别名形态的既有收益），实际：{winner:?}"
        );
        assert!(!providers.is_conflicted("Temp"), "单提供方不是冲突");
        assert_eq!(providers.provider_count("Temp"), 1, "单提供方计数必须为 1");
    }

    #[test]
    fn test_bare_and_alias_conflict_is_or_of_both_levels() {
        // Arrange：裸名提供方 a（1 个）+ 别名提供方 b、c（2 个 ⇒ 别名侧冲突）
        let providers = table(&[("Temp", "a")], &[("Temp", "b"), ("Temp", "c")]);

        // Act
        let conflicted = providers.is_conflicted("Temp");
        let winner = providers.winner("Temp");
        let count = providers.provider_count("Temp");

        // Assert
        assert!(
            conflicted,
            "别名侧多模块必须被 is_conflicted 记为冲突（两级取或），实际：false"
        );
        assert!(
            winner == Some("a"),
            "裸名存在时 winner 仍按裸名优先（注入面规则），实际：{winner:?}"
        );
        assert_eq!(count, 3, "providers 必须列出全部三个提供模块");
    }
}
