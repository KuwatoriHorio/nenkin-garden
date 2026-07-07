//! render_graph_svg CLI（render-002）。
//! 使い方: render_graph_svg [seed] [ticks] [out.svg]
//! core を headless 実行 → 終端 State の解析グラフを静的 SVG に描き出す（非侵襲）。

use std::fs;

use nenkin_garden::graph_svg::graph_to_svg;
use nenkin_garden::params::Params;
use nenkin_garden::state::{Op, ScriptEntry};
use nenkin_garden::world::make_synthetic_archipelago;
use nenkin_garden::run_headless;

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
    let seed: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(42);
    let ticks: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(160);
    let out = args.get(3).cloned().unwrap_or_else(|| "docs/network_graph.svg".to_string());

    let p = Params::default();
    let world = make_synthetic_archipelago(&p);
    let (sx, sy) = land_coord(&p);
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx, y: sy, strength: 400.0 } },
        ScriptEntry { tick: 5, op: Op::PlaceSugar { x: sx + 6.0, y: sy + 4.0, strength: 300.0 } },
    ];

    let r = run_headless(seed, &script, ticks, &p, Some(&world));
    let svg = graph_to_svg(&r.final_state, &world, &p);
    fs::write(&out, &svg).expect("write svg");
    println!("wrote {out} (seed={seed}, ticks={ticks}, {} bytes)", svg.len());
}
