//! キャリブレーション計測ハーネス（規約の未確定数値を実測で決めるための自律ループの実体）。
//! 使い方: calibrate [max_seed] [ticks]
//!
//! 固定の代表シナリオ（テストと同一: 砂糖2源, 既定params）を seeds=1..=max_seed で回し、
//! 各メトリクスの分布（min/median/max/mean/std/MAD/相対ばらつき）と、
//! シード本数に対する中央値の収束を出力する。決定は人間（+この出力）が行う。

use nenkin_garden::metrics::compute_metrics;
use nenkin_garden::params::Params;
use nenkin_garden::state::{Op, ScriptEntry};
use nenkin_garden::world::make_synthetic_archipelago;
use nenkin_garden::run_headless;

fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

fn stats(name: &str, xs: &[f64]) {
    let mut s = xs.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = s.len() as f64;
    let med = median(&s);
    let mean = s.iter().sum::<f64>() / n;
    let var = s.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    let std = var.sqrt();
    // MAD（中央絶対偏差, ロバストなばらつき）
    let mut dev: Vec<f64> = s.iter().map(|v| (v - med).abs()).collect();
    dev.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mad = median(&dev);
    let rel_mad = if med.abs() > 1e-12 { mad / med.abs() * 100.0 } else { 0.0 };
    let rel_std = if med.abs() > 1e-12 { std / med.abs() * 100.0 } else { 0.0 };
    let p10 = s[((0.10 * (n - 1.0)).round() as usize).min(s.len() - 1)];
    let p90 = s[((0.90 * (n - 1.0)).round() as usize).min(s.len() - 1)];
    let rel_span = if med.abs() > 1e-12 { (p90 - p10) / med.abs() * 100.0 } else { 0.0 };
    println!(
        "{:<18} min={:>10.4} med={:>10.4} max={:>10.4} mean={:>10.4} std={:>9.4} MAD={:>8.4} | relMAD={:>6.1}% relStd={:>6.1}% p10-90/med={:>6.1}%",
        name, s[0], med, s[s.len() - 1], mean, std, mad, rel_mad, rel_std, rel_span
    );
}

fn median_convergence(name: &str, per_seed: &[f64]) {
    // 先頭 k シードの中央値が k とともにどう安定するか
    print!("{:<18} median over first k seeds: ", name);
    for &k in &[1usize, 3, 5, 7, 9, 11, 15, 21, 31, 41, 51, 63] {
        if k > per_seed.len() {
            break;
        }
        let mut sub = per_seed[..k].to_vec();
        sub.sort_by(|a, b| a.partial_cmp(b).unwrap());
        print!("k{}={:.4} ", k, median(&sub));
    }
    println!();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let max_seed: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(64);
    let ticks: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(160);

    let p = Params::default();
    let world = make_synthetic_archipelago(&p);
    // 代表シナリオ（テストと同一）
    let mut cells: Vec<usize> = (0..world.h * world.w)
        .filter(|&i| world.land_mask[i] && (world.e[i] as f64) < p.e_lo)
        .collect();
    if cells.is_empty() {
        cells = (0..world.h * world.w).filter(|&i| world.land_mask[i]).collect();
    }
    let c = cells[cells.len() / 2];
    let (sx, sy) = ((c % world.w) as f64 + 0.5, (c / world.w) as f64 + 0.5);
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx, y: sy, strength: 400.0 } },
        ScriptEntry { tick: 5, op: Op::PlaceSugar { x: sx + 6.0, y: sy + 4.0, strength: 300.0 } },
    ];

    let mut coverage = Vec::new();
    let mut sugar_rate = Vec::new();
    let mut max_cc = Vec::new();
    let mut num_cc = Vec::new();
    let mut elev_ratio = Vec::new();
    let mut mean_lo = Vec::new();
    let mut tick_ms = Vec::new();

    for seed in 1..=max_seed {
        let r = run_headless(seed, &script, ticks, &p, Some(&world));
        let m = compute_metrics(&r.final_state, &world, &p);
        coverage.push(m.coverage);
        sugar_rate.push(m.sugar_collected / ticks as f64);
        max_cc.push(m.max_cc as f64);
        num_cc.push(m.num_cc as f64);
        elev_ratio.push(m.elev_trail_ratio);
        mean_lo.push(m.mean_trail_lo);
        tick_ms.push(r.metrics.tick_ms);
    }

    println!("=== calibration: seeds=1..={max_seed}, ticks={ticks} (代表シナリオ) ===\n");
    println!("--- 分布（単一シードの自然ばらつき）---");
    stats("coverage", &coverage);
    stats("sugar_rate", &sugar_rate);
    stats("max_cc", &max_cc);
    stats("num_cc", &num_cc);
    stats("elev_trail_ratio", &elev_ratio);
    stats("mean_trail_lo", &mean_lo);
    stats("tick_ms", &tick_ms);

    println!("\n--- 中央値の収束（シード本数の決定用）---");
    median_convergence("coverage", &coverage);
    median_convergence("sugar_rate", &sugar_rate);
    median_convergence("max_cc", &max_cc);
    median_convergence("num_cc", &num_cc);
    median_convergence("mean_trail_lo", &mean_lo);

    // 決定論の確認: 各シードは再実行でビット一致（golden 粒度=hash の裏付け）
    let mut hash_ok = true;
    for seed in [1u64, 42, 1337] {
        let a = run_headless(seed, &script, ticks, &p, Some(&world)).final_state_hash;
        let b = run_headless(seed, &script, ticks, &p, Some(&world)).final_state_hash;
        if a != b {
            hash_ok = false;
        }
    }
    println!("\nfinal_state_hash 2回一致(1/42/1337): {}", hash_ok);
}
