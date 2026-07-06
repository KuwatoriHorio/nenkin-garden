//! run_analysis CLI（analysis-001）。
//! 使い方: run_analysis [seed] [ticks]
//! core を headless 実行 → 終端 State に対し静的ネットワーク解析 → analysis.json 出力。

use std::fs;

use nenkin_garden::analysis::analyze;
use nenkin_garden::params::Params;
use nenkin_garden::state::{Op, ScriptEntry};
use nenkin_garden::world::make_synthetic_archipelago;
use nenkin_garden::{run_headless, state_hash};

fn land_coord(p: &Params) -> (f64, f64) {
    let w = make_synthetic_archipelago(p);
    let mut cells: Vec<usize> = (0..w.h * w.w)
        .filter(|&i| w.land_mask[i] && (w.e[i] as f64) < p.e_lo)
        .collect();
    if cells.is_empty() {
        cells = (0..w.h * w.w).filter(|&i| w.land_mask[i]).collect();
    }
    let c = cells[cells.len() / 2];
    ((c % w.w) as f64 + 0.5, (c / w.w) as f64 + 0.5)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let ticks: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(160);

    let p = Params::default();
    let world = make_synthetic_archipelago(&p);
    let (sx, sy) = land_coord(&p);
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx, y: sy, strength: 400.0 } },
        ScriptEntry { tick: 5, op: Op::PlaceSugar { x: sx + 6.0, y: sy + 4.0, strength: 300.0 } },
    ];

    let r = run_headless(seed, &script, ticks, &p, Some(&world));

    // 非侵襲の確認: 解析前後で state_hash 不変
    let h_before = state_hash(&r.final_state, &p);
    let a = analyze(&r.final_state, &world, &p);
    let h_after = state_hash(&r.final_state, &p);

    let json = a.metrics.to_json();
    fs::write("analysis.json", &json).expect("write analysis.json");

    println!("seed={} ticks={}", seed, ticks);
    println!("state_hash before/after analysis: {:#018x} / {:#018x}", h_before, h_after);
    println!("analysis={}", json);
}
