# タスク: core-003 — 残量0の砂糖を自動削除

`loop-engineering-rules-v0.md` §11 のフォーマットに沿ったタスク文（人間所有）。
**前提**: `core-000/001/002` 完了・全テスト緑。

> **状態: 🔵 未着手（§8.1 で人間確認待ち）**

---

## 背景
砂糖源は `sugar_remaining` が回収で 0 まで減るが（[step.rs:154-174](../src/step.rs)）、
現状 **残量0でも砂糖源リストに残り続ける**。誘引ビーコンは残量>0 でのみ発火し
（[step.rs:181-217](../src/step.rs)）、回収ループも残量≤0 をスキップするため、
**枯渇した砂糖は既に力学的に不活性**（agent/trail/biomass に何も寄与しない）。
ユーザー要望（2026-07-07）: 残量0の砂糖を自動的に消したい。

## task_id
`core-003`

## goal
`step()` 内で、回収により `sugar_remaining <= 0` になった砂糖源を、その tick の末尾で
**決定論的に（id 昇順）自動削除**する。既存の `RemoveSugar` と同じ除去ロジックを内部発火する。
枯渇砂糖は力学的に不活性なので、これは agent/trail/biomass の**挙動を変えない**想定。

## acceptance_test（headless で判定可能）
1. **枯渇で消える**: 小さな `strength` の砂糖を、エージェントが回収し続けて枯渇させると、
   その砂糖源が `sugar_id/x/y/strength/remaining` から取り除かれ、砂糖数が減る。決定論的。
2. **枯渇前は消えない**: `sugar_remaining > 0` の砂糖はリストに残る。
3. **保存則の維持**: 削除で帳簿を壊さない。`collected_total`・`consumed_total`・`biomass` は
   削除の前後で不変（回収済み分は既に biomass 計上済み、remaining=0 を捨てるだけ）。
   `biomass == collected_total - consumed_total`、非負。
4. **不変条件の維持**: 有限性・境界・再現性・ソフト標高忌避（mean_hi<mean_lo）を S9 で維持。
5. **リグレッション無し（最重要）**: 既存の `core-000/001/002`・`analysis-00x`・
   `test_harness_001`・`metric_thresholds_001` が**全て緑**。既存シナリオは検証tick内で
   枯渇しない（strength 300〜600・collect_rate 0.5 では 600〜1200tick 必要）ため
   `final_state_hash` は不変のはず。**もし既存 hash が変わるなら停止して §8 で相談**（人間所有
   ゴールデン/baseline は編集しない）。
6. **決定性**: 同一 (seed, script, ticks) → 同一 `final_state_hash`。

## constraints
- **編集可**: `src/step.rs`（削除の発火）、必要なら `src/state.rs` の除去ヘルパ、新規テスト。
- **編集不可（人間所有・§7/§11・フック保護）**: 不変条件の定義としきい値、既存受け入れテストの
  アサート、`test_harness_001.rs`/`metric_thresholds_001.rs` の baseline、設計軸（§0）。
- **決定性維持**: 単一 PRNG・index 昇順・スナップショット感知。削除は id 昇順で決定的に。
- **順序の注意**: 削除は回収ループの後・その tick の描画/ハッシュ確定前。同一 tick に置いた
  strength>0 の砂糖は消さない。RemoveSugar と自動削除が同 tick で衝突しても決定的に。

## seeds
`[1, 42, 1337]`（可能なら S9）

## このタスクでやらないこと
- 誘引力を残量に比例させる等のビーコン仕様変更（別タスク候補）。今回は「枯渇したら消す」だけ。
