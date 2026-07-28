---
title: ファイル間分析
description: YaoXiang check におけるファイル間型検査の設計
---

# ファイル間分析

## 問題の説明

初期実装では、`check_files_with_diagnostics` が各ファイルに対して独立した
`Compiler` を作成し、ファイル間の参照を検出できませんでした。fileA で定義された `pub` 関数は fileB では認識されませんでした。

## 解決策

共有 `TypeEnvironment` を使用し、依存関係の順序で全モジュールを検査します。

## 実装フロー

```text
1. 全 .yx ファイルを並列解析 → Vec<(PathBuf, ModuleId, AST)>
2. ModuleDependencyGraph::build_from_ast で依存関係グラフを構築
3. detect_cycles() で循環依存を検出 → エラー報告
4. topological_sort() でコンパイル順序を取得
5. 順序通りに型検査を実行：
   a. 共有 TypeEnvironment を作成（std モジュールを含む）
   b. 各モジュールに対して：そのエクスポートを共有環境に登録 → 型検査
   c. 診断情報を収集
6. CheckResult を返す
```

## 名前空間の分離

エクスポートされたシンボルを `module_name.symbol_name` 形式で保存し、異なるモジュールの同名シンボル間の競合を防ぎます。

## 既知の制限

- `traits/` はプレースホルダー実装（coherence/impl_check/object_safety/resolution）は未完成
- `check_single_module` はまだ各モジュールに対して独立した Compiler を作成（共有 env の型情報伝播がまだ完全に実装されていない）

## 今後の作業

- T8：ファイル間型検査のエンドツーエンドテスト
- A4：trait_table と native_signatures の共有
