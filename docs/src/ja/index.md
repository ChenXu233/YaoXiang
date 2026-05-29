```markdown
---
layout: page
is_home: true
sidebar: false

hero:
  name: YaoXiang // 爻象
  text: 未来志向のプログラミング言語
  tagline: 萬物が並び作す、我れ以って復りを観る
  actions:
    - theme: brand
      text: 🚀 クイックスタート
      link: /getting-started
    - theme: alt
      text: チュートリアル
      link: /tutorial/
    - theme: brand
      text: ダウンロード
      link: /download
    - theme: alt
      text: GitHub ⇗
      link: https://github.com/yaoxiang-lang/yaoxiang

tracks:
  track01:
    trackLabel: TRACK 01
    rfc: RFC-010
    title: 統一された構文
    description: "ミニマリズムの哲学。変数から関数まで、すべての宣言は name: type = value パターンに従い、学習コストが低く、コードの一貫性が高まります。"
    features:
      - 構文宣言の完全な統一
      - 型は第一級市民
  track02:
    rfc: RFC-011
    title: ゼロコストのジェネリクス
    description: "ジェネリクスの特化はコンパイル時に行われ、型の抽象化は実行時のオーバーヘッドを一切もたらしません。コンパイル時ポリモーフィズム。デッドコードの除去。型システムはマクロそのものです。"
  track03:
    rfc: RFC-009
    title: 所有権モデル
    description: "GCの停止に別れを告げましょう。爻象はスコープベースの所有権モデルを採用し、メモリの安全性はコンパイル時に確定し、予期せぬ動作はありません。"
    features:
      - 共有参照
      - 予測可能
      - GCの停止なし
      - ライフタイムなし
  track04:
    trackLabel: TRACK 04
    title: デカップルドスケジューラ
    description: "マイクロコントローラーから高性能サーバーまで、ランタイムが環境に適応します。異なるシナリオで異なるスケジューリング戦略を選択でき、パフォーマンスとリソース効率を両立します。"
    steps:
      - label: Embedded
        sub: "完全同期（Sync）"
      - label: Standard
        sub: "有向非巡回グラフ（DAG）と遅延評価に基づく自動並行管理"
      - label: Full
        sub: "ワークスティール機構（WorkSteal）"
  track05:
    title: 言語仕様 v1.8
    description: "糖衣構文の羅列を拒否。17個キーワードで全機能をカバー、複雑な糖衣構文は一切なく、純粋な表現力のみ。"
```