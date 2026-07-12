//! run_netphys CLI（netphys-001）。網 Physarum（担体A・Tero純）のヘッドレス実行。
//! 使い方: run_netphys [seed] [ticks]
//! 現行 run_headless / run_tree とは無関係（並置の第3の新モデル）。

use nenkin_garden::netphys::{run_netphys_headless, total_mass, NetParams};
use nenkin_garden::params::Params;
use nenkin_garden::state::{Op, ScriptEntry};
use nenkin_garden::world::{make_synthetic_archipelago, World};

fn nearest_land(world: &World, x: f64, y: f64) -> (f64, f64) {
    let fx = x.floor();
    let fy = y.floor();
    let inb = fx >= 0.0 && fx < world.w as f64 && fy >= 0.0 && fy < world.h as f64;
    if inb && world.land_mask[(fy as usize) * world.w + (fx as usize)] {
        return (x, y);
    }
    let mut best: Option<(f64, usize)> = None;
    for cy in 0..world.h {
        for cx in 0..world.w {
            let i = cy * world.w + cx;
            if !world.land_mask[i] {
                continue;
            }
            let cxf = cx as f64 + 0.5;
            let cyf = cy as f64 + 0.5;
            let d = (cxf - x).powi(2) + (cyf - y).powi(2);
            if best.map(|(bd, _)| d < bd).unwrap_or(true) {
                best = Some((d, i));
            }
        }
    }
    let i = best.expect("world has no land cells").1;
    ((i % world.w) as f64 + 0.5, (i / world.w) as f64 + 0.5)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let ticks: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(200);

    let world_params = Params::default();
    let world = make_synthetic_archipelago(&world_params);

    let np = NetParams::default();
    let (hx, hy) = world.default_home(np.e_lo);
    let (s1x, s1y) = nearest_land(&world, hx + 20.0, hy);
    let (s2x, s2y) = nearest_land(&world, hx - 14.0, hy + 14.0);
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: s1x, y: s1y, strength: 400.0 } },
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: s2x, y: s2y, strength: 300.0 } },
    ];

    let r = run_netphys_headless(seed, &script, ticks, &np, &world);
    let s = &r.final_state;

    println!("seed={} ticks={}", seed, ticks);
    println!("nodes={} edges={}", s.n_nodes(), s.n_edges());
    println!("free_budget={:.4}", s.free_budget);
    println!("collected_total={:.4} consumed_total={:.4}", s.collected_total, s.consumed_total);
    println!("total_mass={:.4}", total_mass(s));
    println!("final_state_hash={:#018x}", r.final_state_hash);
    println!("frontier={:?}", s.frontier);
    let redundancy = if s.n_nodes() > 1 {
        s.n_edges() as f64 - (s.n_nodes() as f64 - 1.0)
    } else {
        0.0
    };
    println!("edges - (nodes-1) = {:.1} (>0 なら少なくとも1ループ)", redundancy);
}
