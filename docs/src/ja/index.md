```yaml
---
layout: page
is_home: true
sidebar: false

hero:
  name: YaoXiang # 爻象
  text: 未来のためのプログラミング言語
  tagline: 万物が発生し消える、私はそれらを見守る
  actions:
    - theme: brand
      text: 🚀 クイックスタート
      link: /getting-started
    - theme: alt
      text: チュートリアル
      link: /tutorial/
    - theme: brand
      text: 本体をダウンロード
      link: /download
    - theme: alt
      text: GitHub ⇗
      link: https://github.com/yaoxiang-lang/yaoxiang

tracks:
  track01:
    trackLabel: TRACK 01
    rfc: RFC-010
    title: 統一構文
    description: "ミニマリスト哲学。変数から関数まで、すべての宣言は name: type = value パターンに従う。学習コストが低く、コードがより一貫している。"
    features:
      - 構文宣言の極端な統一
      - 型は第一級市民
  track02:
    rfc: RFC-011
    title: ゼロコストジェネリクス
    description: "ジェネリクスの特殊化はコンパイル時に行われる。型の抽象化による実行時オーバーヘッドなし。コンパイル時単態化、デッドコードエリミネーション。型システムがマクロ。"
  track03:
    rfc: RFC-009
    title: 所有権モデル
    description: "GCのもたつきよ、さようなら。爻象はスコープベースの所有権モデルを使用し、メモリ安全はコンパイル時に確定する。予期せぬ動作なし。"
    features:
      - 共有参照
      - 予測可能
      - GCのもたつきなし
      - ライフタイムなし
  track04:
    trackLabel: TRACK 04
    title: デカップルドスケジューラ
    description: "マイコンから高性能サーバーまで、ランタイムが環境に自動適応。異なるシナリオで異なるスケジューリング戦略を選択でき、パフォーマンスとリソースを両立。"
    steps:
      - label: Embedded
        sub: "完全同期(Sync)"
      - label: Standard
        sub: "有向非巡回グラフ(DAG)と遅延評価に基づく自動並行管理"
      - label: Full
        sub: "ワークスティーリング(WorkSteal)機構"
  track05:
    title: 言語仕様 v1.8
    description: "糖衣構文の氾濫を拒否。17個キーワードで全機能をカバー。複雑な糖衣構文はなく、純粋な表現力のみ。"
---
```