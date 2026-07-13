# タスク: render-net-003 — 網 Physarum デモに標高忌避の強さ `w_elev` スライダーを露出

`loop-engineering-rules-v0.md` §11 のフォーマットに沿ったタスク文（人間所有）。
**前提**: [netphys-003](task-netphys-003.md)（標高方向バイアス `w_elev` を `netphys_step` に実装済み・既定 2.0）／
[render-net-002](task-render-net-002.md)（`NetSim::set_period_n` スライダー＝露出の型）完了・全テスト緑。

> **状態: 🅿 未着手（人間確認待ち）** — §8.1 に従い、タスク切り出しのみ。実装は人間の合図を待つ。

---

## 背景
ユーザー要望（2026-07-12）: 網 Physarum デモで **標高忌避をもっと強くしたい・スライダーで調整したい**。
netphys-003 で `NetParams::w_elev`（探索方向へ `-w_elev·∇E` を加算＝低標高側へ確率的に寄せる方向バイアス。
既定 2.0）を実装済みで、これは**予算に依らず効く**ソフト忌避（壁は作らない）。この係数をライブで上げ下げ
できれば、低地選好の強さ（山をどれだけ避けるか）を観察しながら調整できる。値を大きくするほど強く避け、
0 で方向バイアス無し（コスト/実効抵抗のみ）に戻る。

## task_id
`render-net-003`

## goal
`docs/demo-net/` に **標高忌避の強さ `w_elev` スライダー**を追加し、ライブで変更すると次 tick 以降の探索方向
バイアスに反映される。`render-wasm` の `NetSim` に `set_w_elev` を追加（`NetParams` を読み替えるだけ・非侵襲）。
**netphys の力学・`NetParams::w_elev` の既定値（2.0）は変えない**（露出のみ・一方向依存）。「もっと強く避ける」は
スライダーの**可動域を既定2.0より十分上まで**取り、かつ**デモの初期表示値を既定より強め**にすることで満たす
（下記の決定事項参照）。

## 決定が要る点（§8.1 で確認）
- **スライダー可動域**: 提案 `min=0, max=8`（0=方向バイアス無し、2=現行既定、4〜8=強め）。
- **デモの初期表示値**: 「もっと強く避けてほしい」を満たすため、提案 **初期 4.0**（現行既定 2.0 より強い）。
  - ※ これは**デモ UI の初期値**であり、`NetParams::w_elev` の既定（2.0・テスト/ヘッドレスの正）は**変えない**。
    デモ起動時に `set_w_elev(初期値)` を呼んで反映する。初期値を 2.0（モデル既定と一致）にするか 4.0（強め）に
    するかは人間が決める。もし「モデル既定自体を強くしたい」なら別途 netphys 調整タスク（受け入れ再確認が要る）。

## 実装方針
- **`render-wasm` の `NetSim::set_w_elev(v)`**（`set_period_n`/`set_collect_rate`/`set_w_rand` と同型）:
  - 保持している `NetParams` の `w_elev` を書き換えるだけ（次 step から新値を読む）。`f64`・**0.0〜8.0 にクランプ**
    （負値は 0 に＝忌避無し、上限で壁化を避ける）。
  - **モデルの力学・`NetParams::w_elev` の既定（2.0）は変えない**（読み替えのみ・非侵襲）。
  - native test（既存 `set_period_n` テストと同型）: 「`set_w_elev` が `NetParams.w_elev` を更新する」
    「非侵襲＝設定前後で（step 前は）hash 不変」「クランプ（負→0・8超→8）」「決定性＝同一 seed・操作列・w_elev なら
    同一 `net_state_hash`」。既存 `Sim`/`TreeSim`/`NetSim` native test は不変で緑。
- **`docs/demo-net/index.html` のスライダー**: 「周期(刈込)」の隣に「標高忌避 w_elev」レンジ（min=0, max=8,
  step=0.5, 初期=上記決定値）＋現在値表示。`input` で `sim.set_w_elev(v)` を呼ぶ。
  - **reset/seed 再生成時もスライダー値を保持**する（render-net-002 の period 修正と同じ挙動＝現在値を新 sim に
    `set_w_elev` で適用。初回のみ初期値）。ラベルで「大きいほど低地を強く好む／0 で方向バイアス無し」が分かる文言に。
- 決定性契約は「同一 seed・同一入力列（w_elev 変更操作も入力の一部）→同一 `net_state_hash`」を保つ（壁時計を状態遷移に入れない）。

## acceptance_test
1. **`NetSim::set_w_elev` 露出**: `render-wasm` に追加。native test で「`w_elev` が更新される」「非侵襲・決定的」
   「クランプ（負→0・8超→8）」。既存 `Sim`/`TreeSim`/`NetSim` native test は不変で緑。
2. **デモ**: `docs/demo-net/` に w_elev スライダーが出て、動かすと低地選好の強さが変わる（大きいほど高標高帯へ
   伸びにくくなるのが観察できる）。reset/seed でスライダー値を保持する。
3. **ブラウザ実測**: スライダー変更が `set_w_elev` を呼び反映されること・エラー無しを確認。可能なら「w_elev 大で
   高標高帯のノードが減る」傾向をヘッドレス相当（NetSim 駆動）で数値確認。
4. **リグレッション**: core(Jones)+tree+netphys+render-wasm 全テスト緑。wasm/JS glue 再生成しライブ整合。
   既存デモ `docs/demo/`・`docs/demo-tree/` は無変更。netphys モデル力学・`NetParams` 既定（w_elev=2.0）は無変更。

## constraints
- **編集可**: `render-wasm/src/lib.rs`（`NetSim::set_w_elev` 追加＝params 読み替えのみ）、`docs/demo-net/*`
  （スライダー追加）、生成物（wasm/js 再生成）。
- **編集不可（人間所有・§7/§11）**: netphys のモデル力学（`netphys_step`）と **`NetParams` 既定値（w_elev=2.0 含む）**、
  Jones/tree のコード・不変条件・ゴールデン・受け入れ、`tests/netphys_00x.rs` のアサート（netphys-003 の
  低地選好・ソフト性を弱めない）、設計軸（§0 ソフト忌避＝壁を作らない）。
- **一方向依存**: render は `src/netphys` を駆動して読むのみ。決定性契約（同一 seed・入力→同一 `net_state_hash`）を壊さない。
- §0 の動詞不変（砂糖 置く/取る・時間）。w_elev は観察用コントロール（力学の質は変えない・ソフトのまま）。

## このタスクでやらないこと
- netphys のモデル力学変更・`NetParams::w_elev` 既定変更（露出のみ。既定自体を強めたいなら netphys 調整タスクで
  受け入れ再確認）。
- 他パラメータ（fan_count/tero 係数 等）の同時露出（必要なら別タスク）。
- Jones/tree デモへの反映。

## 関連
- 露出の型: `NetSim::set_period_n`（[render-net-002](task-render-net-002.md)）・`TreeSim::set_w_rand`・`Sim::set_collect_rate`。
- モデル: [src/netphys/step.rs](../src/netphys/step.rs)（`-w_elev·∇E` 方向バイアス）、
  [src/netphys/state.rs](../src/netphys/state.rs)（`NetParams::w_elev` 既定 2.0）、[netphys-003](task-netphys-003.md)。
