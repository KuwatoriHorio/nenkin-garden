//! run_tree CLI（tree-growth-001）。α成長木モデルのヘッドレス実行。
//! 使い方: run_tree [seed] [ticks]
//! 現行 run_headless / Jones モデルとは無関係（並置の新モデル）。

use nenkin_garden::params::Params;
use nenkin_garden::state::{Op, ScriptEntry};
use nenkin_garden::tree::{run_tree_headless, total_volume, TreeParams};
use nenkin_garden::world::{make_synthetic_archipelago, World};

/// 提案座標が海/範囲外なら、最も近い陸セル中心へ決定的にスナップする。
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

    // world 生成は現行 Params 由来（h/w のみ使用・読み取り利用）。
    let world_params = Params::default();
    let world = make_synthetic_archipelago(&world_params);

    let tp = TreeParams::default();
    let (hx, hy) = world.default_home(tp.e_lo);
    let (s1x, s1y) = nearest_land(&world, hx + 12.0, hy);
    let (s2x, s2y) = nearest_land(&world, hx - 8.0, hy + 8.0);
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: s1x, y: s1y, strength: 400.0 } },
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: s2x, y: s2y, strength: 300.0 } },
    ];

    let r = run_tree_headless(seed, &script, ticks, &tp, &world);
    let s = &r.final_state;

    println!("seed={} ticks={}", seed, ticks);
    println!("nodes={}", s.n_nodes());
    println!("b_free={:.4}", s.b_free);
    println!("collected_total={:.4} consumed_total={:.4}", s.collected_total, s.consumed_total);
    println!("total_volume={:.4}", total_volume(s, tp.k));
    println!("final_state_hash={:#018x}", r.final_state_hash);

    for (sx, sy) in [(s1x, s1y), (s2x, s2y)] {
        let mut best = f64::INFINITY;
        for node in &s.nodes {
            let dx = node.x as f64 - sx;
            let dy = node.y as f64 - sy;
            let d = (dx * dx + dy * dy).sqrt();
            if d < best {
                best = d;
            }
        }
        println!("sugar=({:.1},{:.1}) nearest_node_dist={:.3}", sx, sy, best);
    }
    let n_branch = s
        .nodes
        .iter()
        .enumerate()
        .filter(|(i, _)| s.nodes.iter().filter(|n| n.parent == Some(*i)).count() >= 2)
        .count();
    println!("branch_nodes(>=2 children)={}", n_branch);
}
