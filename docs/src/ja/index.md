```yaml
---
layout: page
is_home: true
sidebar: false

hero:
  name: YaoXiang // 爻象
  text: 未来指向のプログラミング言語
  tagline: 万物が并作し，吾れ以って復を觀る
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
    title: 統一された構文
    description: "ミニマリズムの哲学。変数から関数まで、すべての宣言は name: type = value パターンに従い、学習コストが低く、コードの一貫性が高い。"
    features:
      - 構文宣言の極端な統一
      - 型は第一級市民
  track02:
    rfc: RFC-011
    title: ゼロコストなジェネリクス
    description: "ジェネリクスの特殊化はコンパイル時に完了し、型の抽象化は実行時のオーバーヘッドを一切もたらさない。コンパイル時単相化 デッドコード消除。型システムはマクロである。"
  track03:
    rfc: RFC-009
    title: 所有権モデル
    description: "GCの停止に別れを告げる。爻象はスコープベースの所有権モデルを採用しており、メモリの安全性がコンパイル時に保証され、予期せぬ動作がない。"
    features:
      - 共有参照
      - 予測可能
      - GC停止なし
      - ライフタイムなし
  track04:
    trackLabel: TRACK 04
    title: ディスパッチ分離
    description: "マイコンから高性能サーバーまで、実行時環境が自ら適応する。シナリオに応じて異なるディスパッチ戦略を選択でき、パフォーマンスとリソースの両方を手にできる。"
    steps:
      - label: Embedded
        sub: "完全同期（Sync）
      - label: Standard
        sub: "有向非巡回グラフ（DAG）と遅延評価に基づく自動化された並行管理
      - label: Full
        sub: "ワークスティーリング機構（WorkSteal）
  track05:
    title: 言語仕様 v1.8
    description: "糖衣構文の氾濫を拒む。17個の基本語で全機能をカバーし、複雑な糖衣構文はなく、純粋な表現力だけがある。"
---```