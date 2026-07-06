# 設計メモ — core-000: 決定論シミュレーションコア

`loop-engineering-rules-v0.md` の最初のタスク `core-000` を Claude Code に実装させる前の、
インターフェースと契約を固める設計メモ。**実装言語や性能最適化は実装者(Claude Code)の裁量**だが、
ここで定義する入出力・決定性・不変条件は満たすこと。

---

## 1. スコープ

- Jones模型（trailマップ + 拡散・減衰）による粘菌シミュレーションの**最小・決定論・ヘッドレス**実装。
- 含む: 陸海マスク、標高場E、ソフト標高忌避、砂糖→バイオマス成長ループ、`run_headless`、`state_hash`。
- 含まない（この段階では不要）: 描画、GUI、GPU化、複雑なUI。描画は**分離**しておく（後付け）。

---

## 2. モジュール構成（描画と分離）

```
core/        … 純粋なシミュレーション。副作用なし。描画・IO・時刻に依存しない
  state.*    … State のデータ構造
  world.*    … 場（陸海マスク・標高E）のロード/保持（読み取り専用）
  step.*     … step(state, inputs, params, rng) -> state'
  metrics.*  … Stateからメトリクスを算出（読み取りのみ）
  hash.*     … state_hash(state) -> 64bit
headless/    … run_headless エントリ。core を呼ぶだけ
tests/       … 不変条件・ゴールデン・メトリクス・性能
analysis/    … （後日/予約）Stateのtrailからネットワークを導出し効率を数値化。
               coreを読むだけ・逆依存なし。決定論コアの派生レイヤ（analysis-001で実装）
render/      … （後日）Stateを読むだけ。coreに依存、逆はしない
```

> `analysis/` と `render/` は core-000 の実装対象外だが、
> 依存方向（core ← analysis / core ← render、逆は禁止）を最初から守れるよう予約しておく。

---

## 3. データ構造（State）

- `tick: u64`
- `trail: f32[H][W]` … 誘引物質場。海セルは常に0/無効。
- `agents: Agent[]` … `Agent { x: f32, y: f32, heading: f32 }`（配列。**index順が正準順序**）
- `biomass: f64`
- `sugar_sources: Sugar[]` … `Sugar { x, y, strength, remaining }`
- `collected_total: f64`, `consumed_total: f64` … 保存則チェック用の帳簿
- `rng: <seeded PRNG state>` … 単一。State内に持ち、遷移で再現可能に

### 場（World, 読み取り専用・不変）
- `land_mask: bool[H][W]` … true=陸（這える）, false=海
- `E: f32[H][W]` … 標高。0=海面〜1=最高峰。`land_mask=false` は海。

---

## 4. 決定性の契約（最重要）

- 乱数は `state.rng` の**単一PRNGのみ**。壁時計・実時間・環境依存の反復順を遷移に使わない。
- エージェント更新は**必ず index 昇順**で回す（RNG消費順を固定するため）。
- 1tick内のセンシングは**tick開始時のtrailスナップショット**から読む。
  定着(deposit)は別バッファ（または加算）へ書き、その後に拡散・減衰してからswap。
  → 同一tick内でのエージェント処理順が**センシング結果に影響しない**ようにする。
- 浮動小数の非決定性回避のため、比較対象のプラットフォーム/ビルドは固定する（規約§2）。

---

## 5. 1ステップの手順（step）

tick開始時 `trail_read = trail`（スナップショット）, `trail_write = 0`。

各エージェント（index昇順）について:

1. **感知**: 前方3点をサンプル（距離 `sensor_dist`、角度 `-sensor_angle, 0, +sensor_angle`）。
   各点の評価値 `V = sample(trail_read) - w_e * sample(E)`（**標高ペナルティ=忌避①**）。
   海セルは大きな負値扱いで避ける。
2. **旋回**: Jonesの規則で最大Vの方向へ `turn_speed` 回頭
   （中央最大→直進 / 左右いずれか最大→その側へ / 均衡→ランダム、RNG使用）。
3. **前進**: 目標セルへ `step_size` 前進を試みる。
   移動確率 `p = exp(-k_slope * max(0, ΔE))`（ΔE=標高上昇分、**傾斜コスト=忌避②**）。
   移動先が海/範囲外、または確率で失敗 → 前進せずランダムに新方位（RNG使用）。
4. **定着**: 現在セルへ `trail_write += deposit * H(E)`。
   `H(E)=habitability`= `E_hi` 超で滑らかに0へ落ちる smoothstep（**定着差=忌避③**）。

全エージェント処理後:

5. **砂糖回収**: 各 sugar_source について、半径内に到達したエージェントがあれば
   `gain = min(collect_rate, remaining)` を `biomass += gain`, `collected_total += gain`,
   `remaining -= gain`。source は毎tick trail に強い誘引を加算（ビーコン）。
6. **成長**: `max_agents = agent_cap(biomass)`（例: 線形/対数の飽和関数）。
   現在数 < 上限なら、既存の網（trail高セル or 既存エージェント近傍）から新規スポーン。
   スポーン/維持で `consumed_total` を計上（保存則の帳簿を必ず更新）。
7. **拡散・減衰**: `trail_write` を3x3ブラー等で拡散し、
   `trail_write *= (1 - decay(E))`。`decay` は**高標高ほど大きい**（乾燥=**減衰差=忌避④**）。
   海セルは0にマスク。
8. `trail = trail_write`, `tick += 1`。

---

## 6. パラメータ（params, 既定値は実装で定義し1箇所に集約）

`sensor_dist, sensor_angle, turn_speed, step_size, deposit, decay_base, decay_high,
diffuse_rate, w_e(感知標高重み), k_slope(傾斜コスト), E_hi(居住性しきい), E_lo,
collect_rate, sugar_radius, agent_cap()係数, spawn規則`

- **すべて params 経由**でハードコード禁止（ループがチューニングできるように）。
- 既定値は「まず正しく動く」保守的な値でよい。数値の最適化は後続タスク。

---

## 7. run_headless インターフェース

```
run_headless(seed: u64,
             world: {land_mask, E},         // ファイルパス or 既定の合成列島でも可
             input_script: [{tick, op}],    // 時系列の入力
             ticks: u64,
             params) -> { metrics: MetricsJSON, final_state_hash: u64 }
```

- `input_script` の op（園芸型の動詞）:
  - `place_sugar {x, y, strength}`
  - `remove_sugar {id}`
  - （time speedは headless では ticks 数で表現、opには不要）
- 描画は一切呼ばない。**描画ON/OFFで final_state_hash が変わってはならない**。
- 出力の `MetricsJSON` は §8 の全メトリクスを含む。

---

## 8. メトリクス定義（metrics.*）

- `coverage`: `count(trail > θ_cov & land) / count(land)`
- `sugar_collected`: `collected_total`（＋レートは差分で算出可能に）
- `max_cc`: trail>θ を陸上でしきい化した二値網の**最大連結成分のセル数**
- `num_cc`: 連結成分数
- `elev_trail_ratio`: `mean(trail | E>=E_hi) / mean(trail | E<E_lo)`（忌避の健康診断）
- `tick_ms`: 1tick平均処理時間（性能予算, tests/性能用）

連結成分の実装（4/8近傍・Union-Find等）は実装者裁量。しきい値 `θ_cov, θ` は params。

---

## 9. state_hash 定義（決定性の要）

順序を固定して正準シリアライズし、64bitハッシュ（例: FNV-1a / xxhash）を取る:

1. `tick`
2. `biomass`（固定小数へ量子化: 例 1e-6）
3. `agents` を index順に `(x, y, heading)` を量子化（例 1e-4）して連結
4. `trail` を行優先で量子化（例 1e-4）して連結
5. `collected_total, consumed_total`（量子化）
6. `rng` の内部状態

量子化により微小な浮動小数ノイズでハッシュが割れるのを防ぐ。量子化幅は params/定数で1箇所管理。

> **派生レイヤの一意性保証**: `analysis/`（analysis-001）や `render/` の入力は State（特に trail）だけである。
> したがって同一 `state_hash` からは**必ず同一のネットワーク指標・同一の描画**が導かれる。
> これにより解析・描画は「Stateから一意に決まる派生量」となり、決定論コアの枠内に収まる
> （＝規約§2「コアを正」を崩さない）。解析レイヤは State を**読むだけ**で、書き換えてはならない。

---

## 10. 不変条件チェック（tests/ 疑似コード, 規約§3の実体）

```
def check_invariants(run):                 # 複数シードで実行
    s = run.final_state

    # 有限性
    assert all(isfinite(v) for v in s.trail)
    assert all(isfinite(a.x) and isfinite(a.y) and isfinite(a.heading) for a in s.agents)

    # 保存則
    assert s.biomass >= 0
    assert approx_equal(s.biomass, s.collected_total - s.consumed_total, eps)

    # 境界
    for a in s.agents:
        assert in_bounds(a.x, a.y)
        assert world.land_mask[cell(a.x, a.y)] == LAND

    # 再現性（別実行と一致）
    assert run.final_state_hash == rerun_same_inputs(run).final_state_hash
    assert hash_with_render_off == hash_with_render_on

    # 標高忌避（ソフト: ゼロでなく「有意に低い」）
    warmup(run, T)                          # 立ち上げ後に判定
    assert mean(trail | E>=E_hi) < mean(trail | E<E_lo)
```

各アサートは規約§3の項目に1対1で対応。`eps, T` は tests 側の定数。

---

## 11. 受け入れテスト（core-000 の完了定義, 規約§9）

- 同一 `(seed, input_script, ticks)` で `final_state_hash` が**2回一致**（3シード: 1, 42, 1337）。
- §10 の不変条件が**3シードすべてで通る**。
- 描画OFF/ONで（この段階は描画未実装＝常にOFFで）ハッシュが不変であることを担保する構造。
- `run_headless` が `metrics.json` と `final_state_hash` を出力する。

---

## 12. 実装者(Claude Code)の裁量・確認事項

- **言語/依存**: Rust　性能優先
- **世界データ**: 実地形の取得は後回しでよい。まずは**合成の仮想列島**
  （複数の島 land_mask + 山脈状の E フィールドを手続き生成）で可。実地形はネットワーク前提のため別タスク。
- **連結成分・拡散・PRNG** の具体実装は裁量。ただし§4の決定性契約を破らないこと。
- 迷ったら（仕様が曖昧/矛盾する場合）実装を進めず、規約§8に従い停止して質問する。
```

