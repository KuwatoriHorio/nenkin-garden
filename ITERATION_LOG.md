# イテレーション記録（規約 §10）

```
- iter: 1
  task: core-000
  hypothesis: Jones模型の決定論ヘッドレスコアを最小実装し、再現性・不変条件・
              headless出力（metrics.json + final_state_hash）の受け入れ基準を満たす。
  diff_summary: |
    Rust(std のみ)で新規実装。
    src/{params,world,rng,state,step,metrics,hash,headless}.rs, src/bin/run_headless.rs,
    tests/core_000.rs。PRNGは xoshiro256**+splitmix64 を自前実装（外部依存なし=規約§11）。
    step は index昇順・固定3乱数/体、trail_read スナップショット + trail_write 別書き、
    拡散→減衰→swap（§4/§5）。ソフト標高忌避4系統（感知/傾斜/定着/減衰）。
    成長ループ実駆動のため n_init_agents=80(<agent_cap) に設定。
  seeds: [1, 42, 1337]
  invariants: pass  # 有限性/保存則/境界/再現性/ソフト標高忌避 の5項目
  metrics: { seed42@160tick: coverage=0.0216, sugar_collected=9, biomass=7.5,
             consumed=1.5, n_agents=90(80→成長), max_cc=10, num_cc=56 }
  goldens_updated: none
  decision: keep
  note: |
    受け入れテスト5件すべて緑（hash 2回一致 / 3シード不変条件 / 描画OFF-ON不変 /
    headless出力 / シード間ハッシュ相違）。数値の質（被覆・連結性）の最適化は
    設計メモ §6 に従い後続タスク。mean_trail_hi=0 は高標高帯にagentが未到達なだけで、
    忌避は「壁」ではなくソフト（不変条件 hi<lo は成立）。
```

```
- iter: 2
  task: analysis-001
  hypothesis: coreのtrail網から効率ネットワークを静的導出し、決定性・非侵襲・core整合・
              網レベルの標高忌避・analysis.json出力の受け入れ基準を満たす。
  diff_summary: |
    src/analysis/{mod,skeleton,graph,flow}.rs 新規（core ← analysis の一方向依存）。
    しきい化(theta_cc共有)→Zhang-Suen細線化(+成分保存)→グラフ化(ノード=分岐/端点/
    昇格代表, エッジ=degree-2チェーン, 自己ループ破棄・多重辺保持・正準ソート)→
    Kirchhoff密ソルバで単位電流を1回解く。transport_efficiency=HHI採用。
    src/bin/run_analysis.rs, tests/analysis_001.rs 追加。Params に net_alpha 等追加。
  seeds: [1, 42, 1337]
  invariants: pass  # core側の不変条件は不変（analysisはStateを読むだけ）
  metrics: { analysis@seed42/160tick: nodes=73, edges=17, num_cc=56(=core), largest_cc=2,
             total_length=25.14, redundancy≈1(森), flow_connected=false(疎な網) }
  goldens_updated: none
  decision: keep
  note: |
    受け入れ7件緑（#1決定性/#2健全性/#3 num_cc整合/#4網の標高忌避/#5 json/#6非侵襲
    + 流れソルバ直接検証）。
    途中修正1: Zhang-Suenが極小成分を全削除しnum_cc不一致(56 vs 58)→マスク各成分に
    骨格を最低1画素復元する preserve_components を追加してcoreと一致（§7遵守: テストを
    緩めず事実側を修正）。
    既定シナリオは網が断片化しflow_connected=false（源間に経路なし＝正直な結果）。
    Kirchhoffソルバ/コンダクタンス/HHIは連結網の制御テスト
    (flow_solver_on_controlled_connected_network: R=5.0, TE=1.0)で検証。
```

```
- iter: 3
  task: calib-001
  hypothesis: v0で未確定の数値/方針(N/許容%/シード本数/golden粒度/中央値集計法)を、
              代表シナリオの多シード実測で決定し規約に反映する（規約変更=人間承認タスク）。
  diff_summary: |
    src/bin/calibrate.rs 追加（core非侵襲の計測ハーネス, seeds1..=64の分布・中央値収束）。
    loop-engineering-rules-v0.md の §4(許容%表+シード集合+中央値集計法)・§5(golden粒度)・
    §6(N=3)・末尾節を確定値へ更新。tasks/task-calib-001.md 追加。
  seeds: 計測1..=64 / 確定ゲート={1,7,13,42,99,256,1337,2024,31337}(奇数9本)
  invariants: pass  # 規約更新はコード挙動を変えず既存テスト12件緑のまま
  metrics: |
    実測(160tick,代表シナリオ)relStd: coverage19% sugar42% max_cc40% num_cc8% tick28%。
    中央値はcoverage/num_cc/mean_trail_loがk≈9で大標本値に一致。elev_trail_ratioは常に0(退化)。
  goldens_updated: none
  decision: keep
  note: |
    確定: N=3 / 許容%(被覆-8,砂糖-18,max_cc-18,num_cc+10warn,mean_trail_lo-12,tick+25) /
    シード9本 / golden=hash(seedごと)+metricsベクトル / 中央値=平均でなく方向つき比較。
    設定原理=「9本中央値の標準誤差≈relStd×0.417」以上に許容を取りノイズ誤検知を回避
    （旧-2%/-5%は過敏だったと実測で判明）。§7遵守: テストを弱めず外部ポリシーのみ確定。
```
