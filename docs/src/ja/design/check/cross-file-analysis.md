---
title: 'ファイル間分析'
description: 'YaoXiang check ファイル間型チェックの設計'
---

# ファイル間分析

## 問題の説明

初期実装では、`check_files_with_diagnostics` は各ファイルに対して独立した `Compiler`
を作成し、ファイル間参照を検出できなかった。fileA で定義された `pub` 関数は fileB で認識されない。

## 解決策

共有の `TypeEnvironment` を使用し、依存関係順にすべてのモジュールをチェックする。

## 実装フロー

```text
1. すべての .yx ファイルを並列解析 → Vec<(PathBuf, ModuleId, AST)>
2. ModuleDependencyGraph::build_from_ast を使用して依存グラフを構築
3. detect_cycles() で循環依存をチェック → エラー報告
4. topological_sort() でコンパイル順序を取得
5. 順次型チェック：
   a. 共有 TypeEnvironment を作成（std モジュールを含む）
   b. 各モジュールについて：共有環境にエクスポートを登録 → 型チェック
   c. 診断情報を収集
6. CheckResult を返す
```

## 名前空間の分離

`module_name.symbol_name`
形式でエクスポートシンボルを保存し、異なるモジュールの同名シンボルの競合を回避する。

## 既知の制限

- `traits/` のプレースホルダ実装（coherence/impl_check/object_safety/resolution）が未完成
- `check_single_module`
  は依然として各モジュールに対して独立した Compiler を作成する（共有 env の型情報伝達がまだ完全には実装されていない）

## 今後の作業

- T8：ファイル間型チェックのエンドツーエンドテスト
- A4：共有 trait_table と native_signatures
