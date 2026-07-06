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
