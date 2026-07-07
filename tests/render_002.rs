//! render-002 受け入れテスト（tasks/task-render-002.md）。
//! グラフの静的 SVG が決定的・非侵襲で、analysis のノード/エッジ数と整合することを検証する。

use nenkin_garden::graph_svg::{flow_width, graph_to_svg};
use nenkin_garden::analysis::analyze;
use nenkin_garden::hash::state_hash;
use nenkin_garden::params::Params;
use nenkin_garden::state::{Op, ScriptEntry};
use nenkin_garden::world::make_synthetic_archipelago;
use nenkin_garden::run_headless;

fn final_state_scenario(seed: u64) -> (Params, nenkin_garden::world::World, nenkin_garden::state::State) {
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
    let r = run_headless(seed, &script, 160, &p, Some(&w));
    (p, w, r.final_state)
}

fn attr<'a>(svg: &'a str, key: &str) -> &'a str {
    let pat = format!("{key}=\"");
    let s = svg.find(&pat).expect("attr present") + pat.len();
    let e = svg[s..].find('"').unwrap() + s;
    &svg[s..e]
}

/// #4 流量 → 線幅 の写像が単調（0 で最小, max で最大, 単調増加）。
#[test]
fn flow_width_is_monotonic() {
    let max = 2.0;
    assert!(flow_width(0.0, max) < flow_width(1.0, max));
    assert!(flow_width(1.0, max) < flow_width(2.0, max));
    // 最小/最大の端点
    assert!(flow_width(0.0, max) <= flow_width(0.5, max));
    assert!(flow_width(2.0, max) >= flow_width(1.9, max));
    // max=0（流れ無し）でも有限
    assert!(flow_width(0.0, 0.0).is_finite());
}

/// #1 決定性: 同一 State から2回生成 → バイト一致。
#[test]
fn svg_is_deterministic() {
    for &seed in &[1u64, 42, 1337] {
        let (p, w, st) = final_state_scenario(seed);
        let a = graph_to_svg(&st, &w, &p);
        let b = graph_to_svg(&st, &w, &p);
        assert_eq!(a, b, "seed {seed}: SVG がバイト不一致");
        assert!(a.starts_with("<svg"), "SVG ヘッダ不正");
    }
}

/// #2 非侵襲: 生成前後で state_hash 不変。
#[test]
fn svg_is_non_invasive() {
    let (p, w, st) = final_state_scenario(42);
    let h0 = state_hash(&st, &p);
    let _ = graph_to_svg(&st, &w, &p);
    assert_eq!(h0, state_hash(&st, &p), "生成が state_hash を変えた");
}

/// #3 グラフ整合: SVG の data-nodes/data-edges が analysis の nodes/edges と一致。
#[test]
fn svg_matches_analysis_graph_size() {
    for &seed in &[1u64, 42, 1337] {
        let (p, w, st) = final_state_scenario(seed);
        let m = analyze(&st, &w, &p).metrics;
        let svg = graph_to_svg(&st, &w, &p);
        assert_eq!(attr(&svg, "data-nodes"), m.nodes.to_string(), "seed {seed}: nodes 不一致");
        assert_eq!(attr(&svg, "data-edges"), m.edges.to_string(), "seed {seed}: edges 不一致");
    }
}
