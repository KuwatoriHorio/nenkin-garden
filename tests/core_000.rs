//! core-000 受け入れ・不変条件テスト（規約 §3/§10/§11, 設計メモ §10/§11）。
//! seeds = [1, 42, 1337]。各アサートは規約 §3 の項目に対応。

use nenkin_garden::hash::state_hash;
use nenkin_garden::metrics::compute_metrics;
use nenkin_garden::params::Params;
use nenkin_garden::state::{Op, ScriptEntry};
use nenkin_garden::world::{make_synthetic_archipelago, World};
use nenkin_garden::run_headless;

const SEEDS: [u64; 3] = [1, 42, 1337];
const TICKS: u64 = 160;

fn params() -> Params {
    Params::default()
}

fn world(p: &Params) -> World {
    make_synthetic_archipelago(p)
}

/// 低〜中標高の陸セルを1つ選ぶ（決定的）。
fn land_coord(p: &Params, w: &World) -> (f64, f64) {
    let mut cells: Vec<usize> = (0..w.h * w.w)
        .filter(|&i| w.land_mask[i] && (w.e[i] as f64) < p.e_lo)
        .collect();
    if cells.is_empty() {
        cells = (0..w.h * w.w).filter(|&i| w.land_mask[i]).collect();
    }
    let c = cells[cells.len() / 2];
    ((c % w.w) as f64 + 0.5, (c / w.w) as f64 + 0.5)
}

fn script(p: &Params, w: &World) -> Vec<ScriptEntry> {
    let (sx, sy) = land_coord(p, w);
    vec![
        ScriptEntry {
            tick: 0,
            op: Op::PlaceSugar { x: sx, y: sy, strength: 400.0 },
        },
        ScriptEntry {
            tick: 5,
            op: Op::PlaceSugar { x: sx + 6.0, y: sy + 4.0, strength: 300.0 },
        },
    ]
}

// ---------- §11 受け入れテスト ----------

#[test]
fn hash_reproducible_twice() {
    // 同一(seed, input_script, ticks) で final_state_hash が2回一致（3シード）。
    let p = params();
    let w = world(&p);
    let s = script(&p, &w);
    for &seed in &SEEDS {
        let h1 = run_headless(seed, &s, TICKS, &p, Some(&w)).final_state_hash;
        let h2 = run_headless(seed, &s, TICKS, &p, Some(&w)).final_state_hash;
        assert_eq!(h1, h2, "seed {seed}: hash が2回で不一致");
    }
}

#[test]
fn run_headless_outputs() {
    // run_headless が metrics と final_state_hash を出力する。
    let p = params();
    let w = world(&p);
    let s = script(&p, &w);
    let r = run_headless(1, &s, TICKS, &p, Some(&w));
    assert!(r.metrics.n_agents > 0);
    let json = r.metrics.to_json();
    assert!(json.contains("coverage"));
    assert!(json.contains("max_cc"));
    assert!(json.contains("elev_trail_ratio"));
    assert!(json.contains("tick_ms"));
}

#[test]
fn render_off_on_hash_invariant() {
    // 描画は未実装＝常にOFF。派生読み取り（メトリクス算出）は State を書き換えない
    // ため、実行前後で state_hash が不変であることを検証する（analysis/render の契約）。
    let p = params();
    let w = world(&p);
    let s = script(&p, &w);
    let r = run_headless(1, &s, TICKS, &p, Some(&w));
    let h_before = state_hash(&r.final_state, &p);
    let _ = compute_metrics(&r.final_state, &w, &p); // 読み取りのみ
    let h_after = state_hash(&r.final_state, &p);
    assert_eq!(h_before, h_after);
    assert_eq!(h_before, r.final_state_hash);
}

// ---------- §10 不変条件（3シード） ----------

#[test]
fn invariants_all_seeds() {
    let p = params();
    let w = world(&p);
    let s = script(&p, &w);
    for &seed in &SEEDS {
        let r = run_headless(seed, &s, TICKS, &p, Some(&w));
        let st = &r.final_state;

        // 有限性
        assert!(st.trail.iter().all(|v| v.is_finite()), "seed {seed}: trail 非有限");
        assert!(
            st.ax.iter().chain(&st.ay).chain(&st.ah).all(|v| v.is_finite()),
            "seed {seed}: agent 座標非有限"
        );

        // 保存則: biomass = collected - consumed, 非負
        assert!(st.biomass >= -p.eps_conserve, "seed {seed}: biomass 負");
        let identity = st.collected_total - st.consumed_total;
        assert!(
            (st.biomass - identity).abs() <= p.eps_conserve,
            "seed {seed}: 保存則破れ biomass={} collected-consumed={}",
            st.biomass,
            identity
        );

        // 境界: 全エージェントが範囲内かつ陸上
        for i in 0..st.n_agents() {
            let x = st.ax[i] as f64;
            let y = st.ay[i] as f64;
            assert!(x >= 0.0 && x < w.w as f64 && y >= 0.0 && y < w.h as f64, "seed {seed}: 範囲外");
            let cix = (x.floor() as usize).min(w.w - 1);
            let ciy = (y.floor() as usize).min(w.h - 1);
            assert!(w.land_mask[ciy * w.w + cix], "seed {seed}: 海上のエージェント");
        }

        // 標高忌避（ソフト）: 高標高帯の平均trail < 低標高帯の平均trail
        let m = compute_metrics(st, &w, &p);
        assert!(m.mean_trail_lo > 0.0, "seed {seed}: 低標高帯に trail 無し");
        assert!(
            m.mean_trail_hi < m.mean_trail_lo,
            "seed {seed}: 標高忌避不成立 hi={} lo={}",
            m.mean_trail_hi,
            m.mean_trail_lo
        );
    }
}

#[test]
fn seeds_differ() {
    // 異なるシードは（通常）異なるハッシュ＝シードが実際に効いている。
    let p = params();
    let w = world(&p);
    let s = script(&p, &w);
    let hashes: Vec<u64> = SEEDS
        .iter()
        .map(|&seed| run_headless(seed, &s, TICKS, &p, Some(&w)).final_state_hash)
        .collect();
    let mut uniq = hashes.clone();
    uniq.sort_unstable();
    uniq.dedup();
    assert_eq!(uniq.len(), SEEDS.len(), "シード間でハッシュが衝突");
}
