//! analysis-001 受け入れテスト（tasks/task-analysis-001.md acceptance_test 1..6）。
//! seeds = [1, 42, 1337]。core を非侵襲に読むだけの静的解析であることを検証する。

use nenkin_garden::analysis::{analyze, mean_land_elevation};
use nenkin_garden::hash::state_hash;
use nenkin_garden::metrics::compute_metrics;
use nenkin_garden::params::Params;
use nenkin_garden::state::{apply_op, initial_state, Op, ScriptEntry, State};
use nenkin_garden::world::{make_synthetic_archipelago, World};
use nenkin_garden::run_headless;

const SEEDS: [u64; 3] = [1, 42, 1337];
const TICKS: u64 = 160;

fn setup(seed: u64) -> (Params, World, State) {
    let p = Params::default();
    let w = make_synthetic_archipelago(&p);
    let mut cells: Vec<usize> = (0..w.h * w.w)
        .filter(|&i| w.land_mask[i] && (w.e[i] as f64) < p.e_lo)
        .collect();
    if cells.is_empty() {
        cells = (0..w.h * w.w).filter(|&i| w.land_mask[i]).collect();
    }
    let c = cells[cells.len() / 2];
    let (sx, sy) = ((c % w.w) as f64 + 0.5, (c / w.w) as f64 + 0.5);
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx, y: sy, strength: 400.0 } },
        ScriptEntry { tick: 5, op: Op::PlaceSugar { x: sx + 6.0, y: sy + 4.0, strength: 300.0 } },
    ];
    let r = run_headless(seed, &script, TICKS, &p, Some(&w));
    (p, w, r.final_state)
}

/// #1 決定性/一意性: 同一 State から解析を2回 → 指標が完全一致（3シード）。
#[test]
fn deterministic_unique() {
    for &seed in &SEEDS {
        let (p, w, st) = setup(seed);
        let a = analyze(&st, &w, &p).metrics;
        let b = analyze(&st, &w, &p).metrics;
        assert_eq!(a.nodes, b.nodes);
        assert_eq!(a.edges, b.edges);
        assert_eq!(a.num_cc, b.num_cc);
        assert_eq!(a.largest_cc, b.largest_cc);
        // 浮動小数も決定的（同一コード・同一入力）なのでビット一致するはず
        assert_eq!(a.total_length.to_bits(), b.total_length.to_bits());
        assert_eq!(a.mst_length.to_bits(), b.mst_length.to_bits());
        assert_eq!(a.redundancy.to_bits(), b.redundancy.to_bits());
        assert_eq!(a.total_conductance.to_bits(), b.total_conductance.to_bits());
        assert_eq!(a.effective_resistance.to_bits(), b.effective_resistance.to_bits());
        assert_eq!(a.transport_efficiency.to_bits(), b.transport_efficiency.to_bits());
        assert_eq!(a.edge_mean_elevation.to_bits(), b.edge_mean_elevation.to_bits());
    }
}

/// #2 グラフ健全性: well-formed（有限, 自己ループ無, a<=b, 有効な多重辺のみ）。
#[test]
fn graph_well_formed() {
    for &seed in &SEEDS {
        let (p, w, st) = setup(seed);
        let res = analyze(&st, &w, &p);
        let m = &res.metrics;
        // 有限性
        for v in [
            m.total_length, m.mst_length, m.redundancy, m.total_conductance,
            m.transport_efficiency, m.edge_mean_elevation,
        ] {
            assert!(v.is_finite(), "seed {seed}: 非有限メトリクス");
        }
        // エッジの整合: a<=b, 自己ループ無, ノード範囲内, length>0
        let n_nodes = res.graph.node_px.len();
        for e in &res.graph.edges {
            assert!(e.a <= e.b, "seed {seed}: エッジ端点非正準");
            assert!(e.a != e.b, "seed {seed}: 自己ループが残存");
            assert!(e.b < n_nodes, "seed {seed}: ノード id 範囲外");
            assert!(e.length > 0.0 && e.length.is_finite(), "seed {seed}: 不正な length");
            assert!((0.0..=1.0).contains(&e.mean_e), "seed {seed}: mean_e 範囲外");
        }
        // redundancy >= 1（MST は総延長の下界）— エッジがあるとき
        if m.mst_length > 0.0 {
            assert!(m.redundancy >= 1.0 - 1e-9, "seed {seed}: redundancy<1");
        }
        // transport_efficiency は (0,1]（流れが成立したとき）
        if m.flow_connected {
            assert!(m.transport_efficiency > 0.0 && m.transport_efficiency <= 1.0 + 1e-9);
        }
    }
}

/// #3 coreとの整合: 解析グラフの num_cc が core の num_cc（同一しきい値）と一致。
#[test]
fn consistent_with_core_num_cc() {
    for &seed in &SEEDS {
        let (p, w, st) = setup(seed);
        let core_m = compute_metrics(&st, &w, &p);
        let a = analyze(&st, &w, &p).metrics;
        assert_eq!(
            a.num_cc, core_m.num_cc,
            "seed {seed}: num_cc 不整合 analysis={} core={}",
            a.num_cc, core_m.num_cc
        );
    }
}

/// #4 忌避のネットワーク側確認: edge_mean_elevation < 陸地平均標高。
#[test]
fn network_elevation_avoidance() {
    for &seed in &SEEDS {
        let (p, w, st) = setup(seed);
        let a = analyze(&st, &w, &p).metrics;
        let land_mean = mean_land_elevation(&w);
        assert!(a.edges > 0, "seed {seed}: エッジ無し（前提不足）");
        assert!(
            a.edge_mean_elevation < land_mean,
            "seed {seed}: 網の平均標高が陸地平均以上 edge={} land={}",
            a.edge_mean_elevation, land_mean
        );
    }
}

/// #5 出力: analysis.json 相当（JSON 文字列）に主要キーが含まれる。
#[test]
fn outputs_json() {
    let (p, w, st) = setup(1);
    let json = analyze(&st, &w, &p).metrics.to_json();
    for key in [
        "nodes", "edges", "total_length", "mst_length", "redundancy",
        "total_conductance", "effective_resistance", "transport_efficiency",
        "edge_mean_elevation", "num_cc", "largest_cc",
    ] {
        assert!(json.contains(key), "json に {key} が無い");
    }
}

/// 流れソルバの直接検証（制御された連結網）。
///
/// 創発ダイナミクスに依存せず、既知のトポロジ（水平な1本の網 x=2..6, 平坦 E=0）で
/// Kirchhoff ソルバを直接評価する。source/sink が同一成分に載るため flow_connected=true。
/// 期待: 1本の直線網 → 実効抵抗 = 実効長 = 4, コンダクタンス = 0.25,
/// 全電流が単一エッジを通る → transport_efficiency(HHI) = 1.0。
#[test]
fn flow_solver_on_controlled_connected_network() {
    let mut p = Params::default();
    p.h = 9;
    p.w = 9;
    // 平坦・全陸の世界（E=0）
    let world = World {
        h: 9,
        w: 9,
        land_mask: vec![true; 81],
        e: vec![0.0f32; 81],
    };
    let mut st = initial_state(1, &world, &p);
    // trail をゼロにし、y=4 の x=2..=6 に閾値超の直線網を敷く
    for v in st.trail.iter_mut() {
        *v = 0.0;
    }
    for x in 2..=6 {
        st.trail[4 * 9 + x] = 1.0;
    }
    // 直線の両端に砂糖源を置く（source=id最小, sink=id最大）
    apply_op(&mut st, &Op::PlaceSugar { x: 2.5, y: 4.5, strength: 100.0 });
    apply_op(&mut st, &Op::PlaceSugar { x: 6.5, y: 4.5, strength: 100.0 });

    let a = analyze(&st, &world, &p).metrics;

    assert!(a.flow_connected, "連結網で flow_connected=false");
    assert!(a.effective_resistance > 0.0, "実効抵抗が非正");
    assert!(a.total_conductance > 0.0, "コンダクタンスが非正");
    assert!(
        a.transport_efficiency > 0.0 && a.transport_efficiency <= 1.0 + 1e-9,
        "transport_efficiency 値域外: {}",
        a.transport_efficiency
    );
    // 直列: tap(0.5) + 骨格エッジ実効長(4) + tap(0.5) = 5.0, HHI=1（単一エッジに全電流）
    assert!((a.effective_resistance - 5.0).abs() < 1e-6, "R={} (期待5)", a.effective_resistance);
    assert!((a.transport_efficiency - 1.0).abs() < 1e-9, "TE={} (期待1)", a.transport_efficiency);
}

/// #6 非侵襲: 解析実行前後で State と state_hash が不変。
#[test]
fn non_invasive() {
    for &seed in &SEEDS {
        let (p, w, st) = setup(seed);
        let h_before = state_hash(&st, &p);
        let _ = analyze(&st, &w, &p);
        let h_after = state_hash(&st, &p);
        assert_eq!(h_before, h_after, "seed {seed}: 解析が state_hash を変えた");
    }
}
