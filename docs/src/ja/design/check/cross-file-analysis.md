```markdown
---
title: ファイル間分析
description: YaoXiang check ファイル間型チェックの設計
---

# ファイル間分析

## 問題の説明

初期実装では、`check_files_with_diagnostics` が各ファイルに対して独立した `Compiler` を作成し、ファイル間の参照を検出できませんでした。fileA で定義された `pub` 関数は fileB では認識されませんでした。

## 解決策

共有 `TypeEnvironment` を使用し、依存順序に従ってすべてのモジュールをチェックします。

## 実装フロー

```text
1. すべての .yx ファイルを並行解析 → Vec<(PathBuf, ModuleId, AST)>
2. ModuleDependencyGraph::build_from_ast で依存グラフを構築
3. detect_cycles() で循環依存をチェック → エラーを報告
4. topological_sort() でコンパイル順序を取得
5. 順序に従って型チェック：
   a. 共有 TypeEnvironment を作成（std モジュールを含む）
   b. 各モジュールに対して：エクスポートを共有環境に登録 → 型チェック
   c. 診断情報を収集
6. CheckResult を返す
```

## 名前空間隔离

`module_name.symbol_name` 形式でエクスポートシンボルsto保存し、異なるモジュールの同 名シンボルの競合を避けます。

## 既知の制限

- `traits/` はプレースホルダー実装（coherence/impl_check/object_safety/resolution）が未完成
- `check_single_module` はまだ各モジュールに対して独立した Compiler を作成（共有 env の 型情報伝播がまだ完全に実装されていない）

## 今後の作業

- T8: ファイル間型チェックのエンドツーエンドテスト
- A4: 共有 trait_table と native_signatures
```