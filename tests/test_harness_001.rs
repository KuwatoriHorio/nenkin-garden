//! test-harness-001: 正準シード集合 S（9本）と中央値ソフトゲートをテストへ反映（規約 §4）。
//!
//! - Tier 0（§3 不変条件）を **S 全9本** で検証（有限性/保存則/境界/再現性/ソフト標高忌避）。
//! - Tier 2（§4 メトリクス）を **S 上の中央値・方向つき比較・許容%** で baseline と比較する
//!   共通ヘルパ soft_gate を提供。baseline は固定の参照値（前回green中央値）を定数で持つ。
//!
//! 既存の core-000 / analysis-001 テストは弱めない（本ファイルは被覆と集計法を追加するだけ）。

use nenkin_garden::metrics::compute_metrics;
use nenkin_garden::params::Params;
use nenkin_garden::state::{Op, ScriptEntry};
use nenkin_garden::world::{make_synthetic_archipelago, World};
use nenkin_garden::run_headless;

/// 規約 §4 の正準シード集合（奇数9本）。
const S9: [u64; 9] = [1, 7, 13, 42, 99, 256, 1337, 2024, 31337];
const TICKS: u64 = 160;
const EPS_CONSERVE: f64 = 1.0e-4;

fn world_and_script() -> (Params, World, Vec<ScriptEntry>) {
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
    (p, w, script)
}

/// 奇数本の中央値（S9 は 9 本なので中央一意）。
fn median(mut xs: Vec<f64>) -> f64 {
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs[xs.len() / 2]
}

// ---------- Tier 0: 不変条件を S 全9本で（§3, acceptance #1/#2） ----------

#[test]
fn tier0_invariants_all_9_seeds() {
    let (p, w, script) = world_and_script();
    for &seed in &S9 {
        let r = run_headless(seed, &script, TICKS, &p, Some(&w));
        let s = &r.final_state;

        // 有限性
        assert!(s.trail.iter().all(|v| v.is_finite()), "seed {seed}: trail 非有限");
        assert!(
            s.ax.iter().chain(&s.ay).chain(&s.ah).all(|v| v.is_finite()),
            "seed {seed}: agent 座標非有限"
        );

        // 保存則
        assert!(s.biomass >= -EPS_CONSERVE, "seed {seed}: biomass 負");
        assert!(
            (s.biomass - (s.collected_total - s.consumed_total)).abs() <= EPS_CONSERVE,
            "seed {seed}: 保存則破れ"
        );

        // 境界（範囲内かつ陸上）
        for i in 0..s.n_agents() {
            let (x, y) = (s.ax[i] as f64, s.ay[i] as f64);
            assert!(x >= 0.0 && x < w.w as f64 && y >= 0.0 && y < w.h as f64, "seed {seed}: 範囲外");
            let cix = (x.floor() as usize).min(w.w - 1);
            let ciy = (y.floor() as usize).min(w.h - 1);
            assert!(w.land_mask[ciy * w.w + cix], "seed {seed}: 海上のエージェント");
        }

        // ソフト標高忌避
        let m = compute_metrics(s, &w, &p);
        assert!(m.mean_trail_lo > 0.0, "seed {seed}: 低標高帯に trail 無し");
        assert!(m.mean_trail_hi < m.mean_trail_lo, "seed {seed}: ソフト標高忌避違反");

        // 再現性（同一入力2回で final_state_hash 一致）
        let h2 = run_headless(seed, &script, TICKS, &p, Some(&w)).final_state_hash;
        assert_eq!(r.final_state_hash, h2, "seed {seed}: hash 再現性違反");
    }
}

// ---------- Tier 2: §4 メトリクスの中央値ソフトゲート（共通ヘルパ, acceptance #3） ----------

/// メトリクスの劣化方向。
#[derive(Clone, Copy)]
enum Dir {
    LowerWorse,  // 低いほど悪い（被覆・砂糖・max_cc・mean_trail_lo）
    HigherWorse, // 高いほど悪い（num_cc・elev_avoidance・tick_ms）
}

/// S 上の中央値を baseline と方向つき・許容% で比較する共通ヘルパ（§4 の集計法）。
fn soft_gate(name: &str, current_median: f64, baseline: f64, tol: f64, dir: Dir) {
    match dir {
        Dir::LowerWorse => assert!(
            current_median >= baseline * (1.0 - tol),
            "{name}: 中央値 {current_median} が基準 {baseline} 比 -{:.0}% を下回り劣化",
            tol * 100.0
        ),
        Dir::HigherWorse => assert!(
            current_median <= baseline * (1.0 + tol),
            "{name}: 中央値 {current_median} が基準 {baseline} 比 +{:.0}% を上回り劣化",
            tol * 100.0
        ),
    }
}

// baseline（前回green中央値, S9・160tick・既定params で実測固定）。
// 「意図した挙動変更」時のみ理由付きで更新すること（§5/§7）。
const BASE_COVERAGE: f64 = 0.029240;
const BASE_SUGAR_RATE: f64 = 0.090625;
const BASE_MAX_CC: f64 = 10.0;
const BASE_NUM_CC: f64 = 52.0;
const BASE_ELEV_AVOIDANCE: f64 = 0.126969;
const BASE_MEAN_TRAIL_LO: f64 = 0.058124;

#[test]
fn tier2_metric_softgates_median_over_9_seeds() {
    let (p, w, script) = world_and_script();
    let (mut cov, mut sug, mut mcc, mut ncc, mut avoid, mut mlo) =
        (vec![], vec![], vec![], vec![], vec![], vec![]);
    for &seed in &S9 {
        let r = run_headless(seed, &script, TICKS, &p, Some(&w));
        let m = compute_metrics(&r.final_state, &w, &p);
        cov.push(m.coverage);
        sug.push(m.sugar_collected / TICKS as f64);
        mcc.push(m.max_cc as f64);
        ncc.push(m.num_cc as f64);
        avoid.push(m.elev_avoidance);
        mlo.push(m.mean_trail_lo);
    }
    // §4 の許容%・方向で基準比較（中央値）
    soft_gate("coverage", median(cov), BASE_COVERAGE, 0.08, Dir::LowerWorse);
    soft_gate("sugar_rate", median(sug), BASE_SUGAR_RATE, 0.18, Dir::LowerWorse);
    soft_gate("max_cc", median(mcc), BASE_MAX_CC, 0.18, Dir::LowerWorse);
    soft_gate("num_cc", median(ncc), BASE_NUM_CC, 0.10, Dir::HigherWorse);
    soft_gate("elev_avoidance", median(avoid), BASE_ELEV_AVOIDANCE, 0.08, Dir::HigherWorse);
    soft_gate("mean_trail_lo", median(mlo), BASE_MEAN_TRAIL_LO, 0.12, Dir::LowerWorse);
}
