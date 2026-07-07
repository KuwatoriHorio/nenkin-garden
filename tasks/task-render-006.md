# タスク: render-006 — デモに trail 濃度上限(trail_max)スライダーを追加

`loop-engineering-rules-v0.md` §11 のフォーマットに沿ったタスク文（人間所有）。
**前提**: [core-004](task-core-004.md)（trail_max のコア実装）完了・全テスト緑。
`render-001〜005` 完了（対話デモ・チューニングパネル稼働）。

> **状態: ✅ 完了**（2026-07-07, iter:18）— render-wasm に `set_trail_max()` 追加、デモに
> trail_max スライダー（id=trailMax・6〜60は数値/端61=∞上限なし・既定18=core-004 の緩和値）を
> チューニングパネルに実装。render-wasm 8テスト緑、core/保護ファイル無変更。ブラウザ実測で
> スライダー配線（∞切替含む）・採餌・エラー無しを確認。実装 sonnet、検証・keep はオーケストレーター(opus)。

---

## 背景
core-004 で trail 濃度の上限 `trail_max` を追加し局在化を緩和できるようにするが、
コア既定は上限なし。ユーザーが「見え方を確認しながら」上限値を調整したいので、
render-005 で作った開発用チューニング・パネルに `trail_max` スライダーを足す。

## task_id
`render-006`

## goal
デモで `trail_max`（trail 濃度上限）をスライダーで実行時調整し、局在化が緩和される様子
（砂糖を置くと群れがホームから離れて探しに行く）を見られるようにする。
core は読むだけ＋実行時 setter（既定値は変えない）。

## acceptance_test
1. **render-wasm 露出**: `pub fn set_trail_max(&mut self, v: f64)`（実行中 Sim の `params.trail_max` を
   更新・`params.rs` 既定は不変）を追加。native test で「setter が params.trail_max を変える」
   「core の力学は読むだけで非侵襲（既存の非侵襲テストが緑）」を確認。
2. **デモ UI**: チューニング・パネル（render-005 の `.panel`）に `trail_max` スライダー
   （現在値表示つき・「上限なし」を端に用意: 例 最大値=∞相当の大きな値または OFF ラベル）。
   採餌モードの既定として**局在化が目に見えて緩和される finite 値**をプリセットしてよい
   （デモを開いた時点で効果が分かるように）。既存のトグル・スライダー・砂糖操作は従来どおり。
3. **ブラウザ実測**: trail_max を下げると群れがホームに張り付きにくくなり、離れた砂糖へ
   向かいやすくなること、上限なしだと局在化することを目視で確認。コンソールエラー無し。
4. **リグレッション**: core + render-wasm 全テスト緑。wasm/JS glue 再生成しライブ整合。

## constraints
- **編集可**: `render-wasm/src/lib.rs`（読み取り＋実行時 setter）、`docs/demo/index.html`、生成物。
- **編集不可（人間所有・§7/§11）**: core の不変条件・ゴールデン・受け入れテスト、設計軸（§0）。
- **core の力学非変更**: render は State を読むだけ。`set_trail_max` は実行中 params のみ。
- §0 の動詞不変（trail_max スライダーは開発用チューニング・パネル＝プレイ動詞ではない）。

## このタスクでやらないこと
- 調整で決めた `trail_max` を `params.rs` 既定へ恒久反映すること（別ステップ・人間承認の
  ゴールデン更新）。まずは見ながら探る手段を作る。
