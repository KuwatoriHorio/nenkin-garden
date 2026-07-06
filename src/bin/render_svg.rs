//! docs 用の静的 SVG 生成（読み取り専用の派生・render は State を読むだけ）。
//! 使い方: render_svg [seed] [ticks] [out.svg]
//! core を headless 実行し、終端 State の trail 網＋analysis の骨格を SVG に描く。

use std::fs;

use nenkin_garden::analysis::skeleton::{binarize, preserve_components, thin};
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

fn lerp(a: (f64, f64, f64), b: (f64, f64, f64), t: f64) -> (f64, f64, f64) {
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
    )
}

fn land_color(e: f64) -> (f64, f64, f64) {
    // 低: 緑 → 中: 土 → 高: 岩肌
    let low = (46.0, 92.0, 60.0);
    let mid = (120.0, 104.0, 66.0);
    let high = (168.0, 162.0, 156.0);
    if e < 0.5 {
        lerp(low, mid, e / 0.5)
    } else {
        lerp(mid, high, (e - 0.5) / 0.5)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(42);
    let ticks: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(160);
    let out = args.get(3).cloned().unwrap_or_else(|| "docs/network.svg".to_string());

    let p = Params::default();
    let world = make_synthetic_archipelago(&p);
    let (sx, sy) = land_coord(&p);
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx, y: sy, strength: 400.0 } },
        ScriptEntry { tick: 5, op: Op::PlaceSugar { x: sx + 6.0, y: sy + 4.0, strength: 300.0 } },
    ];
    let r = run_headless(seed, &script, ticks, &p, Some(&world));
    let st = &r.final_state;

    // analysis と同じ手順で骨格を抽出（オーバーレイ表示用）
    let mask = binarize(st, &world, &p);
    let mut skel = thin(&mask, world.h, world.w, p.skeleton_max_iter);
    preserve_components(&mask, &mut skel, world.h, world.w);

    let maxt = st.trail.iter().cloned().fold(0.0f32, f32::max).max(1e-6) as f64;

    let cell = 6.0;
    let (h, w) = (world.h, world.w);
    let width = w as f64 * cell;
    let height = h as f64 * cell;

    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" width=\"{width}\" height=\"{height}\" shape-rendering=\"crispEdges\">\n"
    ));
    // 海（背景）
    svg.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{width}\" height=\"{height}\" fill=\"#0b1e2d\"/>\n"
    ));

    let trail_glow = (124.0, 246.0, 152.0); // 網の発光色

    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if !world.land_mask[i] {
                continue;
            }
            let e = world.e[i] as f64;
            let base = land_color(e);
            let t = (st.trail[i] as f64 / maxt).clamp(0.0, 1.0);
            let a = (t * 1.6).min(1.0);
            let c = lerp(base, trail_glow, a);
            svg.push_str(&format!(
                "<rect x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" fill=\"rgb({},{},{})\"/>\n",
                x as f64 * cell,
                y as f64 * cell,
                cell,
                cell,
                c.0 as u8,
                c.1 as u8,
                c.2 as u8,
            ));
        }
    }

    // 抽出した骨格（analysis のグラフの元）をシアンで重ねる
    for y in 0..h {
        for x in 0..w {
            if skel[y * w + x] {
                svg.push_str(&format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"#38f0e0\" opacity=\"0.9\"/>\n",
                    x as f64 * cell + cell * 0.25,
                    y as f64 * cell + cell * 0.25,
                    cell * 0.5,
                    cell * 0.5,
                ));
            }
        }
    }

    // 砂糖源（赤）
    for k in 0..st.sugar_x.len() {
        svg.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"#ff5a5a\" stroke=\"#fff\" stroke-width=\"1\"/>\n",
            st.sugar_x[k] * cell,
            st.sugar_y[k] * cell,
            cell * 1.1,
        ));
    }

    svg.push_str("</svg>\n");
    fs::write(&out, svg).expect("write svg");
    println!("wrote {out} (seed={seed}, ticks={ticks}, max_trail={maxt:.4})");
}
