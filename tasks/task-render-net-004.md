# タスク: render-net-004 — 網 Physarum デモに蜘蛛の巣バイアス（放射＋同心リング）を露出

`loop-engineering-rules-v0.md` §11 のフォーマットに沿ったタスク文（人間所有）。
**前提**: [netphys-006](task-netphys-006.md)（`w_radial`・`ring_period`・`ring_reach` を `netphys_step` に実装済み・
**既定オフ**）／[render-net-002](task-render-net-002.md)（`set_period_n`）・[render-net-003](task-render-net-003.md)
（`set_w_elev`）＝露出の型。完了・全テスト緑。

> **状態: 🅿 未着手（人間確認待ち）** — §8.1 に従い、タスク切り出しのみ。実装は人間の合図を待つ。

---

## 背景
netphys-006 で forage に蜘蛛の巣バイアス（放射スポーク `w_radial`＋同心リング `ring_period`/`ring_reach`）を
実装したが、**新パラメータは既定オフ**なので現在のデモには蜘蛛の巣が出ない。ユーザー要望（2026-07-12）:
デモで蜘蛛の巣（同心円＋放射線）を**見られるようにしたい**。この3パラメータをライブ調整できるよう露出する。

## task_id
`render-net-004`

## goal
`render-wasm` の `NetSim` に `set_w_radial`・`set_ring_period`・`set_ring_reach` を追加（`NetParams` を読み替える
だけ・非侵襲）し、`docs/demo-net/` にスライダーを追加して**蜘蛛の巣（放射＋同心リング）をライブで発現・調整**できる
ようにする。**netphys の力学・`NetParams` 既定（すべてオフ）は変えない**（露出のみ・一方向依存）。既存 Jones/tree
デモ・netphys モデルは無変更。

## 決定が要る点（§8.1 で確認）
- **デモ初期表示値**: 「蜘蛛の巣を見たい」を満たすため、提案 **初期＝web 構成**（`w_radial=2.0, ring_period=6,
  ring_reach=10`＝起動時から蜘蛛の巣が出る）。※これは**デモ UI の初期値**で、`NetParams` 既定（全オフ・テスト/
  ヘッドレスの正）は**変えない**。起動時に各 setter を呼んで反映。初期を全オフ（0）にしてユーザーが上げる方式も可。
- **スライダー可動域（提案）**: `w_radial` 0〜8 (step0.5)、`ring_period` 0〜60 (整数, 0=リングoff)、`ring_reach` 1〜30 (step1)。

## 実装方針
- **`render-wasm` の setter 3種**（`set_period_n`/`set_w_elev` と同型・保持 `NetParams` を書き換えるだけ）:
  - `set_w_radial(v: f64)`: `params.w_radial = v.clamp(0.0, 8.0)`。
  - `set_ring_period(v: f64)`: `params.ring_period = v.round().clamp(0.0, 60.0) as u64`（0=リングoff）。
  - `set_ring_reach(v: f64)`: `params.ring_reach = v.clamp(1.0, 30.0)`。
  - **モデル力学・`NetParams` 既定は変えない**（読み替えのみ・次 step から反映）。
  - native test（既存 `set_*` テストと同型）: 各 setter が対応フィールドを更新・**非侵襲（step 前は hash 不変）**・
    クランプ・**決定性（同一 seed・操作列・パラメータ → 同一 `net_state_hash`）**。既存 `Sim`/`TreeSim`/`NetSim` native test は不変で緑。
- **`docs/demo-net/index.html` のスライダー**: 「標高忌避」の隣に「蜘蛛の巣」系3スライダー（放射 `w_radial`・
  リング周期 `ring_period`・リング距離 `ring_reach`）＋現在値表示。`input` で各 setter を呼ぶ。
  - **reset/seed 再生成時もスライダー値を保持**（render-net-002/003 と同じ挙動＝現在値を新 sim に各 setter で適用。
    初回のみ初期値）。ラベルは「放射スポーク／同心リング（0でオフ）」等わかりやすく。
- 決定性契約は「同一 seed・同一入力列（web パラメータ変更も入力の一部）→同一 `net_state_hash`」を保つ（壁時計を状態遷移に入れない）。

## acceptance_test
1. **setter 露出**: `NetSim` に `set_w_radial`/`set_ring_period`/`set_ring_reach` を追加。native test で更新・非侵襲・
   クランプ・決定性。既存 `Sim`/`TreeSim`/`NetSim` native test は不変で緑。
2. **デモ**: `docs/demo-net/` に3スライダーが出て、放射・リングを動かすと蜘蛛の巣（放射スポーク＋同心リング）が
   発現/変化する。reset/seed で値を保持。
3. **ブラウザ実測**: スライダー変更が各 setter を呼び反映されること・エラー無しを確認。可能なら「web 構成でリング辺
   （円周方向）や放射整列が増える」傾向を NetSim 駆動で数値確認。
4. **リグレッション**: core(Jones)+tree+netphys+render-wasm 全テスト緑。wasm/JS glue 再生成しライブ整合。
   既存デモ `docs/demo/`・`docs/demo-tree/` は無変更。netphys モデル力学・`NetParams` 既定（全オフ）は無変更。

## constraints
- **編集可**: `render-wasm/src/lib.rs`（setter 3種追加＝params 読み替えのみ）、`docs/demo-net/*`（スライダー）、生成物。
- **編集不可（人間所有・§7/§11）**: netphys のモデル力学（`netphys_step`）と **`NetParams` 既定値**、Jones/tree のコード・
  受け入れ・ゴールデン、`tests/netphys_00x.rs` のアサート、設計軸（§0）。
- **一方向依存**: render は `src/netphys` を駆動して読むのみ。決定性契約を壊さない。§0 の動詞不変（砂糖 置く/取る・時間）。
  web パラメータは観察用コントロール（力学の質は変えない）。

## このタスクでやらないこと
- netphys のモデル力学変更・`NetParams` 既定変更（露出のみ）。
- 他パラメータ（fan_count/tero 係数 等）の同時露出（必要なら別タスク）。
- Jones/tree デモへの反映。

## 関連
- 露出の型: `NetSim::set_period_n`（[render-net-002](task-render-net-002.md)）・`set_w_elev`（[render-net-003](task-render-net-003.md)）。
- モデル: [src/netphys/step.rs](../src/netphys/step.rs)（放射バイアス・`phase2_ring`）、
  [src/netphys/state.rs](../src/netphys/state.rs)（`w_radial`/`ring_period`/`ring_reach`）、[netphys-006](task-netphys-006.md)。
