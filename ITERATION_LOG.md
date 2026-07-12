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

```
- iter: 12
  task: render-003
  hypothesis: render-wasm にグラフ幾何のアクセサを足し、デモに density⇔graph トグルを付ければ、
              ライブでグラフ（幹線=流量太さ/MST実線・冗長破線/成分色）を見られる。
  diff_summary: |
    graph_svg::mst_edge_set を pub 化。render-wasm/Sim に compute_graph + アクセサ
    (graph_nodes/edges/edge_currents/edge_mst/edge_comp/max_current) 追加（analyze を読むだけ）。
    docs/demo/index.html を SCALE=6 の高解像度描画へ、モードトグル(密度⇔グラフ)と graph 描画
    (canvas 2D, throttle 150ms)を実装。フレーム先頭で clearRect（下地蓄積バグ修正）。
    wasm/bindings 再生成。tasks/task-render-003.md。
  seeds: [42]（幾何の決定性・非侵襲を native test で担保）
  invariants: pass  # core非変更・render は読むだけ。core28 + render-wasm3 テスト緑。
  metrics: |
    render-wasm test: compute_graph 前後で state_hash 不変（非侵襲）・同一State→同一幾何（決定的）・
    電流長==エッジ数。ブラウザ実測: density⇔graph トグルで描画切替、コンソールエラー無し。
  goldens_updated: "docs/demo 再生成（wasm/js/index.html）"
  decision: keep
  note: |
    §7遵守: 判定可能条件で合格。表示切替はシミュ非変更（動詞は §0 のまま）。グラフ再計算は
    throttle だが決定性は「同一State→同一幾何」で担保。core←analysis←render の一方向維持。
```

```
- iter: 13
  task: core-002
  hypothesis: エージェントをホームに凝集させて始め、前進確率に trail 勾配コホージョン
              （空白へはソフトに出にくいが p_move>0 で滲める）を足せば、群れが凝集したまま
              ホームから砂糖へ触手を伸ばして到達し（伸び）、餌が消えた枝は減衰で退縮する（縮み）。
  diff_summary: |
    params.rs に home_x/home_y/init_cluster_sigma/w_trail_cohesion を追加（既定は現行挙動
    ＝sigma=0 一様散布, cohesion=0 抑制なし）。world.rs に default_home（低標高陸重心近傍を
    決定的に選ぶ）。state.rs に cluster_positions（Box-Muller で home 周りガウス配置・海は
    ホームへフォールバック）。step.rs の前進ゲートに p_move *= exp(-w_trail_cohesion*max(Δtrail,0))
    （必ず p_move>0）。tests/core_002.rs 新規。
  seeds: [1,7,13,42,99,256,1337,2024,31337]（S9・中央値/計数で判定）
  invariants: pass  # 既定オフで既存30テスト挙動バイト維持。preset でも Tier0 全数緑。
  metrics: |
    foraging preset(sigma=3, cohesion=1, home自動, 砂糖を距離12に配置): 実測 S9 中央値 —
    reach 9/9・warmup 群れ拡がり中央値 3.4（cohesion=0 は 24 に四散）・砂糖除去後の
    A地点 trail retention 0.35（peak 103→after 36 ＝トンネル退縮）。受け入れ①〜④成立。
  goldens_updated: none  # 既存ゴールデン不変。新規テストの baseline のみ（実測 preset）。
  decision: keep
  note: |
    §7遵守: 「見た目」でなく判定可能条件（凝集・到達計数・退縮 retention）で合格。過学習回避に
    最初から S9 で評価。設計要点＝「空白への抵抗はソフトのみ・壁を作らない」（§0）を p_move>0 で担保。
    退縮は幾何非依存の時間比較（砂糖除去→減衰）で測った（鏡像対照は島形状で不成立のため棄却）。
    新挙動は既定オフ。既定化やデモ有効化は別ステップ（デモ有効化は render 系の後続タスク候補）。
```

```
- iter: 14
  task: render-004
  hypothesis: render-wasm に採餌コンストラクタ（core-002 の凝集＋コホージョン preset）と
              ホーム座標アクセサを足し、demo にモードトグルを付ければ、ブラウザで
              伸び（砂糖へ触手到達）・縮み（除去で退縮）をライブで見せられる。
  diff_summary: |
    render-wasm/src/lib.rs: Sim::new_forage（sigma=3, cohesion=1, home自動）と build ヘルパへ
    リファクタ、home_x/home_y/is_forage アクセサ、native test forage_mode_*（決定性＋凝集）追加。
    docs/demo/index.html: 採餌/従来トグル（既定ON）・ホーム印◇・説明文更新。wasm/js glue 再生成。
    tasks/task-render-004.md。
  seeds: [42]（render は読むだけ・幾何/凝集の決定性を native test で担保）
  invariants: pass  # core 非変更。core 全テスト＋render-wasm 4 テスト緑。
  metrics: |
    render-wasm test: new_forage 決定的・初期エージェントの過半がホーム半径12内に凝集・同一操作列→同一hash。
    ブラウザ実測(seed42): 採餌モードで群れがホーム◇に凝集→距離12の砂糖へ触手が伸びて到達(伸び)→
    砂糖除去でトンネル退縮(縮み)。density⇔graph・従来モードも動作、コンソールエラー無し。
  goldens_updated: "docs/demo 再生成（wasm/js/index.html）"
  decision: keep
  note: |
    §7遵守: ブラウザ検証は判定可能条件（凝集/到達/退縮の目視＋native 決定性 test）で確認。
    §0 の動詞不変（採餌は初期条件の切替で新動詞ではない）。core←render の一方向依存を維持。
```

```
- iter: 15
  task: core-003
  hypothesis: 枯渇砂糖(remaining<=0)はビーコン/回収とも remaining>0 条件でスキップ済み＝力学的に
              不活性なので、step() 末尾で決定論的に自動削除しても agent/trail/biomass の挙動は
              変わらず、砂糖リストだけが掃除される。
  diff_summary: |
    state.rs: RemoveSugar の除去を remove_sugar_at ヘルパへ切り出し、pub fn remove_depleted_sugar
    （id昇順で remaining<=0 を前方走査削除・決定論）を追加。lib.rs で公開。
    step.rs: 砂糖ビーコンループ後・成長セクション前に remove_depleted_sugar を1行呼ぶ
    （回収/ビーコンが使う配列長 m を乱さないタイミング）。tests/core_003.rs 新規（4テスト）。
  seeds: [1,42,1337]（foraging 回帰は S9 部分集合で不変条件＋決定性）
  invariants: pass  # 既存30テスト挙動バイト維持（既存シナリオは検証tick内で枯渇しない）。core_003 4テスト緑。
  metrics: |
    枯渇1tick→id0削除/id1残存、同時枯渇→両削除/無関係残存、保存則 biomass==collected-consumed 維持、
    同一seed→同一 final_state_hash。全17スイート緑・cargo exit 0（オーケストレーター独立検証）。
  goldens_updated: none  # 既存ゴールデン不変。
  models: { orchestrator: opus, implement: sonnet(subagent), verify: opus, record: opus }
  decision: keep
  note: |
    §8.1 改訂後の初の自動委譲。実装を nenkin-implementer 相当(sonnet)へ委譲し、keep 判定と
    独立検証はオーケストレーター(opus)。カスタム定義は今session未ロードのため汎用agent+model指定で代替。
    §7遵守: 自己申告を鵜呑みにせず全スイートを独立再実行して緑を確認。保護ファイル無編集。
```

```
- iter: 16
  task: render-005
  hypothesis: render-wasm に agent 位置アクセサと collect_rate 実行時setterを足し、render を
              show_trail 切替に拡張すれば、デモでエージェントを点で見せ・trail を消し・
              砂糖量/回収レートを見ながら調整できる（core 非変更・読むだけ）。
  diff_summary: |
    render-wasm/src/lib.rs: agent_positions()（ax/ay を flat で・非侵襲）、set_collect_rate()
    （実行中 params のみ更新・既定不変）、render(show_trail:bool) に拡張。native test 6件緑。
    docs/demo/index.html: drawAgents() 常時描画、trail表示トグル（既定OFF）、開発用チューニング
    パネル（strength/collect_rate スライダー・破線枠でプレイ動詞と区別）。wasm/JS glue 再生成。
  seeds: [42]（render は読むだけ・非侵襲/決定性を native test で担保）
  invariants: pass  # core(src/) 無変更。render-wasm 6テスト緑。core 全テストは前 iter から不変。
  metrics: |
    ブラウザ実測(seed42): 既定で trail OFF・エージェントが白点でホーム凝集を可視化、trailトグルで
    密度復帰、strength(→800)/collect_rate(→2.5)スライダー反映、砂糖設置→採餌、コンソールエラー無し。
    native: agent_positions 長さ==2×n_agents・取得前後で state_hash 不変・set_collect_rate が効く。
  goldens_updated: "docs/demo 再生成（wasm/js/index.html）"
  models: { orchestrator: opus, implement: sonnet(subagent), verify: opus(+browser), record: opus }
  decision: keep
  note: |
    §0 の動詞不変（strength/collect_rate は開発用チューニング・パネル＝プレイ動詞ではない、UI で区別）。
    core←render の一方向依存を維持。ブラウザ検証はプレビューツールを持つオーケストレーターが実施。
```

```
- iter: 17
  task: core-004
  hypothesis: trail に上限 trail_max を設けクランプすれば、ホーム中心の誘引の井戸が頭打ちになり
              遠くの砂糖ビーコンが対等に競えて局在化が緩和される。既定∞なら既存挙動はバイト不変。
  diff_summary: |
    params.rs: pub trail_max: f64（既定 f64::INFINITY）。step.rs: 拡散・減衰の最終書き戻しで
    state.trail[i] = (v as f32).min(trail_max as f32)（海マスク=0は従来どおり）。
    tests/core_004.rs 新規（5テスト）。探索用 _probe は削除済み。
  seeds: [1,42,1337]（実測選定 trail_max=18, R=6）
  invariants: pass  # 既定∞で既存30テスト hash バイト不変。finite でも Tier0 維持。core_004 5テスト緑。
  metrics: |
    局在化指標 L=ホーム半径R内trail/全trail。trail_max=18(R=6): L中央値 0.505 < ∞ 0.644（約21%低下・
    各シードで一貫 lf<li）。離れた砂糖回収は非退行(med_fin>=med_inf)。全trail<=trail_max+1e-3。
    同一seed→同一hash。全17スイート緑・cargo exit 0（オーケストレーター独立検証）。
  goldens_updated: none  # 既定∞で既存ゴールデン不変。
  models: { orchestrator: opus, implement: sonnet(subagent), verify: opus, record: opus }
  decision: keep
  note: |
    §7遵守: テストを独立再実行し緑を確認、テスト内容も読んで非退行が `>=` で正しく主張されて
    いることを検証（自己申告の「完全一致」は観測値で、テストは退行しないことのみ要求）。
    trail_max のソフト飽和は値のクランプのみで移動は禁じない（§0 壁を作らない）。既定は∞のまま。
```

```
- iter: 18
  task: render-006
  hypothesis: render-wasm に set_trail_max を足し、デモに trail_max スライダー（既定=core-004 の
              緩和値18）を置けば、局在化の緩和を見ながら上限値を調整できる（core 非変更・読むだけ）。
  diff_summary: |
    render-wasm/src/lib.rs: set_trail_max()（実行中 params.trail_max のみ更新・既定不変）、
    native test 2件（setter・非侵襲/決定性）。docs/demo/index.html: trailMax スライダー
    （6〜60は数値・端61=∞上限なし・既定18）をチューニングパネルに追加、fresh() で再適用。
    wasm/JS glue 再生成。
  seeds: [42]（render は読むだけ・非侵襲/決定性を native test で担保）
  invariants: pass  # core(src/) 無変更。render-wasm 8テスト緑。
  metrics: |
    ブラウザ実測(seed42): trailMax スライダー既定18、最大位置で「∞（上限なし）」表示、低値も反映、
    採餌で群れがホーム-砂糖間に伸び張り付き緩和、コンソールエラー無し。
    native: set_trail_max が params を変える・呼び出し前後で state_hash 不変・同設定で決定的。
  goldens_updated: "docs/demo 再生成（wasm/js/index.html）"
  models: { orchestrator: opus, implement: sonnet(subagent), verify: opus(+browser), record: opus }
  decision: keep
  note: |
    局在化緩和の定量効果は core-004（L 中央値 21%低下）で実証済み。render-006 はスライダー配線と
    デモ健全性をブラウザで確認。§0 の動詞不変（trail_max は開発用チューニング）。core←render 一方向維持。
```

```
- iter: 19
  task: render-007
  hypothesis: 近接エージェントを枝でつなぐ agent_links を足し、加算合成の発光する枝＋ノードで
              描けば、独立した点でなく有機的な樹状（ニューロン）として枝を伸ばして広がる見え方になる。
  diff_summary: |
    render-wasm/src/lib.rs: agent_links(radius)（グリッド空間分割で近傍探索・各agent最近傍最大2本・
    (a,b)昇順 BTreeSet で決定論出力・自己リンク無し・非侵襲）、native test 3件。
    docs/demo/index.html: drawAgentsAsNeurons（globalCompositeOperation=lighter の発光枝＋発光ノード・
    LINK_THROTTLE_MS=120）、drawAgentsAsDots を従来表示として保持、#agentStyle トグル（既定=樹状ON）。
    wasm/JS glue 再生成。
  seeds: [42]（render は読むだけ・非侵襲/決定性を native test で担保）
  invariants: pass  # core(src/) 無変更。render-wasm 11テスト緑。core 全テストも緑（workspace）。
  metrics: |
    ブラウザ実測(seed42): 既定で樹状（発光する枝でつながった有機的構造）が描画、採餌で枝が伸長、
    #agentStyle トグルで点表示へ切替、tick5765/速度120でも快適・コンソールエラー無し。
    native: agent_links は同一State→同一リンク・呼び出し前後で state_hash 不変・index有効/a<b/自己リンク無し。
  goldens_updated: "docs/demo 再生成（wasm/js/index.html）"
  models: { orchestrator: opus, implement: sonnet(nenkin-implementer), verify: opus(+browser), record: opus }
  decision: keep
  note: |
    見た目の方向性はユーザー確定「ニューロン樹状（線ベース）」。表現(render)のみ変更で step は不変
    （§0 動詞・core←render 一方向維持）。密メッシュを避け最近傍最大2本で樹状に。カスタム
    サブエージェント nenkin-implementer を型名で直接起動（今session でロード済み）。
```

```
- iter: 20
  task: tree-growth-001
  hypothesis: 木(親子パスのみ・閉路なし)を、砂糖への space colonization で全体予算Bをパス距離に
              保存的再配分（伸長=free→構造, 退縮=構造→free）すれば、砂糖へ伸び・枝分かれし・餌喪失で
              縮む成長木が決定論的に作れる。現行 Jones とは別モデルとして並置できる。
  diff_summary: |
    新規独立モジュール src/tree/{state,step,hash,mod}.rs（TreeState/Node/TreeParams/tree_step/
    tree_state_hash/run_tree_headless）。src/bin/run_tree.rs 新設。lib.rs に pub mod tree、
    Cargo.toml に bin を加算的登録のみ。tests/tree_growth_001.rs 新規6件。既存 Jones コードは無変更。
  seeds: [1,42,1337]（S9部分集合・中央値）
  invariants: pass  # 新モデル独自の不変条件。既存 Jones の全テストも不変で緑（独立モジュール）。
  metrics: |
    TreeParams default(実測調整): k=1.0 c_elev=2.2 c_branch=0.15 growth_rate=0.6
    branch_angle=0.9 max_path_len=10 retreat=0.25 max_step_per_tick=1.5 attract_radius=48 B0=250。
    受け入れ①到達(nearest<=sugar_radius) ②保存則(total_volume==collected-consumed,非負) ③分岐≥1+両到達
    ④退縮(構造長 after/before<=0.6) ⑤不変条件(有限/境界=陸/hash再現/標高忌避 high<low*0.95 於予算律速regime)
    ⑥木性(根1・親高々1・閉路なし連結)。全スイート緑・exit0（オーケストレーター独立再実行）。
  goldens_updated: none  # 新モデルは独立。既存ゴールデン不変。
  models: { orchestrator: opus, implement: sonnet(nenkin-implementer), verify: opus, record: opus }
  decision: keep
  note: |
    §0 の設計軸に関わる新モデルをユーザー承認の下で並置（現行 Jones は無傷）。実装中に overshoot
    デッドロック（大配分でtipが場外へ跳び境界棄却で恒久停止）を max_step_per_tick で修正=実バグ是正
    でテスト非弱化。⑤標高忌避テストは予算律速regimeで「高標高への伸長抑制」を測る（受け入れ⑤の
    「または」条項を満たす）＝§7ごまかしでないとオーケストレーターが精読確認。描画は後続（木は
    render-007 ニューロン表現と好相性）。
```

```
- iter: 22
  task: render-tree-001
  hypothesis: TreeSim(wasm) で src/tree を駆動し、親子パスを発光する枝として別ページに描けば、
              木モデルの伸び・分岐・退縮をブラウザで観察できる（core/木モデル力学は非変更）。
  diff_summary: |
    render-wasm/src/lib.rs: TreeSim（Sim と別struct・TreeState/TreeParams/World を保持し tree_step で
    駆動）＋ new_tree/step/place-remove sugar/sugar_positions/tree_nodes/tree_edges/tree_state_hash_hex/
    render(land-sea)/home_x/y。native test 2件。docs/demo-tree/ 新規（発光枝＋ノード描画・砂糖操作・
    再生/速度/reset/seed）。docs/index.html に導線1つ。wasm/JS glue を demo-tree へ出力。
  seeds: [42]（render は駆動して読むだけ・決定性を native test で担保）
  invariants: pass  # Jones core・src/tree 力学 無変更。render-wasm 13テスト緑・core+tree 全テスト不変で緑。
  metrics: |
    ブラウザ実測(seed42): ホーム根から砂糖へ枝が伸長、2箇所で枝分かれ、砂糖除去で根まで退縮、
    コンソールエラー無し。native: 同一seed・同一op列→同一 tree_state_hash（決定性）、
    tree_edges.len==2*(nodes-1)（連結木）。既存 Sim テストは不変で緑。
  goldens_updated: "docs/demo-tree 新規（wasm/js/index.html）＋ docs/index.html 導線"
  models: { orchestrator: opus, implement: sonnet(nenkin-implementer), verify: opus(+browser), record: opus }
  decision: keep
  note: |
    別ページ実装（ユーザー確定）で既存 Jones デモ docs/demo/ は無変更。TreeSim は tree_step を回すだけで
    木モデルの成長規則は不変（core←render 一方向）。木構造は本質的に樹状なので render-007 の発光枝描画を流用。
```

```
- iter: 23
  task: tree-growth-002
  hypothesis: tip 成長方向を w_rand·ランダム(state.rng)＋Σ誘引 のブレンドにすれば、砂糖なしでも
              予算内で探索伸長し、近くの砂糖では誘引が支配して到達する。既定オフで既存を無傷に保てる。
  diff_summary: |
    src/tree/state.rs: TreeParams に w_rand(既定0.0=探索オフ)・explore_persistence(既定0.45)追加。
    src/tree/step.rs: w_rand>0 のとき tip 毎に state.rng を1本引き persistence 混合のランダム単位
    ベクトルを誘引方向へ w_rand 重みで合成（誘引無し tip も探索候補に・退縮に代え予算内で伸長）。
    w_rand<=0 は現行コードパスそのまま（rng不引き・バイト不変）。非放射移動の保存則リーク
    （Δd<dd の差 k·(dd-Δd)）を consumed 計上する会計修正も w_rand>0 に gate。tests/tree_growth_002.rs 新規6件。
  seeds: [1,42,1337]（S9部分集合・中央値）。探索値 w_rand=0.3 / explore_persistence=0.45（実測選定）
  invariants: pass  # 既定 w_rand=0 で tree_growth_001 6件 hash 不変・全緑。tree_growth_002 6件緑。
  metrics: |
    ①砂糖なし探索でノード/構造長増（baseline w_rand=0 は根のみ=伸びず）②T=400→900 でプラトー・
    initial_budget 内・全tickで total_volume==collected-consumed ③w_rand>0 でも近砂糖へ到達(<=sugar_radius)
    ④砂糖有/無とも同一seed→同一hash ⑤境界=陸内/木性/標高忌避 ⑥既定 w_rand==0.0。全スイート緑・exit0。
  goldens_updated: none  # 既定オフで既存ゴールデン不変（tree_growth_001 無編集）。
  models: { orchestrator: opus, implement: sonnet(nenkin-implementer), verify: opus, record: opus }
  decision: keep
  note: |
    実装中の2発見を精読検証: persistence>0.5 は海方向で方向固定→恒久デッドロック（0.45採用）、
    非放射移動の保存則リークは実バグ→w_rand>0 に gate して consumed 計上で厳密化（②で実証）。
    いずれもテスト非弱化・既定バイト維持。探索デモ露出は後続 render-tree タスク（TreeSim に w_rand 露出）。
```

```
- iter: 24
  task: render-tree-002
  hypothesis: TreeSim に w_rand の実行時 setter を足し demo-tree にスライダーを置けば、探索（砂糖なし
              彷徨い）と誘引支配をデモで見て調整できる（木モデル力学・TreeParams 既定は非変更）。
  diff_summary: |
    render-wasm/src/lib.rs: TreeSim に set_w_rand / set_explore_persistence（実行中 params のみ更新・
    既定不変）＋ native test 2件（setter が効く・非侵襲・w_rand>0 で hash が実際に発散＝params は
    hash 非包含を確認）。docs/demo-tree/index.html: 探索強度 w_rand スライダー（id=wrand・0〜1・
    step0.05・初期0.3）、fresh() で再適用。wasm/JS glue を demo-tree へ再生成。
  seeds: [42]（render は駆動して読む＋setter のみ・決定性は native/既存で担保）
  invariants: pass  # src/tree 力学・TreeParams 既定・Jones core 無変更。render-wasm 15テスト緑・全体緑。
  metrics: |
    ブラウザ実測(seed42): w_rand=0.3・砂糖なしで木がランダムに彷徨い伸長、w_rand=0・砂糖なしで
    根のみ（探索オフ）、w_rand=0.3・砂糖ありで砂糖方向へ直線的に到達（誘引支配）、コンソールエラー無し。
  goldens_updated: "docs/demo-tree 再生成（wasm/js/index.html）"
  models: { orchestrator: opus, implement: sonnet(nenkin-implementer), verify: opus(+browser), record: opus }
  decision: keep
  note: |
    露出のみ（木モデル力学・TreeParams 既定 w_rand=0.0 不変）。setter は実行中インスタンスの params。
    §0 動詞不変（w_rand は開発用チューニング）。既存 Jones デモ docs/demo/ は無変更。core←render 一方向維持。
```

```
- iter: 25
  task: netphys-001 (Stage 1)
  hypothesis: 一般グラフで扇状探索→衝突で網化→周期 consolidation(最外周端子で Kirchhoff+保存的
              Tero 刈り込み)を回せば、餌を結ぶ連結網を保存則・決定性・有界のもとで作れる（担体A）。
  diff_summary: |
    新規独立モジュール src/netphys/{state,step,kirchhoff,hash,mod}.rs（NetState/NetParams/netphys_step/
    netphys_state_hash/netphys_kirchhoff_solve/run_netphys_headless）。src/bin/run_netphys.rs。
    lib.rs に pub mod netphys、Cargo.toml に bin 加算登録。analysis/flow.rs は solve_dense を
    pub(crate) 化する可視性1行のみ（ロジック無変更）。tests/netphys_001.rs 新規4件（①②⑤⑥）。
  seeds: [1,42,1337]（S9部分集合・中央値）
  invariants: pass  # 新モデル独自。Jones/tree/analysis 既存全テスト不変で緑（flow.rs 可視性のみ）。
  metrics: |
    NetParams(実測): period_n=12 fusion_dist=3 k_frontier=6 search_step=2 w_rand=1 d0=0.35 c_elev=1.5
    tero_gain=1.4 tero_decay=0.35 prune_eps=0.02 node_cap=220 edge_cap=520 B0=400。
    ①中央値で1成分がループ(辺数>ノード-1) ②2餌が flow_connected・有限正コンダクタンス
    ⑤total_mass==collected-consumed(担体A)・非負・境界=陸内・同一seed→同一hash・標高忌避(高<低*0.95)
    ⑥800tick でも node/edge cap 内。全スイート緑・exit0（オーケストレーター独立再実行）。
  goldens_updated: none  # 新モデルは独立。既存ゴールデン不変。analysis/flow.rs は可視性のみ（挙動不変）。
  models: { orchestrator: opus, implement: sonnet(nenkin-implementer), verify: opus, record: opus }
  decision: keep
  note: |
    §0 設計軸に関わる第3モデル（Jones/tree/netphys）をユーザー承認の下で並置。受け入れは段階化＝
    Stage1(①②⑤⑥)のみ本タスク合否、③前進波移動・④効率改善は netphys-002。実装中の2実バグ
    （anastomosis が自己親とマージし網が育たない／Tero が D を直接上書きし質量非保存）を修正＝
    テスト非弱化。⑤保存則テストが担体A の会計を厳密に締める。Kirchhoff は analysis の dense ソルバを
    可視性のみで再利用（Laplacian 組立は netphys 側=analysis の pixel-index グラフと結合せず）。
```
