---
layout: page
is_home: true
sidebar: false

hero:
  name: YaoXiang // 爻象
  text: 未来志向のプログラミング言語
  tagline: 万物並び作す、吾以て復を観る
  actions:
    - theme: brand
      text: 🚀 クイックスタート
      link: /tutorial/getting-started
    - theme: alt
      text: チュートリアル
      link: /tutorial/
    - theme: brand
      text: 本体をダウンロード
      link: /download
    - theme: alt
      text: GitHub ⇗
      link: https://github.com/ChenXu233/yaoxiang

tracks:
  track01:
    trackLabel: TRACK 01
    rfc: RFC-010
    title: 統一構文
    description: "ミニマリズムの哲学。変数から関数まで、すべての宣言は name: type = value パターンに従い、学習コストが低く、コードが一貫します。"
    features:
      - 構文宣言の究極的な統一
      - 型は第一級市民
  track02:
    rfc: RFC-011
    title: ゼロコストジェネリックス
    description: "ジェネリックスの特殊化はコンパイル時に完了し、型の抽象化はランタイムのオーバーヘッドを伴いません。コンパイル時の単相化。デッドコードの削除。型システムこそマクロ。"
  track03:
    rfc: RFC-009
    title: 所有権モデル
    description: "GCのストールにさよなら。YaoXiangはスコープベースの所有権モデルを採用し、メモリ安全性はコンパイル時に保証され、想定外はありません。"
    features:
      - 共有参照
      - 予測可能
      - GCストールなし
      - ライフタイムなし
  track04:
    trackLabel: TRACK 04
    title: 疎結合スケジューラ
    description: "マイコンから高性能サーバーまで、ランタイムが環境に自動適応します。異なるシナリオで異なるスケジューリング戦略を選択し、性能とリソースを両立します。"
    steps:
      - label: Embedded
        sub: "完全同期(Sync)"
      - label: Standard
        sub: "有向非巡回グラフ(DAG)と遅延評価に基づく自動並行管理"
      - label: Full
        sub: "ワークスティーリング機構(WorkSteal)"
  track05:
    title: 言語仕様 v1.8
    description: "シンタックスシュガーの氾濫を拒否。17個のキーワードで全機能を網羅し、複雑なシンタックスシュガーは不要、純粋な表現力だけが残ります。"

---
```