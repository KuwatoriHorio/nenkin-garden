//! metric-thresholds-001 受け入れテスト（tasks/task-metric-thresholds-001.md）。
//! 退化した忌避健全性指標を、常に定義される連続指標へ見直したことを検証する。
//! しきい θ_cov/θ_cc は分解能を維持（正準9シードで relMAD>0）。core 不変条件は不可侵。

use nenkin_garden::analysis::analyze;
use nenkin_garden::metrics::compute_metrics;
use nenkin_garden::params::Params;
use nenkin_garden::state::{Op, ScriptEntry};
use nenkin_garden::world::{make_synthetic_archipelago, World};
use nenkin_garden::run_headless;

const S9: [u64; 9] = [1, 7, 13, 42, 99, 256, 1337, 2024, 31337];
const TICKS: u64 = 160;

fn scenario(p: &Params, w: &World) -> Vec<ScriptEntry> {
    let mut cells: Vec<usize> = (0..w.h * w.w)
        .filter(|&i| w.land_mask[i] && (w.e[i] as f64) < p.e_lo)
        .collect();
    if cells.is_empty() {
        cells = (0..w.h * w.w).filter(|&i| w.land_mask[i]).collect();
    }
    let c = cells[cells.len() / 2];
    let (sx, sy) = ((c % w.w) as f64 + 0.5, (c / w.w) as f64 + 0.5);
    vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx, y: sy, strength: 400.0 } },
        ScriptEntry { tick: 5, op: Op::PlaceSugar { x: sx + 6.0, y: sy + 4.0, strength: 300.0 } },
    ]
}

fn median(mut xs: Vec<f64>) -> f64 {
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = xs.len();
    if n % 2 == 1 { xs[n / 2] } else { (xs[n / 2 - 1] + xs[n / 2]) / 2.0 }
}

fn rel_mad(xs: &[f64]) -> f64 {
    let med = median(xs.to_vec());
    let dev: Vec<f64> = xs.iter().map(|v| (v - med).abs()).collect();
    let mad = median(dev);
    if med.abs() > 1e-12 { mad / med.abs() } else { 0.0 }
}

/// #2 忌避の健全性指標が退化していない: 常に正・連続で、忌避を捉える（加重平均標高<陸地平均）。
#[test]
fn health_metric_robust_and_non_degenerate() {
    let p = Params::default();
    let w = make_synthetic_archipelago(&p);
    let script = scenario(&p, &w);

    let mut twe = Vec::new();
    let mut avoid = Vec::new();
    for &seed in &S9 {
        let r = run_headless(seed, &script, TICKS, &p, Some(&w));
        let m = compute_metrics(&r.final_state, &w, &p);
        // 退化していない（elev_trail_ratio は常に0だったが、こちらは正の連続値）
        assert!(m.trail_weighted_mean_elevation > 0.0, "seed {seed}: 加重平均標高が0（退化）");
        assert!(m.land_mean_elevation > 0.0, "seed {seed}: 陸地平均標高が0");
        twe.push(m.trail_weighted_mean_elevation);
        avoid.push(m.elev_avoidance);
    }
    // 忌避が網加重レベルで効く: 中央値で avoidance < 1（trail は低標高に偏る）
    assert!(median(avoid) < 1.0, "忌避健全性: 加重平均標高が陸地平均以上（中央値）");
    // 連続指標としてばらつきを持つ（0固定でない）
    assert!(rel_mad(&twe) > 0.0, "加重平均標高が単一値に潰れている");
}

/// #1 coverage / num_cc / max_cc が分解能を持つ（relMAD>0, 正準9シード）。
#[test]
fn coverage_and_cc_have_resolution() {
    let p = Params::default();
    let w = make_synthetic_archipelago(&p);
    let script = scenario(&p, &w);

    let mut cov = Vec::new();
    let mut ncc = Vec::new();
    let mut mcc = Vec::new();
    for &seed in &S9 {
        let r = run_headless(seed, &script, TICKS, &p, Some(&w));
        let m = compute_metrics(&r.final_state, &w, &p);
        cov.push(m.coverage);
        ncc.push(m.num_cc as f64);
        mcc.push(m.max_cc as f64);
    }
    assert!(rel_mad(&cov) > 0.0, "coverage が潰れている");
    assert!(rel_mad(&ncc) > 0.0, "num_cc が潰れている");
    assert!(rel_mad(&mcc) > 0.0, "max_cc が潰れている");
}

/// #2(後半)/#3 不変条件 mean_hi<mean_lo 維持 & analysis num_cc == core num_cc（θ_cc共有）。
#[test]
fn invariant_and_core_consistency() {
    let p = Params::default();
    let w = make_synthetic_archipelago(&p);
    let script = scenario(&p, &w);
    for &seed in &S9 {
        let r = run_headless(seed, &script, TICKS, &p, Some(&w));
        let m = compute_metrics(&r.final_state, &w, &p);
        assert!(m.mean_trail_hi < m.mean_trail_lo, "seed {seed}: ソフト標高忌避(不変条件)違反");
        let a = analyze(&r.final_state, &w, &p).metrics;
        assert_eq!(a.num_cc, m.num_cc, "seed {seed}: analysis と core の num_cc 不整合");
    }
}
