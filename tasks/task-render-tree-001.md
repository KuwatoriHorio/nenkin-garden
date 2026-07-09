# タスク: render-tree-001 — 成長木モデルのブラウザ対話デモ（別ページ）

`loop-engineering-rules-v0.md` §11 のフォーマットに沿ったタスク文（人間所有）。
**前提**: [tree-growth-001](task-tree-growth-001.md)（α成長木モデル・ヘッドレス）完了・全テスト緑。
`render-001〜007`（Jones デモ・render-wasm 基盤）完了。

> **状態: 🔵 未着手（§8.1 で人間確認待ち）**

---

## 背景
tree-growth-001 で「木（親子パスのみ・閉路なし）を砂糖への space colonization で成長させる」新モデルを
ヘッドレスで実装した。木構造は本来**樹状**なので、親子パスをそのまま枝として描けば
「砂糖へ伸び・枝分かれ・餌喪失で退縮」する木をブラウザで観察できる。
ユーザー要望（2026-07-09）: この木モデルの描画（対話デモ）を作る。

## task_id
`render-tree-001`

## goal
成長木モデル（`src/tree/`）を wasm で実行し、**木（ノード＋親子パス）を有機的な発光する枝**として
描くブラウザ対話デモを作る。クリックで砂糖を置くと木がそちらへ伸び・枝分かれし、取り除くと退縮する。
**現行 Jones デモ・core は変更しない**（別ページ・独立）。render は木モデルを**駆動して読むだけ**
（core←render の一方向依存を維持）。

## 実装方針
- **別ページ `docs/demo-tree/index.html`**（推奨）: 既存 `docs/demo/`（Jones）を壊さず、樹木モデル専用の
  対話デモを新設する。トップページ `docs/index.html` から導線を1つ追加。
  （代替案: 既存デモにモデル切替トグルを足す統合案もあるが、実験的な別モデルなので分離を推奨。
  §8.1 の確認で人間が方針を選べる。）
- **render-wasm に `TreeSim`**（wasm_bindgen・`Sim` と別struct）: 内部に `TreeState`＋`TreeParams`＋`World`
  を持ち、`tree_step` で駆動する（render レイヤがモデルを回す＝`Sim` と同様、モデル力学は `src/tree/`
  のまま・改変しない）。公開メソッド:
  - `new_tree(seed) -> TreeSim`、`step()`（保留砂糖 op を tick 境界適用）、`tick()`、`home_x/home_y()`。
  - `place_sugar_at_canvas / remove_sugar_at_canvas`（既存 `Sim` と同型・陸のみ）、`sugar_positions()`。
  - `tree_nodes() -> Vec<f32>`（flat [x0,y0,...] グリッド座標）、
    `tree_edges() -> Vec<u32>`（flat [child,parent,...] の index ペア）。読み取り専用アクセサ。
  - `tree_state_hash_hex() -> String`（決定性検証用）。
  - native test: 「同一 seed・同一操作列 → 同一 tree_state_hash（決定性）」「アクセサ長の整合
    （edges 長 == 2×(nodes数−根1)）」等。
- **描画（demo-tree の JS）**: `tree_edges()` の親子パスを**発光する枝**（`globalCompositeOperation='lighter'`・
  太さ/α で有機的に。render-007 のニューロン描画に準じる）、`tree_nodes()` を発光ノードとして描く。
  砂糖は赤点、ホームは印。背景は陸/海（`Sim` の density 描画は使わず、木モデルには trail 場が無いので
  land/sea 色だけ描くか、簡素な背景でよい）。再生/停止・速度・reset・seed、クリック=砂糖設置/右クリック=除去。
- 性能: ノード数は成長で増える。枝描画は素直に O(nodes)。throttle は不要か軽微でよい。

## acceptance_test
1. **render-wasm 露出**: `TreeSim` と上記アクセサを追加。native test で決定性（同一操作列→同一hash）と
   アクセサ整合を検証。既存 `Sim`（Jones）native test は不変で緑。
2. **デモ**: `docs/demo-tree/` が木を発光する枝で描画し、クリックで砂糖設置→木が伸長、
   2箇所置くと枝分かれ、右クリック除去で退縮。再生/停止/速度/reset/seed が機能。
3. **ブラウザ実測**: 上記挙動（伸び・分岐・縮み）とエラー無し・過負荷なしを確認。
4. **リグレッション**: core（Jones）+ src/tree + render-wasm 全テスト緑。wasm/JS glue 再生成しライブ整合。
   既存 Jones デモ `docs/demo/` は無変更・従来どおり動く。

## constraints
- **編集可**: `render-wasm/src/lib.rs`（`TreeSim` 追加＝木モデルを駆動して読む）、
  `docs/demo-tree/*`（新規）、`docs/index.html`（導線1つ追加）、生成物。
- **編集不可（人間所有・§7/§11）**: Jones core の不変条件・ゴールデン・受け入れテスト、設計軸（§0）、
  `src/tree/` のモデル力学（`tree_step` のロジックは変えない＝描画のために回すだけ）。
- **一方向依存**: render は `src/tree` を読む/駆動するのみ。木モデルの決定性契約（同一 seed・入力→
  同一 hash）を壊さない。
- §0 の動詞不変（砂糖 置く/取る・時間）。

## このタスクでやらないこと
- 木モデルの力学（成長規則）変更。今回は描画のみ。挙動を変えたくなったら別タスク（tree-growth-00x）で。
- Jones デモとの統合トグル（分離ページで進める。統合したくなったら別途）。
- β 要素や閉路（木のまま）。

## 関連
- モデル: [src/tree/](../src/tree/)（`tree_step`/`TreeState`/`run_tree_headless`）、[tree-growth-001](task-tree-growth-001.md)。
- 描画の参考: 既存 `Sim`（[render-wasm/src/lib.rs](../render-wasm/src/lib.rs)）・render-007 のニューロン枝描画。
