# タスク: render-tree-002 — 樹木デモに探索（ランダム伸長 w_rand）を露出

`loop-engineering-rules-v0.md` §11 のフォーマットに沿ったタスク文（人間所有）。
**前提**: [tree-growth-002](task-tree-growth-002.md)（`w_rand` によるランダム探索伸長・既定オフ）完了・全テスト緑。
[render-tree-001](task-render-tree-001.md)（`TreeSim`・`docs/demo-tree/`）完了。

> **状態: ✅ 完了**（2026-07-09, iter:24）— `TreeSim` に `set_w_rand`/`set_explore_persistence`（実行中
> params のみ・既定不変）を追加、`docs/demo-tree/` に探索強度 w_rand スライダー（id=wrand・0〜1・
> step0.05・初期0.3）を実装（fresh で再適用）。render-wasm 15テスト緑（新規2）。木モデル力学・
> TreeParams 既定・Jones core/デモは無変更。ブラウザ実測: w_rand=0.3 砂糖なしで彷徨い/0で育たず/
> 砂糖で誘引支配到達・エラー無し。実装 nenkin-implementer(sonnet)、ブラウザ検証・keep はオーケストレーター(opus)。

---

## 背景
tree-growth-002 で「砂糖なしでもランダムに探索伸長し、近くの砂糖では誘引が支配」する挙動を
`w_rand>0`（既定オフ）で実装した。ただし `TreeSim`（wasm）は `TreeParams::default()`＝探索オフで
動くため、デモ `docs/demo-tree/` では**まだ探索が見えない**。ユーザー要望（2026-07-09）: デモで
探索（彷徨い）を見せる。

## task_id
`render-tree-002`

## goal
樹木デモで探索の強さ `w_rand` を実行時に調整でき、砂糖が無くても木がランダムに彷徨って広がり、
砂糖を置くと誘引が支配して到達する様子を見られるようにする。**木モデルの力学（`tree_step`）・
`TreeParams` 既定値は変えない**（render は駆動して読む＋実行時 setter のみ・一方向依存）。

## 実装方針
- **render-wasm/src/lib.rs（`TreeSim` に実行時 setter を追加・読むだけ＋params書換のみ）**:
  - `set_w_rand(&mut self, v: f64)`（実行中 `TreeSim` の `params.w_rand` を更新。`state.rs` の既定は不変）。
  - 任意で `set_explore_persistence(&mut self, v: f64)`（既定 0.45 の微調整用）。
  - native test: setter が params を変えること・非侵襲（アクセサは State を書き換えない）。既存 `Sim`/`TreeSim` テストは不変で緑。
- **docs/demo-tree/index.html**:
  - **探索強度 `w_rand` スライダー**（現在値表示つき）。値変更で即 `sim.set_w_rand(v)`。`fresh()`（reset/seed 変更）でも現在値を再適用。
  - デモを開いた時点で探索が見えるよう、**採用済みの妥当値（w_rand=0.3 付近）を初期プリセット**にしてよい
    （スライダー 0 で探索オフ＝誘引のみに戻せる）。
  - 既存の描画（発光する枝＋ノード）・砂糖 置く/取る・再生/速度/reset/seed は従来どおり。
- 木モデル（`src/tree`）・Jones core・`docs/demo/`（Jones デモ）は変更しない。

## acceptance_test
1. **露出**: `TreeSim::set_w_rand`（＋任意で set_explore_persistence）を追加。native test で
   setter が効くこと・非侵襲を検証。既存の TreeSim/Sim native test は不変で緑。
2. **デモ UI**: `w_rand` スライダーがあり、値変更で挙動が変わる。0 で探索オフ（誘引のみ）、>0 で砂糖なしでも彷徨う。
3. **ブラウザ実測**: (a) w_rand>0・砂糖なしで木がランダムに伸びて広がる、(b) 砂糖を置くと誘引が支配して到達、
   (c) スライダーを 0 にすると探索が止まる（誘引のみ）、をコンソールエラー無し・過負荷なしで確認。
4. **リグレッション**: core(Jones)+src/tree+render-wasm 全テスト緑。wasm/JS glue 再生成しライブ整合。
   既存 Jones デモ `docs/demo/` は無変更・従来どおり動く。

## constraints
- **編集可**: `render-wasm/src/lib.rs`（`TreeSim` に実行時 setter）、`docs/demo-tree/index.html`、生成物
  （`docs/demo-tree/` の wasm/js glue）。
- **編集不可（人間所有・§7/§11）**: Jones core の不変条件・ゴールデン・受け入れ、`src/tree` の**モデル力学**
  と `TreeParams` の**既定値**（`w_rand` 既定 0.0 は不変）、設計軸（§0）。
- **一方向依存**: render は `src/tree` を駆動して読む＋実行中 params の setter のみ。決定性契約
  （同一 seed・入力・params → 同一 tree_state_hash）を壊さない。
- §0 の動詞不変（w_rand スライダーは開発用チューニング・見え方確認用であってプレイ動詞ではない）。

## このタスクでやらないこと
- `w_rand` を `TreeParams` の**既定に恒久反映**すること（既定 0.0 のまま。既定化は別ステップ・人間承認）。
- 木モデルの成長規則変更（tree-growth 系）。今回は露出のみ。

## 関連
- モデル: [tree-growth-002](task-tree-growth-002.md)（`w_rand`/`explore_persistence`）。
- デモ/wasm: [render-tree-001](task-render-tree-001.md)（`TreeSim`・`docs/demo-tree/`）。既存 Jones の
  setter 露出パターン（`Sim::set_collect_rate`/`set_trail_max`, render-005/006）に倣う。
