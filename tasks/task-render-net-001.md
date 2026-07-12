# タスク: render-net-001 — 網 Physarum モデルのブラウザ対話デモ（別ページ）

`loop-engineering-rules-v0.md` §11 のフォーマットに沿ったタスク文（人間所有）。
**前提**: [netphys-001](task-netphys-001.md)（網 Physarum・Stage 1・ヘッドレス）完了・全テスト緑。
`render-tree-001`（`TreeSim`・`docs/demo-tree/`）完了（wasm 別モデル露出の型）。

> **状態: 🔵 未着手（§8.1 で人間確認待ち）**

---

## 背景
netphys-001 で「扇状探索→衝突で網化(ループ)→周期 consolidation で Kirchhoff＋Tero 刈り込み→餌を結ぶ
連結網」をヘッドレスで実装した（担体A・辺に太さ D）。木と違い**ループを持つ網**で、辺の**コンダクタンス
D＝管の太さ**が Tero 刈り込みで背骨に集約される。これをブラウザで見せると Physarum らしさが出る。
ユーザー要望（2026-07-11）: 網 Physarum の可視化（対話デモ）。

## task_id
`render-net-001`

## goal
網 Physarum モデル（`src/netphys/`）を wasm で実行し、**ノード＋辺（管）をブラウザで描く**別ページ対話デモ。
クリックで砂糖を置くと網が広がり衝突してループを作り、周期 consolidation で**流れの多い管が太く残り**
細い管が刈られる様子を観察できる。**現行 Jones/tree デモ・netphys モデル力学は変更しない**
（render は netphys を駆動して読むだけ・一方向依存）。

## 実装方針
- **別ページ `docs/demo-net/index.html`**（推奨）: 既存 `docs/demo/`（Jones）・`docs/demo-tree/`（木）を
  壊さず新設。トップ `docs/index.html` から導線を1つ追加。
- **render-wasm に `NetSim`**（`Sim`/`TreeSim` と別struct・netphys を駆動して読む）:
  - `new_net(seed)`、`step()`（保留砂糖 op を tick 境界適用）、`tick()`、`width()/height()`、
    `place_sugar_at_canvas/remove_sugar_at_canvas`（陸のみ・既存と同型）、`sugar_positions()`。
  - `net_nodes() -> Vec<f32>`（flat [x,y,...]）、`net_edges() -> Vec<u32>`（flat [a,b,...] index ペア）、
    `net_edge_widths() -> Vec<f32>`（各辺のコンダクタンス D＝管の太さ・gedges と同順）。
  - `net_state_hash_hex()`、背景描画 `render()`＋`pixels_ptr/len`（land/sea 地形色のみ・trail 無し。既存 `land_color` 流用可）。
  - native test: 「同一 seed・同一操作列 → 同一 net_state_hash（決定性）」「edge_widths 長 == edges 長/2」。既存 Sim/TreeSim native test は不変で緑。
- **描画（demo-net の JS）**: 背景（land/sea）に、**辺を管として描く（線幅を D で決める＝Tero の太さが見える）・
  加算合成の発光**（render-007 のニューロン描画に準じる）、ノードを発光点、砂糖を赤点。**ループ（網）が
  見える**こと。コントロール: 再生/停止・速度・reset・seed、左クリック=砂糖/右クリック=除去。
- 性能: ノード/辺は netphys の cap 内なので素直に O(edges) 描画で可。

## acceptance_test
1. **render-wasm 露出**: `NetSim` と上記アクセサ（`net_nodes`/`net_edges`/`net_edge_widths`/`net_state_hash_hex`）を追加。
   native test で決定性・アクセサ整合（edge_widths 長 == edges/2）。既存 `Sim`/`TreeSim` native test は不変で緑。
2. **デモ**: `docs/demo-net/` が網（ノード＋太さ D の管）を描画。クリックで砂糖→網が広がりループ形成、
   周期 consolidation で背骨が太く残り細管が刈られる。再生/停止/速度/reset/seed 機能。
3. **ブラウザ実測**: 上記（網化・ループ・餌を結ぶ・consolidation での太さ変化）とエラー無し・過負荷なしを確認。
4. **リグレッション**: core(Jones)+tree+netphys+render-wasm 全テスト緑。wasm/JS glue 再生成しライブ整合。
   既存デモ `docs/demo/`・`docs/demo-tree/` は無変更・従来どおり動く。

## constraints
- **編集可**: `render-wasm/src/lib.rs`（`NetSim` 追加＝netphys を駆動して読む）、`docs/demo-net/*`（新規）、
  `docs/index.html`（導線1つ）、生成物。
- **編集不可（人間所有・§7/§11）**: Jones core・tree・**netphys のモデル力学（`netphys_step`）と `NetParams` 既定**、
  既存の不変条件・ゴールデン・受け入れテスト、設計軸（§0）。
- **一方向依存**: render は `src/netphys` を駆動して読むのみ。決定性契約（同一 seed・入力→同一 net_state_hash）を壊さない。
- §0 の動詞不変（砂糖 置く/取る・時間）。

## このタスクでやらないこと
- netphys のモデル力学変更（Stage 2 = netphys-002 は別）。今回は描画のみ。
- Jones/tree デモとの統合トグル（別ページで進める）。

## 関連
- モデル: [src/netphys/](../src/netphys/)（`netphys_step`/`NetState`/`net_edge_widths`＝D）、[netphys-001](task-netphys-001.md)。
- 描画/wasm の型: `TreeSim`／`docs/demo-tree/`（[render-tree-001](task-render-tree-001.md)）、render-007 の発光枝描画。
