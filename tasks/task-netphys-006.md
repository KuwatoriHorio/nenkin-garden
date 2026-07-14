# タスク: netphys-006 — forage に「蜘蛛の巣バイアス」（放射スポーク＋同心リング）を重ねる（B案）

`loop-engineering-rules-v0.md` §11 のフォーマットに沿ったタスク文（人間所有）。
**前提**: [netphys-001](task-netphys-001.md)〜[netphys-005](task-netphys-005.md) 完了・全テスト緑
（一般グラフ・扇状探索・anastomosis・周期 consolidation・標高忌避・高標高砂糖の見捨て）。

> **状態: 🅿 未着手（人間確認待ち）** — §8.1 に従い、タスク切り出しのみ。実装は人間の合図を待つ。

## ⚑ 人間確定事項（2026-07-12・AskUserQuestion）
- **B案 = forage に蜘蛛の巣バイアスを重ねる**（純粋な幾何学モード＝A案ではない）。餌探索（砂糖誘引）・標高忌避は
  **残したまま**、放射スポーク＋同心リングの"癖"を重ね、餌や地形で歪んだ**半対称の網**にする。
- 実際の粘菌は蜘蛛の巣を作らない → これは**様式化した別バリアント**（観察デモの見た目の選択肢）。

---

## 背景
現状の netphys 探索は「ホームから外向きに扇状 probe → 衝突で anastomosis 融合 → 周期 consolidation」で、
放射状の素地はあるがリング（同心円の横糸）は狙っていない。ユーザー要望（2026-07-12）: **同心円＋放射線を組み合わせた
蜘蛛の巣のように広がる動き**。B案として、餌探索を主に残しつつ蜘蛛の巣構造のバイアスを重ねる。

## task_id
`netphys-006`

## goal
`netphys_step` に **(1) 放射スポークバイアス** と **(2) 同心リング形成** の2機構を**加算的に**導入し、
既存の砂糖誘引・標高忌避・扇状拡散と**併存**させて、蜘蛛の巣状（放射＋同心）に広がる網を作れるようにする。
**新パラメータは既定オフ**（`NetParams::default()` は現状の挙動＝netphys-001〜005 の全テスト緑を維持）。
蜘蛛の巣は**新パラメータを有効化した構成でのみ**発現し、その構成で放射・リング・forage 併存を headless で実証する。
決定論・保存則・有界・ソフト（壁を作らない）を保つ。

## 実装方針（`src/netphys/step.rs`・`src/netphys/state.rs`・加算的・既定オフ）
- **(1) 放射スポークバイアス** `w_radial`（既定 0＝オフ）:
  - 探索の中心方向ブレンド（誘引 + `w_rand` + `w_elev`）に、**ホームから外向きの半径方向**への項
    `+ w_radial * r_hat`（`r_hat` = (node − home) の単位ベクトル）を加算。角度方向の揺れが抑えられ数本のスポークになる。
  - 既存 `w_rand`/`w_elev`/`fan_count` は残す（併用）。`w_radial=0` で従来と厳密一致（後方互換）。
- **(2) 同心リング形成** `ring_period`（既定 0＝オフ）・`ring_reach`（接線 probe の距離）:
  - `ring_period>0` のとき、**周期 `ring_period` tick ごと**に各前線ノードから**接線（円周）方向**（`r_hat` に直交、両側）へ
    `ring_reach` 距離の probe を出し、**近傍(fusion_dist 以内)の別スポークのノードと融合**して横糸（リング＝ループ）を作る。
  - 決定論厳守: 接線 probe は**乱数を使わない算術**（`r_hat` 回転±90°）。id 昇順・単一 PRNG の引き順を崩さない。
    リング辺も既存の質量会計（`d0`・`ΣD·L`）に従い free_budget から支弁（`total_mass == collected − consumed` 維持）。
  - ソフト: 接線 probe も海/範囲外はハード棄却（従来どおり）、標高は既存コスト/バイアスのまま（壁は作らない）。
- **home = 放射中心**: ノード0（`default_home`）を中心とする。`r_hat` は各ノードで (x−hx, y−hy) から算出（ゼロ長時は無バイアス）。

## acceptance_test（headless・複数シード中央値・S9 部分集合 [1,42,1337]・新規 `tests/netphys_006.rs`）
**蜘蛛の巣構成** `web = NetParams::default()` に `w_radial>0`・`ring_period>0`・`ring_reach>0` を設定したもので判定。
1. **放射スポーク（web on）**: web 構成で、ノードの成長方向が**半径方向に整列**する度合いが、web off（`w_radial=0`）より
   **中央値で有意に高い**。指標例: 各新規辺の方向と `r_hat` の内積 |cos| の平均、または「角度ビンあたりの半径到達長」の
   集中度。§7 exemplary の red→green: `w_radial=0` で整列低い（赤）→ web 既定で高い（緑）。同一 world/seed で対比。
2. **同心リング（web on）**: web 構成で、**円周方向の辺（リング辺）**が web off より**中央値で有意に多い**。
   リング辺の定義例: 両端の半径差 `|r_a − r_b|` が小さく（同心帯）かつ角度差が一定以上（別スポーク間）を繋ぐ辺。
   併せて**冗長度>1（ループを持つ）**。red→green: `ring_period=0`(リングなし=赤)→ web 既定(緑)。
3. **forage 併存（B案の核）**: web 構成でも **低〜中標高(e<attract_e_hi)の砂糖は連結する**（netphys-005 accept2 と同趣旨）
   ＝蜘蛛の巣バイアスが餌探索を壊していないこと。加えて **高標高砂糖の見捨て（netphys-005 accept1 相当）が web 構成でも
   中央値で保たれる**（標高忌避が残っている＝B案）。
- **回帰（必須）**: `NetParams::default()`（web off）で netphys-001 ①②⑤⑥・002 ③④・003 accept1・004・005 が**緑のまま**。
  Jones/tree/analysis/render-wasm 全緑。決定性 hash 契約維持。保存則・有界（cap 内・爆発しない）を保つ。
- パラメータ（`w_radial`・`ring_period`・`ring_reach`）は**探索用一時テスト**で実測 → 決定後に削除。まず①②の赤を確認してから実装。

## constraints
- **編集可**: `src/netphys/{state,step}.rs`（放射バイアス・リング機構・`NetParams` 追加＝**既定オフ**）、
  新規 `tests/netphys_006.rs`、`src/bin/run_netphys.rs`（任意）。
- **編集不可（人間所有・§7/§11）**: netphys-001〜005 の既存アサート（弱めない・**既定オフで全緑維持**）、Jones/tree のコード・
  受け入れ・ゴールデン、`test_harness_001.rs`/`metric_thresholds_001.rs`、設計軸 §0（決定論・保存則・ソフト＝壁禁止）。
- **決定性**: 放射/リングとも単一 PRNG・id 昇順。接線 probe は乱数不使用の算術。壁時計・ハッシュ反復順を使わない。
- **保存則・有界・過学習禁止**（複数シード中央値・tick/seed/cap を通すために縮小しない。リング辺で cap を突くなら §8）。

## seeds
`[1, 42, 1337]`（可能なら S9 中央値）

## §8 停止の指針（このタスク特有）
- 放射スポーク（角度集中）と netphys-004 の扇状拡散（角度分散）が本質的に綱引き → web 構成でも既定オフの netphys-004
  accept1 は緑のはずだが、もし両立できない設計問題なら弱めず停止。
- リング形成で cap を突く／保存則が締まらない／決定性が崩れる → 停止。
- 蜘蛛の巣バイアスを効かせると forage（餌連結）や標高忌避が壊れる（B案が成立しない＝実質A案化）→ 停止・相談。
- モデル昇格が要ると感じたら勝手に上げず停止（人間承認事項）。

## このタスクでやらないこと
- 既定を web on にする（既定はオフ＝既存挙動維持）。demo での有効化（トグル/スライダー露出）は後続 render-net タスク。
- A案（餌・標高を無視する純粋な幾何学モード）。あくまで forage 併存の B案。
- Jones/tree 変更・担体 A→β 切替。demo/wasm 反映は keep 後にオーケストレーターが別途。

## 関連
- モデル: [src/netphys/step.rs](../src/netphys/step.rs)（`attractor_dir`・扇状 `fan_offsets`・anastomosis・consolidation）、
  [src/netphys/state.rs](../src/netphys/state.rs)（`NetParams`）、[netphys-004](task-netphys-004.md)（扇状拡散＝角度分散）、
  [netphys-005](task-netphys-005.md)（標高忌避・高標高砂糖の見捨て）。
