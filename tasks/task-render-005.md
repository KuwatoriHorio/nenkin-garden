# タスク: render-005 — エージェント可視化・trail非表示・砂糖量/回収レートの調整

`loop-engineering-rules-v0.md` §11 のフォーマットに沿ったタスク文（人間所有）。
**前提**: `core-000/001/002` 完了。`render-001〜004` 完了（対話デモ稼働）。
関連: [core-003](task-core-003.md)（枯渇砂糖の自動削除・独立だが同時期の要望）。

> **状態: 🔵 未着手（§8.1 で人間確認待ち）**

---

## 背景
現状デモは trail 場（緑）を描くだけで、**実際の生物＝エージェント（`ax/ay`）を描いていない**。
ユーザー要望（2026-07-07）:
1. **エージェントを可視化**（歩き手を点で見せる）。
2. 逆に**誘引物質(trail)を不可視にできる**ようにする。
3. **見え方を確認しながら、砂糖の初期量（strength）とバイオマス増加量（collect_rate）を調整**したい。

## task_id
`render-005`

## goal
対話デモで (a) エージェント位置を点で可視化、(b) trail 密度表示の on/off 切替、
(c) 砂糖の初期量 `strength` と回収レート `collect_rate` をスライダーで実行時調整して、
見え方を見ながらチューニングできるようにする。**core の力学は変更しない**
（render は State を読むだけ＋パラメータの実行時 setter。既定値は変えない）。

## acceptance_test
1. **render-wasm 露出**: エージェント位置アクセサ `agent_positions()`（flat [x,y,...]・読み取り・
   非侵襲）と `set_collect_rate(f64)` セッターを追加。native test で
   「位置数 == n_agents」「取得前後で state_hash 不変（非侵襲）」「setter が params.collect_rate を変える」を検証。
2. **デモ UI**: エージェント点描画、trail 表示トグル（既定=エージェント表示・trail オフ）、
   `strength` と `collect_rate` のスライダー（現在値表示つき）。既存の砂糖 置く/取る・
   density⇔graph・採餌トグル・速度は従来どおり機能。
3. **ブラウザ実測**: (a) エージェントが点として見える、(b) trail を消せる、
   (c) スライダー変更で挙動（回収の速さ・網の育ち）が変わる、をコンソールエラー無しで確認。
4. **リグレッション**: core + render-wasm の全テストが緑。wasm/JS glue を再生成しライブ整合。

## constraints
- **編集可**: `render-wasm/src/lib.rs`（読み取りアクセサ＋実行時 setter）、`docs/demo/index.html`、
  生成物（wasm/js glue）。
- **編集不可（人間所有・§7/§11）**: core の不変条件・ゴールデン・受け入れテスト、設計軸（§0）。
- **core の力学非変更**: render は State を読むだけ。`set_collect_rate` は**実行中 Sim の params を
  変えるだけ**で、`params.rs` の**既定値は変更しない**（恒久的な既定変更は別の承認済みゴールデン更新）。
- **§0 の動詞との関係（要確認）**: プレイヤーの正規動詞は「砂糖 置く/取る」「時間速度」のみ。
  strength/collect_rate スライダーは**開発用チューニング・パネル**であって新しいプレイ動詞ではない、
  という位置づけで実装する（UI 上も区別）。この解釈でよいか §8.1 の確認で人間に諮る。
- **決定性の注記**: スライダーで実行中に params を変えるとその後の展開は変わるが、これは
  対話入力の一部（golden 対象外のデモ）。core の決定性契約（同一 params・同一入力→同一hash）自体は不変。

## このタスクでやらないこと
- 調整して決めた値を `params.rs` の既定や demo 既定に**恒久反映すること**。まずは
  「見ながら探る」手段を作る。良い値が決まったら、既定 collect_rate の変更（＝保護された
  baseline のゴールデン更新）や demo 既定の更新を**別ステップで人間承認の上**行う。
- エージェントの向きや軌跡の凝った描画。まずは位置を点で。数百体の密集表現は最小限で可。

## 関連メモ（実装の当たり）
- エージェント位置: `State.ax/ay`（[src/state.rs](../src/state.rs)）。render-wasm は既に
  `sugar_positions()` を持つ（[render-wasm/src/lib.rs](../render-wasm/src/lib.rs)）ので同型で追加。
- trail 描画は `Sim::render()`＋`drawDensity()`。トグルで密度描画をスキップし背景（陸/海）＋
  エージェント点＋砂糖のみ描く構成に。
