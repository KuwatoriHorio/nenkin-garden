# 粘菌ガーデン — 決定論シミュレーションコア (core-000)

Jones 模型（trail マップ + 拡散・減衰）による粘菌シミュレーションの、
**最小・決定論・ヘッドレス**実装。設計は [`tasks/design-memo-core-000.md`](tasks/design-memo-core-000.md)、
開発規約は [`loop-engineering-rules-v0.md`](loop-engineering-rules-v0.md) に従う。

## 言語選定（設計メモ §12）

**Rust（std のみ、外部クレートなし）**。理由:

- 性能と将来の描画/GPU 化を見据える（設計メモ §12 の「Rust 等」）。
- 規約 §11「ネットワーク不使用」に沿い、**外部依存を追加しない**。PRNG（xoshiro256\*\* +
  splitmix64 seed）も自前実装し、プラットフォームに依らない完全決定論を担保する。

## モジュール構成（描画と分離・設計メモ §2）

```
src/
  params.rs   … 全パラメータ（1箇所集約, ハードコード禁止）
  world.rs    … 場（陸海マスク・標高E）。読み取り専用・seed非依存の合成仮想列島
  rng.rs      … 決定論PRNG（単一シード, 内部状態はハッシュに含む）
  state.rs    … State と初期化・入力op適用（agentsはindex順が正準）
  step.rs     … step(state, world, params, ops): 1tickの純粋遷移（§5）
  metrics.rs  … Stateから指標を算出（読み取りのみ, §8）
  hash.rs     … state_hash: 量子化して64bit FNV-1a（§9）
  headless.rs … run_headless エントリ（coreを呼ぶだけ, §7）
  bin/run_headless.rs … CLI（metrics.json 書き出し + hash 表示）
tests/core_000.rs … 不変条件・受け入れテスト（§10/§11, seeds=[1,42,1337]）
```

`analysis/` `render/` は core-000 の対象外だが、依存方向（core ← analysis/render、
逆は禁止）を守るため後付けする。core はそれらに依存しない。

## 決定性の契約（規約 §2, 設計メモ §4）

- 乱数は `state.rng` の**単一PRNGのみ**。壁時計・環境依存の反復順を遷移に使わない。
- エージェント更新は **index 昇順**、固定 3 本/体（均衡時の旋回 / 前進確率 / 失敗時方位）。
- 感知は tick 開始時の trail スナップショットから読み、定着は別バッファへ書いてから
  拡散・減衰して差し替える → 同一 tick 内の処理順がセンシング結果に影響しない。
- 描画は一切呼ばない。`final_state_hash` は描画 ON/OFF で不変（＝描画を呼ばない構造）。

## ソフト標高忌避（4系統・壁は作らない）

① 感知ペナルティ `w_e` / ② 傾斜コスト `p=exp(-k_slope·ΔE)` /
③ 定着差 `H(E)`（E_hi 超で smoothstep→0）/ ④ 減衰差 `decay(E)`（高標高ほど大）。

## ビルド & テスト

```sh
cargo test                                   # 不変条件+受け入れテスト（3シード）
cargo run --release --bin run_headless -- 42 160   # seed=42, 160tick 実行
```

> Windows で MSVC リンカが無い場合は GNU ツールチェインで動く（リンカ同梱・自己完結）:
> `cargo +stable-x86_64-pc-windows-gnu test`

## 受け入れ基準（core-000 完了定義・規約 §9）

- 同一 `(seed, input_script, ticks)` で `final_state_hash` が 2 回一致（seeds 1/42/1337）。
- 不変条件（有限性 / 保存則 / 境界 / 再現性 / ソフト標高忌避）が 3 シードで通る。
- 描画 OFF/ON でハッシュ不変（この段階は描画未実装＝常に OFF）。
- `run_headless` が `metrics.json` と `final_state_hash` を出力する。
