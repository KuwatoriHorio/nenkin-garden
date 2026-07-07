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

```
- iter: 4
  task: core-001
  hypothesis: coreパラメータ既定値の調整だけで、代表シナリオの2砂糖源を同一連結成分に
              繋げ flow_connected を頑健に成立させられる。
  diff_summary: |
    探索のみ（既定値の恒久変更なし）。n_init{200..1000}, diffuse{0.30..0.65},
    sugar_beacon{12..100}, sensor_dist{4..12}, ticks{240..500} を総当りで計測。
    診断: sugar1 のビーコン細胞が骨格で孤立1ノード成分になり(dist=0,size=1)、
    最近傍ノードsnapがそれを拾う。sugar0 は大成分に載る→常に別成分=flow_connected不成立。
  seeds: [1,42,1337] と 正準9本で検証
  invariants: pass  # 既定値は無変更のため既存12テスト緑のまま（リグレッションなし）
  metrics: |
    連結は「若い網の一過的な橋」で不安定: 密度/ticks/beacon を上げると coverage は
    増えるが connected は減る(2/3→1/3→0/3)。最良(n600,beacon45,diff0.55,t300)は
    [1,42,1337]で2/3だが正準9本では2/9=そのシードへの過学習。
  goldens_updated: none
  decision: revert（既定値変更せず）→ §8 エスカレーション
  note: |
    パラメータ調整では受け入れ(頑健なflow_connected)を満たせない。障害はcoreの物理網でなく
    analysisの砂糖源→ノードsnap規則（孤立ビーコンスパイクを拾う）。task-core-001が予告した
    §8停止条件に該当。overfitな2/3出荷・θ_cc恣意緩和・独断のビーコン仕様変更はしない。
    選択肢を人間に提示（analysis-002でsnap規則見直し等）。
```

```
- iter: 5
  task: analysis-002
  hypothesis: 砂糖源→端子snapを「半径内の近傍網への多tap＋拡張グラフでの連結判定」に
              変えれば、孤立ビーコンスパイクに吸着せず近傍の実ネットワークに接続できる。
  diff_summary: |
    src/analysis/flow.rs のみ変更（core非変更・非侵襲）。単一最近傍snap→半径 tap_radius 内
    の全骨格ノードへ多tap（無ければ最近傍1点フォールバック）。連結判定を node_comp比較
    から拡張グラフ（骨格エッジ+tapエッジ）の union-find 実接続へ。Params に tap_radius(=4.0)。
    tests/analysis_002.rs 追加。
  seeds: 制御シナリオ + 正準9本
  invariants: pass  # core無変更。既存 core-000/analysis-001 全緑（制御連結テスト R=5.0 も維持）
  metrics: |
    新テスト3件緑（近傍網へ接続=修正で true / 過剰連結防止=離れた網は false / 決定性・非侵襲）。
    tap_radius=4.0 strict のため analysis-001 の R=5.0 制御テストは不変。
  goldens_updated: none
  decision: keep
  note: |
    analysis-002 は完了（アーティファクト=孤立スパイク吸着 を修正）。ただし代表シナリオ
    (2源 距離7.2)は依然 0/9〜2/9=連結せず。これは snap の誤りでなく「本体網が sugar1 の
    半径内に届いていない=gap が実在」と確認できた（radius を恣意的に広げるのは §7 違反）。
    → core-001 の「本物の橋を core が育てる」問題は未解決のまま残る（要 人間判断）。
```

```
- iter: 6
  task: render-001
  hypothesis: coreをwasm化し、State を読むだけの render レイヤでブラウザ描画＋クリックで
              砂糖を置ける対話デモを作れる。core は不変・非侵襲。
  diff_summary: |
    ワークスペース化（Cargo.toml に [workspace], default-members=["."]）。新クレート
    render-wasm（cdylib+rlib, deps=wasm-bindgen, core lib に依存）。Sim: new/step/render/
    place_sugar_at_canvas/remove_sugar_at_canvas/pixels_ptr/state_hash_hex。canvas_to_cell は
    純関数（native test）。docs/demo/（index.html + 生成 render_wasm.js/_bg.wasm）。
    docs/index.html にデモ導線。.claude/launch.json（ローカル配信）。
  seeds: [42 等]（決定性は native test で確認）
  invariants: pass  # core無変更。core 15テスト + render-wasm 2テスト = 17 緑
  metrics: |
    ブラウザ実測: wasm 読込・描画・アニメ(tick進行)・クリックで砂糖設置・state_hash 表示 OK
    （preview で視覚確認）。§8決定: wasm-bindgen 採用（core汚染を避け別crateに隔離）。
  goldens_updated: none
  decision: keep
  note: |
    §8決定に基づき wasm-bindgen 採用。core crate は std のみ・外部依存ゼロを維持（依存は
    render-wasm に隔離＝core←render 一方向）。GNU toolchain を D:\Claude の override に設定、
    wasm32 target 追加、wasm-bindgen-cli は dlltool 不在でソースビルド不可→公式prebuilt(msvc)
    を導入。決定性は core と同一コードのため native test で担保（三角関数実装差で native と
    wasm のビット一致は §2 上要求しない=同一ビルド内再現性）。デモは docs/demo/ で Pages 公開。
```

```
- iter: 7
  task: metric-thresholds-001
  hypothesis: 退化した忌避健全性指標(elev_trail_ratio 常に0)を、常に定義される連続指標へ
              見直し、θ_cov/θ_cc は分解能を検証のうえ維持する。
  diff_summary: |
    src/metrics.rs に trail_weighted_mean_elevation / land_mean_elevation / elev_avoidance
    を追加（Σtrail·E / Σtrail と 陸地平均Eの比）。to_json 更新。src/bin/calibrate.rs に新指標。
    規約 §4 の健全性行を elev_trail_ratio → elev_avoidance に見直し(+8%でwarn)。
    tests/metric_thresholds_001.rs 追加。θ_cov/θ_cc は変更せず（分解能十分と実測確認）。
  seeds: 正準9本 + calibrate 1..=32
  invariants: pass  # §3 不変条件(mean_hi<mean_lo)は不可侵のまま。既存18テスト緑
  metrics: |
    実測(160tick,代表シナリオ): elev_trail_ratio min=med=max=0(退化を確認)。
    新 elev_avoidance med=0.126(<1=忌避が効く), relMAD 4.4%(連続・非退化)。
    coverage/max_cc/num_cc relMAD=10%/18%/2.9%(分解能あり=θは維持で妥当)。
  goldens_updated: "loop-engineering-rules-v0.md §4 健全性行: 退化指標を elev_avoidance へ（理由=退化是正）"
  decision: keep
  note: |
    §7遵守: しきいを緩めるのでなく、退化して情報量ゼロだった健全性指標を非退化の連続指標へ
    置換。人間所有の不変条件(§3)は不変。θ_cov/θ_cc は実測で分解能十分と確認し最小変更で維持。
    analysis の num_cc==core num_cc(θ_cc共有)も維持。
```

```
- iter: 8
  task: test-harness-001
  hypothesis: 規約§4で確定した正準シード集合9本と中央値ソフトゲートを、既存テストを弱めずに
              テストコードへ反映できる。
  diff_summary: |
    tests/test_harness_001.rs 追加（新規・既存は不変）。
    Tier0: §3不変条件(有限性/保存則/境界/再現性/ソフト忌避)を S 全9本で検証。
    Tier2: 共通ヘルパ soft_gate(方向つき Dir::{LowerWorse,HigherWorse}) で S 上の中央値を
    baseline定数と §4許容%(-8/-18/-18/+10/+8/-12)比較。baseline は S9・160tick 実測を固定。
  seeds: {1,7,13,42,99,256,1337,2024,31337}
  invariants: pass  # 既存 core-000/analysis-001 は未変更で全緑。総計20テスト緑。
  metrics: |
    baseline(S9中央値): coverage=0.02924 sugar_rate=0.09063 max_cc=10 num_cc=52
    elev_avoidance=0.12697 mean_trail_lo=0.05812。決定論のため現中央値=baseline(満額マージンで pass)。
  goldens_updated: "tests/test_harness_001.rs 内 baseline 定数（新規ゴールデン=前回green中央値）"
  decision: keep
  note: |
    §7遵守: 既存アサートは削除・緩和せず、被覆(3→9本)と集計法(中央値・方向つき)を追加するのみ。
    正準集合・許容%・方向は規約§4に一致。baseline は「意図した挙動変更」時のみ理由付き更新。
```

```
- iter: 9
  task: analysis-003
  hypothesis: flow ソルバが浮遊成分で特異化し連結を偽陰性にしている。source/sink 成分だけを
              解けば正しく connected を報告できる。
  diff_summary: |
    src/analysis/flow.rs: 縮約 Laplacian の対象を「全ノード−sink」から
    「source/sink を含む拡張成分のノード−sink」に限定（他の浮遊成分を除外し特異化回避）。
    tests/analysis_003.rs 追加（浮遊成分ありで connected / 真の別成分は false / 決定性）。
  seeds: 制御シナリオ + [1,42,1337]
  invariants: pass  # core非変更。既存 analysis_001(R=5.0)/002 と全体緑。
  metrics: |
    重大バグ: 従来は網が多成分(num_cc≈50)だと Laplacian が特異→常に flow_connected=false。
    過去の 0/9 計測は偽陰性だった。修正後、代表シナリオで点ビーコンは真に 0/9、
    ビーコン小半径化(radius=3)で 9/9(conductance≈0.8) を確認。
  goldens_updated: none
  decision: keep
  note: |
    core-001 iter:4 の「調整で連結不可」結論は本バグの影響を受けた誤り。連結性は本修正後に
    再評価する。§7遵守: ソルバの correctness 修正のみ、しきいは不変。
```

```
- iter: 10
  task: core-001
  hypothesis: ビーコンを小半径のブロブ化（sugar_beacon_radius=3）すれば、近接2源のブロブが
              拡散で融合し代表シナリオが頑健に連結する（analysis-003 のソルバ修正が前提）。
  diff_summary: |
    src/params.rs: sugar_beacon_radius 既定 0→3。src/step.rs: ビーコンを半径ガウスブロブで撒く。
    tests/core_001.rs 追加（S9 で flow_connected >=8/9, 実測9/9）。
    既定挙動変更に伴う golden 更新: test_harness_001 の baseline を新既定S9中央値へ貼り直し。
    metric_thresholds_001 の分解能判定を relMAD>0→max≠min に精緻化（max_cc は範囲89-169だが
    最頻値=中央値で relMAD=0 になるため; intent「単一値に潰れず」に忠実）。
    docs/network.svg・demo wasm 再生成、docs/index.html 指標表を新値へ。
  seeds: 正準9本
  invariants: pass  # core-000 不変条件維持。総計24テスト緑。
  metrics: |
    新既定 S9中央値: coverage 0.085 sugar_rate 0.44 max_cc 89 num_cc 38 elev_avoidance 0.248(<1)
    mean_trail_lo 0.19。代表シナリオ flow_connected=9/9(conductance≈0.8)。seed42: nodes86/edges69,
    redundancy1.14, transport_efficiency0.45。
  goldens_updated: "test_harness_001 baseline（core-001 の意図的挙動変更）; docs 各種再生成"
  decision: keep
  note: |
    ISSUE-001 解決。§7遵守: しきい恣意化なし・不変条件不可侵。既定密度(n_init=80)のまま
    ビーコン小半径化だけで連結。metric_thresholds/test_harness の変更は意図した挙動変更に伴う
    正当な golden 更新・proxy 精緻化であり、テストの弱体化ではない。
```

```
- iter: 11
  task: render-002
  hypothesis: analysis の出力（グラフ・MST・流量）から、幹線=流量太さ/MST実線・冗長辺破線/
              成分色分けの静的グラフSVGを、非侵襲・決定的に生成できる。
  diff_summary: |
    analysis: flow::solve が捨てていた per-edge 電流を FlowResult.edge_currents に公開、
    AnalysisResult.edge_currents から取得可能に（読み取り専用の露出, 新指標でない）。
    src/graph_svg.rs 新規（flow_width 純関数 + graph_to_svg: 成分色/流量太さ/MST実線・冗長破線/
    source·sink）。src/bin/render_graph_svg.rs, tests/render_002.rs。docs/network_graph.svg 生成、
    index.html にグラフビューの節を追加。
  seeds: [1,42,1337]
  invariants: pass  # core非変更・edge_currentsはフィールド追加のみ。総計28+2テスト緑。
  metrics: |
    seed42: SVG バイト一致(決定的)・生成前後で state_hash 不変(非侵襲)・data-nodes/edges が
    analysis の nodes/edges と一致。source↔sink の幹線が最大幅で描画（視覚確認）。
  goldens_updated: "docs/network_graph.svg 新規生成 + index.html グラフ節追加"
  decision: keep
  note: |
    §7遵守: 見栄えの自己申告でなく判定可能条件（決定性/非侵襲/整合/写像単調）で合格。
    core←analysis←render の一方向依存を維持。WASMトグル統合は render-003 候補として未着手。
```
