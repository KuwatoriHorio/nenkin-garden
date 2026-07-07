# tests/ を触るときの規範ポインタ（§4 詳細のパススコープ）

規約 §4/§5 の判定はこのディレクトリのコードが「正」。常駐コアには数値を置かず、ここで所在を示す。

- **正準シード集合 S9** = `{1, 7, 13, 42, 99, 256, 1337, 2024, 31337}`（奇数9本, 中央値一意）。全テストで共通に使う。
- **メトリクス許容%（§4 表・calib-001 で確定）**: 被覆 −8% / 砂糖 −18% / max_cc −18% / num_cc +10% で warn / mean_trail_lo −12% / tick_ms +25% / elev_avoidance +8% で warn。
  設定原理は「9本中央値の標準誤差 ≈ relStd×0.417」以上に取る（固有ノイズでの誤検知防止）。詳細根拠は `../tasks/task-calib-001.md`。
- **集計法**: S9 上の**中央値**（平均でない＝外れ値耐性）で方向つき比較。ハード対象は全て許容内(AND)で pass。→ 実装は [test_harness_001.rs](test_harness_001.rs) の `soft_gate` と baseline定数。
- **忌避指標**: elev_trail_ratio は退化のため elev_avoidance（trail加重平均標高/陸地平均）へ変更済み。→ [metric_thresholds_001.rs](metric_thresholds_001.rs)。
- **N（試行上限）= 3**。

## この配下での編集の制約（§7・§11）

- `test_harness_001.rs`・`metric_thresholds_001.rs` は**人間所有のしきい値/baseline**を持つため PreToolUse フックで編集拒否される（`.claude/protected-paths.txt`）。しきい値変更は §8 の承認タスクで。
- 新しい受け入れ/再現テスト（§6-2）は**新規ファイル**として追加する（既存の人間所有テストを弱めない＝§7）。
