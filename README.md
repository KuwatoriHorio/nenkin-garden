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
  analysis/   … 効率ネットワーク解析（analysis-001, core を読むだけ・逆依存なし）
    skeleton.rs … しきい化 + Zhang-Suen 細線化 + 成分保存
    graph.rs    … 骨格→グラフ（ノード/エッジ, MST, 連結成分）
    flow.rs     … Kirchhoff 線形系を1回解く（実効抵抗/コンダクタンス/輸送効率）
    mod.rs      … パイプライン統合と analysis.json
  bin/run_headless.rs … CLI（metrics.json 書き出し + hash 表示）
  bin/run_analysis.rs … CLI（core実行→静的解析→analysis.json）
tests/core_000.rs    … core 不変条件・受け入れテスト（§10/§11, seeds=[1,42,1337]）
tests/analysis_001.rs … analysis 受け入れテスト（#1..#6 + 流れソルバ直接検証）
```

`render/` は今後の対象。依存方向（core ← analysis/render、逆は禁止）を守る。
core は analysis/render に依存しない。

## analysis-001: 効率ネットワーク解析（静的・裏方・非侵襲）

Jones コアが育てた trail 網を Tero–Nakagaki の土俵に一度だけ乗せて効率を数値化する
静的レイヤ。**適応則は回さない**（管の太化/細化なし）。State は読むだけ・書き換え禁止。
同一 `state_hash` → 同一指標。

- パイプライン: しきい化(theta_cc, core と共有) → 骨格抽出(Zhang-Suen) →
  グラフ化(ノード=分岐/端点/昇格代表, エッジ=枝) → 流れを1回解く → 指標算出。
- 出力 `analysis.json`: `nodes, edges, total_length, mst_length, redundancy,
  total_conductance, effective_resistance, transport_efficiency, edge_mean_elevation,
  num_cc, largest_cc, flow_connected`。
- **transport_efficiency** は正規化エッジ流量の Herfindahl 指数 `Σ(I_e/ΣI)^2`（採用理由:
  外生パラメータ不要・全エッジ利用・値域(0,1]が明快で「少数の幹線への集約=効率」を表す）。
- 砂糖源は流れの端子（source=id最小 / sink=id最大）。両端が別の連結成分なら経路が無く
  `flow_connected=false`（実効抵抗∞）＝疎な網の正直な答え。ソルバ本体は連結網の
  制御テストで検証している。
- `num_cc` は core の `num_cc`（同一しきい値）と一致（骨格の成分保存で担保）。
- `edge_mean_elevation`（length加重）は陸地平均標高より低い＝ソフト忌避が網でも効く。

```sh
cargo run --release --bin run_analysis -- 42 160   # analysis.json 出力
```

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
