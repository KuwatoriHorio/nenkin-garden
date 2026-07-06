//! run_headless CLI（設計メモ §7）。
//! 使い方: run_headless [seed] [ticks]
//! 既定シナリオを実行し、metrics.json を書き出して final_state_hash を表示する。

use std::fs;

use nenkin_garden::params::Params;
use nenkin_garden::state::{Op, ScriptEntry};
use nenkin_garden::world::make_synthetic_archipelago;
use nenkin_garden::{run_headless, State};

/// 低〜中標高の陸セルを1つ選び砂糖設置座標にする（決定的）。
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
    let (sx, sy) = land_coord(&p);
    let script = vec![
        ScriptEntry {
            tick: 0,
            op: Op::PlaceSugar {
                x: sx,
                y: sy,
                strength: 400.0,
            },
        },
        ScriptEntry {
            tick: 5,
            op: Op::PlaceSugar {
                x: sx + 6.0,
                y: sy + 4.0,
                strength: 300.0,
            },
        },
    ];

    let r = run_headless(seed, &script, ticks, &p, None);

    let json = r.metrics.to_json();
    fs::write("metrics.json", &json).expect("write metrics.json");

    // 出力: metrics.json（ファイル）+ final_state_hash（stdout）
    let _final: &State = &r.final_state;
    println!("seed={} ticks={}", seed, ticks);
    println!("final_state_hash={:#018x}", r.final_state_hash);
    println!("metrics={}", json);
}
