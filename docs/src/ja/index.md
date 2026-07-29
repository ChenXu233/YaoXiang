---
layout: page
is_home: true
sidebar: false

hero:
  name: YaoXiang // 爻象
  text: 未来指向のプログラミング言語
  tagline: 万物并作，吾以观复
  actions:
    - theme: brand
      text: 🚀 クイックスタート
      link: /tutorial/getting-started
    - theme: alt
      text: チュートリアル
      link: /tutorial/
    - theme: brand
      text: ダウンロード
      link: /download
    - theme: alt
      text: GitHub ⇗
      link: https://github.com/ChenXu233/yaoxiang

tracks:
  track01:
    trackLabel: TRACK 01
    rfc: RFC-010
    title: 統一構文
    description:
      'ミニマリスト哲学。変数から関数まで、すべての宣言は name: type = value
      パターンに従い、学習コストが低く、コードが一貫しています。'
    features:
      - 構文宣言の完全な統一
      - 型は第一級市民
  track02:
    rfc: RFC-011
    title: ゼロコストジェネリクス
    description: 'ジェネリクスの特殊化はコンパイル時に完了し、型の抽象化は実行時のオーバーヘッドをもたらしません。コンパイル時モノモーフィズム。デッドコード排除。型システムはマクロそのものです。'
  track03:
    rfc: RFC-009
    title: 所有権モデル
    description: 'GCの一時停止に別れを告げます。爻象はスコープベースの所有権モデルを採用し、メモリ安全性がコンパイル時に確定し、予期せぬ動作がありません。'
    features:
      - 共有参照
      - 予測可能
      - GCの一時停止なし
      - ライフタイムなし
  track04:
    trackLabel: TRACK 04
    title: デカップリングスケジューラ
    description: 'マイコンから高性能サーバーまで、実行時に環境が適応します。異なるシナリオで異なるスケジューリング戦略を選択でき、パフォーマンスとリソースの両方を獲得。'
    steps:
      - label: Embedded
        sub: '完全同期(Sync)'
      - label: Standard
        sub: '有向非巡回グラフ(DAG)と遅延評価に基づく自動化された同時実行管理'
      - label: Full
        sub: 'ワークスティーリング(WorkSteal)'
  track05:
    title: 言語仕様 v1.8
    description: '構文糖衣の氾濫を拒否。17個のキーワードですべての機能をカバーし、複雑な構文糖衣はなく、純粋な表現力のみ。'
---
