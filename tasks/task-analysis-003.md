# タスク: analysis-003 — flow ソルバの多成分特異バグ修正

`loop-engineering-rules-v0.md` §11 のフォーマットに沿ったタスク文。
**発見**: core-001 再開中の診断で、`analysis/flow.rs` の Kirchhoff ソルバが、
source/sink と非連結な**浮遊成分**を含む全ノードで Laplacian を組み、接地が sink 1点
のみのため**特異化して解けず**、連結しているのに `flow_connected=false` を返していた。

---

## 位置づけ（重大な correctness バグ）
網は通常、多数の連結成分を持つ（num_cc≈50）。従来のソルバは全成分を含む Laplacian を
1点接地で解こうとして特異になり、**source/sink が実際に連結していても常に false** を
返していた。これにより過去の `flow_connected` 計測（core-001 iter:4 の 0/9 等）は
**バグ由来の偽陰性**で、物理的連結性を反映していなかった。

## task_id
`analysis-003`

## goal
flow ソルバの連結判定を、拡張グラフ（骨格エッジ + tap）の union-find に基づき、
**source/sink を含む連結成分のノードだけで縮約 Laplacian を解く**よう修正する。
浮遊成分を除外して特異化を防ぎ、実際の連結を正しく `flow_connected=true` と報告する。

## acceptance_test（headless で判定可能）
1. **浮遊成分があっても解ける**: 「source/sink を繋ぐ網A」＋「無関係な孤立網B（浮遊）」の
   制御シナリオで `flow_connected=true`・`total_conductance>0`（修正前は特異で false）。
2. **非連結は依然 false**: 半径外で真に別成分の2源は `flow_connected=false`（analysis-002 の
   過剰連結防止テストが維持）。
3. **決定性・非侵襲**: 同一 State で2回一致・`state_hash` 不変・core 非変更。
4. `core-000`/`analysis-001`/`analysis-002` の既存テストが全緑（制御連結テスト R=5.0 維持）。

## constraints
- 変更は `analysis/flow.rs` のみ（core 非変更・State 読み取り専用）。
- 決定性契約（§2）維持。ノード/成分の走査順を正準化。
- §7: ソルバを正しくするだけで、しきい等は都合よく変えない。

## seeds
制御シナリオ + `[1,42,1337]`。

## 影響
- この修正後、`flow_connected` が初めて物理的連結を正しく反映する。core-001 の連結性評価は
  本修正を前提に再評価する（点ビーコンは真に 0/9、ビーコン小半径化で 9/9 を確認済み）。
