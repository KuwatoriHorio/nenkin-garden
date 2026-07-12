# タスク: render-net-002 — 網 Physarum デモに consolidation 周期 `period_n` スライダーを露出

`loop-engineering-rules-v0.md` §11 のフォーマットに沿ったタスク文（人間所有）。
**前提**: [render-net-001](task-render-net-001.md)（`NetSim`・`docs/demo-net/`）完了・全テスト緑。
netphys-002/003/004（前進波・標高忌避・扇状拡散）反映済み。設定露出の型は
[render-tree-002](task-render-tree-002.md)（`set_w_rand` スライダー）・`Sim::set_collect_rate`/`set_trail_max`。

> **状態: 🅿 未着手（人間確認待ち）** — §8.1 に従い、タスク切り出しのみ。実装は人間の合図を待つ。

---

## 背景
ユーザー要望（2026-07-12）: 網 Physarum デモで **consolidation の周期（サイクルの周期）を調整できるようにしたい**。
netphys では `NetParams::period_n`（既定 12）が「N tick ごとに最外周端子で Kirchhoff＋Tero 刈り込み＝
consolidation を発火」する周期で、これが**扇状探索→網化→刈り込み→前進波**のリズムを決める。これをライブで
変えると、周期が短い＝こまめに背骨へ集約して前進が速い／長い＝広く網を張ってからまとめて刈る、といった
挙動の違いを観察できる。**再生速度（`speed`＝実時間あたりの tick 数）とは別物**（周期はシミュレーション内の
consolidation 間隔）である点に注意。

## task_id
`render-net-002`

## goal
`docs/demo-net/` に **consolidation 周期 `period_n` のスライダー**を追加し、ライブで変更すると次 tick 以降の
consolidation 発火間隔に反映される。`render-wasm` の `NetSim` に `set_period_n` を追加（`NetParams` を読み替える
だけ・**netphys の力学・既定値は変えない**）。既存の Jones/tree デモ・netphys モデルは無変更（render は netphys を
駆動して読むだけの一方向依存）。

## 実装方針
- **`render-wasm` の `NetSim::set_period_n(v)`**（`Sim::set_collect_rate`/`set_trail_max`・`TreeSim::set_w_rand` と同型）:
  - 保持している `NetParams` の `period_n` を書き換えるだけ（次 step から新周期を読む）。`u64`・**1 以上にクランプ**
    （0 は consolidation が毎 tick or ゼロ除算になりうるので下限を設ける）。上限も妥当値でクランプ（例 200）。
  - **モデルの力学・`NetParams::period_n` の既定値（12）は変えない**（読み替えのみ・非侵襲）。
  - native test（`render-wasm` の既存 `set_*` テストと同型）: 「`set_period_n` が `NetParams.period_n` を更新する」
    「非侵襲＝設定前に決定的で、同一 seed・同一操作列・同一 period_n なら同一 `net_state_hash`（決定性契約）」。
    既存 `Sim`/`TreeSim`/`NetSim` の native test は不変で緑。
- **`docs/demo-net/index.html` のスライダー**: `speed` の隣に「周期 period_n」レンジ（例 min=1, max=120,
  初期=12）＋現在値表示。`input` で `sim.set_period_n(v)` を呼ぶ。reset/seed 再生成時は初期値（12）へ戻す。
  ラベルで**再生速度とは別**（consolidation 間隔）と分かる文言にする。
- 決定性契約は「同一 seed・同一入力列（周期変更操作も入力の一部）→同一 `net_state_hash`」を保つ。スライダー操作は
  ユーザー入力であり、既存 `speed` 同様に非決定化しない（壁時計を状態遷移に入れない）。

## acceptance_test
1. **`NetSim::set_period_n` 露出**: `render-wasm` に追加。native test で「`period_n` が更新される」「非侵襲・決定的
   （同一 seed・操作列・period_n → 同一 `net_state_hash`）」。既存 `Sim`/`TreeSim`/`NetSim` native test は不変で緑。
2. **デモ**: `docs/demo-net/` に period_n スライダーが出て、動かすと consolidation 間隔が変わる（周期が短いほど
   こまめに刈り込み・長いほど網が広がってから刈る、が観察できる）。reset/seed で初期値に戻る。
3. **ブラウザ実測**: スライダー変更が `set_period_n` を呼び反映されること・エラー無しを確認。
4. **リグレッション**: core(Jones)+tree+netphys+render-wasm 全テスト緑。wasm/JS glue 再生成しライブ整合。
   既存デモ `docs/demo/`・`docs/demo-tree/` は無変更・従来どおり動く。netphys モデル力学・`NetParams` 既定は無変更。

## constraints
- **編集可**: `render-wasm/src/lib.rs`（`NetSim::set_period_n` 追加＝params 読み替えのみ）、`docs/demo-net/*`
  （スライダー追加）、生成物（wasm/js 再生成）。
- **編集不可（人間所有・§7/§11）**: netphys のモデル力学（`netphys_step`）と **`NetParams` 既定値**、Jones/tree の
  コード・不変条件・ゴールデン・受け入れ、`tests/netphys_00x.rs` のアサート、設計軸（§0）。
- **一方向依存**: render は `src/netphys` を駆動して読むのみ。決定性契約（同一 seed・入力→同一 `net_state_hash`）を壊さない。
- §0 の動詞不変（砂糖 置く/取る・時間）。period_n は「時間速度の変更」に準じる観察用コントロール（力学の質は変えない）。

## このタスクでやらないこと
- netphys のモデル力学変更・`NetParams` 既定変更（露出のみ）。
- 他パラメータ（fan_count/w_elev/tero 係数 等）の同時露出（必要なら別タスク）。
- Jones/tree デモへの反映・統合トグル。

## 関連
- 露出の型: `Sim::set_collect_rate`/`set_trail_max`・`TreeSim::set_w_rand`（[render-tree-002](task-render-tree-002.md)）。
- モデル: [src/netphys/step.rs](../src/netphys/step.rs)（consolidation 周期 `period_n` の発火）、
  [src/netphys/state.rs](../src/netphys/state.rs)（`NetParams::period_n`）、[render-net-001](task-render-net-001.md)。
