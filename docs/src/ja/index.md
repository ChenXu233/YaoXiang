```yaml
---
layout: page
is_home: true
sidebar: false

hero:
  name: YaoXiang // 爻象
  text: 未来志向のプログラミング言語
  tagline: 万物並作、吾以て復を観る
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
    title: '統一構文'
    description:
      'ミニマリズム哲学。変数から関数まで、全ての宣言が name: type = value
      パターンに従い、学習コストが低く、コードがより一貫しています。'
    features:
      - 構文宣言の極致的統一
      - 型はファーストクラス市民
  track02:
    rfc: RFC-011
    title: 'ゼロコストジェネリクス'
    description: 'ジェネリクス特殊化はコンパイル時に完了し、型の抽象化はランタイムのオーバーヘッドを一切もたらしません。コンパイル時の単相化。デッドコード除去。型システムがマクロです。'
  track03:
    rfc: RFC-009
    title: '所有権モデル'
    description: 'GCのストールに別れを告げましょう。爻象はスコープベースの所有権モデルを使用し、メモリ安全性はコンパイル時に確定し、予期せぬことはありません。'
    features:
      - 共有参照
      - 予測可能
      - GCストールなし
      - ライフタイムなし
  track04:
    trackLabel: TRACK 04
    title: '疎結合スケジューラ'
    description: 'マイコンから高性能サーバーまで、ランタイムが環境に自動適応します。異なるシナリオで異なるスケジューリング戦略を選択し、パフォーマンスとリソースを両立します。'
    steps:
      - label: Embedded
        sub: '完全同期(Sync)'
      - label: Standard
        sub: '有向非巡回グラフ(DAG)と遅延評価に基づく自動並行管理'
      - label: Full
        sub: 'ワークスティーリング機構(WorkSteal)'
  track05:
    title: '言語仕様 v1.8'
    description: 'シンタックスシュガーの氾濫を拒否。17個のキーワードで全機能をカバーし、複雑なシンタックスシュガーはなく、純粋な表現力のみ。'
---
```
