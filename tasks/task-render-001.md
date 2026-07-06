# タスク: render-001 — ブラウザ対話デモ（WASM・クリックで砂糖）

`loop-engineering-rules-v0.md` §11 のフォーマットに沿ったタスク文。
**前提**: `core-000`（決定論コア）完了。設計メモ §2 で予約された `render/` レイヤの初回実装。

---

## 位置づけ
決定論コアを WASM でブラウザ実行し、キャンバスに網の成長をリアルタイム描画、
**クリックで砂糖を置ける**対話デモを作る。`render/` は **State を読むだけ**・core に依存し
逆依存しない（設計メモ §2, 規約 §11）。プレイヤーの動詞は §0 の
「砂糖を置く／取り除く」「時間速度の変更」のみに限定する。

## task_id
`render-001`

## goal
`core` を `wasm32` にビルドし、ブラウザ上で `step` をアニメーション実行して trail 網を
canvas に描画。キャンバスのクリック座標をセルへ写像して `place_sugar`（右クリック等で
`remove_sugar`）を tick 境界で適用。再生/停止/速度（tick/s）の時間制御を付ける。
決定論コアは不変、描画・入力は非侵襲。

## acceptance_test（可能な範囲で headless 判定）
1. **WASM 決定性**: wasm ビルドで同一 `(seed, input_script, ticks)` の `final_state_hash` が
   **2回一致**（規約 §2 の「対象ビルドを固定して比較」。三角関数等の実装差により native と
   ビット一致する保証はないため、**同一 wasm ビルド内での再現性**を合格条件とする）。
2. **描画ON/OFF不変**: 描画・入力処理の呼び出し有無で `final_state_hash` が変わらない
   （render は State を読むだけ）。render 呼び出し前後で `state_hash` 不変。
3. **入力写像の純関数テスト**: キャンバス座標（+ズーム/オフセット）→ セル座標 →
   `place_sugar/remove_sugar` op への写像が純関数として正しい（陸/海判定・範囲外の扱いを
   含む）。native で headless にテスト可能。
4. **動詞の限定（§0）**: 公開操作は `place_sugar` / `remove_sugar` / 時間速度（再生・停止・
   tick/s）のみ。他の状態変更 API を露出しない。入力の乱数・壁時計を遷移に混入させない
   （砂糖 op は tick 境界で適用し決定性を保つ）。
5. **成果物**: `docs/` 配下に wasm + 最小 HTML/JS を出力し、GitHub Pages で読み込める。
   既存 [docs/index.html](../docs/index.html) からデモへ導線を張る。

> 視覚・操作感そのもの（アニメの見栄え等）は headless では判定せず**手動確認**とする
> （§7: 主観の自己申告は合格根拠にしない。合格根拠は上記の判定可能条件）。

## constraints
- **core を変更しない・逆依存禁止**（設計メモ §2）。core・決定性契約（§2）・不変条件（§3）は不変。
- **描画ON/OFFで `state_hash` 不変**。render/入力は副作用として State を書き換えない。
- **外部依存/ネットワーク（§11 の判断事項）**: 現行方針は「std のみ・外部クレートなし・
  ネットワーク不使用」。既定は **外部クレートなしの手書き wasm**（`wasm32-unknown-unknown`
  で関数を export、JS がリニアメモリを読んで canvas に描画）を推奨。`wasm-bindgen`/`web-sys`
  等の採用は外部依存追加＝**§8 で人間承認**が必要。
- 砂糖配置は既存 op（`place_sugar/remove_sugar`）を使い **tick 境界で適用**（同一操作列 →
  同一挙動）。しきい/パラメータは params 集約。

## seeds
決定性検証は `[1, 42, 1337]`（正準9本の一部）。

## 実装者(Claude Code)の裁量
- 描画方式（canvas 2D の `ImageData` 直書き／WebGL）、配色、ズーム/パン、時間速度 UI。
- wasm のメモリレイアウト・export 関数設計（決定性契約を破らない範囲で）。
- ビルド手順（`cargo build --target wasm32-unknown-unknown` + 手書き JS glue 等）。

## このタスクで意図的に「やらない」こと
- analysis のネットワーク/グラフ可視化（別タスク候補 `render-002`）。
- 適応則アニメ等 analysis の動的化。
- マルチプレイヤー・保存/共有・サーバ連携。
