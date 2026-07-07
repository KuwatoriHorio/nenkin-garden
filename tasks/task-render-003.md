# タスク: render-003 — WASM デモに density⇔graph 表示トグル

`loop-engineering-rules-v0.md` §11 のフォーマットに沿ったタスク文。
**前提**: `render-001`（WASM 対話デモ）と `render-002`（グラフ SVG）完了。

---

## 位置づけ
ブラウザの対話デモ（render-001）に「密度ビュー ⇔ グラフビュー」の表示トグルを追加し、
ライブでグラフ（幹線=流量太さ / MST 実線・冗長辺 破線 / 連結成分 色分け / source·sink）を
見られるようにする。グラフは Rust 側でラスタライズせず、**幾何データを JS に渡して canvas 2D で描く**。
core・analysis は読むだけ・非侵襲。

## task_id
`render-003`

## goal
`render-wasm` の `Sim` にグラフ幾何（ノード座標・エッジ・エッジ電流・MST 判定・成分）を
公開するアクセサを追加し、デモ HTML にトグルを付けてグラフを canvas に描画する。
表示切替はシミュレーションを一切変更しない（描画のみ）。

## acceptance_test（判定可能な範囲）
1. **グラフ幾何の決定性**: 同一 State から `compute_graph` 後のノード数・エッジ配列が2回一致
   （同一シミュ状態→同一グラフ）。native test。
2. **非侵襲**: `compute_graph`/描画の前後で `state_hash` 不変（render は読むだけ）。
3. **整合**: 公開エッジ数が `analysis` の `edges` と一致、電流配列長がエッジ数と一致。
4. **動詞不変（§0）**: トグルは表示切替のみで、公開する操作は砂糖 place/remove・時間速度のまま。
5. 既存 `render-wasm`・`analysis`・core テストが全緑。デモが density/graph 両方を表示（手動確認）。

## constraints
- **core を変更しない・逆依存禁止**。State 読み取り専用・決定性維持（§2）。
- グラフ幾何は既存 `analysis::analyze` と `graph_svg`（`flow_width`/`mst_edge_set` の pub 化）を
  再利用。新しい解析指標は追加しない。
- グラフ再計算は毎フレームでなくてよい（JS 側で throttle 可）。決定性は「同一 State→同一幾何」で担保。

## seeds
`[42]` 等（幾何の決定性・整合の検証）。

## このタスクで意図的に「やらない」こと
- 力学レイアウト（ノードはセル座標のまま）・実地形・グラフ編集 UI。
