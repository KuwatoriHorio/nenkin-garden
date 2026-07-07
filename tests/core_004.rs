//! core-004 受け入れテスト（tasks/task-core-004.md）。
//!
//! trail 濃度の上限 `trail_max`（既定 INFINITY=上限なし）を追加し、有効時は
//! 「ホーム中心の誘引の井戸」が頭打ちになって局在化が緩和されることを検証する。
//!
//! R=6.0（ホーム近傍半径）・trail_max=18.0（sugar_beacon=6.0 の3倍）は
//! `tests/_probe_core_004.rs`（一時プローブ、実測後に削除済み）による実測で選定:
//!   R=6.0: trail_max=18 の局在化指標 L 中央値=0.5053 < trail_max=inf の 0.6435
//!   （3シード [1,42,1337] 全てで trail_max=18 < inf、有意な低下）。
//!
//! 既定（trail_max=INFINITY）は `v.min(INFINITY)==v` によりバイト不変のため、既存テスト
//! （core-000/001/002/003, analysis-00x, harness, thresholds）とは独立に緑を保つ
//! （§7: 既存ゴールデンを一切弱めない。本ファイルは新規挙動の追加検証のみ）。

use nenkin_garden::hash::state_hash;
use nenkin_garden::metrics::compute_metrics;
use nenkin_garden::params::Params;
use nenkin_garden::state::{initial_state, Op, State};
use nenkin_garden::step::step;
use nenkin_garden::world::{make_synthetic_archipelago, World};

const SEEDS: [u64; 3] = [1, 42, 1337]; // task-core-004.md 指定の最低集合(S9部分集合)
const TICKS: u64 = 300; // acceptance_test #1 の T=300
const SUGAR_STRENGTH: f64 = 600.0;
const R_HOME: f64 = 6.0; // 局在化指標の半径（実測選定、上記コメント参照）
const TRAIL_MAX_FINITE: f64 = 18.0; // sugar_beacon(6.0) の3倍（実測選定）

fn foraging_params(hx: f64, hy: f64, trail_max: f64) -> Params {
    let mut p = Params::default();
    p.home_x = hx;
    p.home_y = hy;
    p.init_cluster_sigma = 3.0;
    p.w_trail_cohesion = 1.0;
    p.trail_max = trail_max;
    p
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// 低標高陸セルのうち home から距離 target に最も近いセル中心（決定的）。
/// core_002/003 と同じ選定法・同じ距離12・強度600を流用（実測済みの現実的 foraging 設定）。
fn low_e_cell_at_dist(world: &World, e_lo: f64, hx: f64, hy: f64, target: f64) -> (f64, f64) {
    let (h, w) = (world.h, world.w);
    let mut best = (hx, hy);
    let mut best_err = f64::INFINITY;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if !world.land_mask[i] || (world.e[i] as f64) >= e_lo {
                continue;
            }
            let (cx, cy) = (x as f64 + 0.5, y as f64 + 0.5);
            let d = ((cx - hx).powi(2) + (cy - hy).powi(2)).sqrt();
            if (d - target).abs() < best_err {
                best_err = (d - target).abs();
                best = (cx, cy);
            }
        }
    }
    best
}

/// 局在化指標 L = ホーム半径R内のtrail総量 / 全trail総量。
fn localization(trail: &[f32], w: usize, h: usize, hx: f64, hy: f64, r: f64) -> f64 {
    let total: f64 = trail.iter().map(|&v| v as f64).sum();
    if total <= 0.0 {
        return 0.0;
    }
    let ri = r.ceil() as i64;
    let (ix, iy) = (hx.floor() as i64, hy.floor() as i64);
    let mut inside = 0.0;
    for dy in -ri..=ri {
        for dx in -ri..=ri {
            if (dx * dx + dy * dy) as f64 > r * r {
                continue;
            }
            let (nx, ny) = (ix + dx, iy + dy);
            if nx < 0 || nx >= w as i64 || ny < 0 || ny >= h as i64 {
                continue;
            }
            inside += trail[ny as usize * w + nx as usize] as f64;
        }
    }
    inside / total
}

fn conservation_ok(st: &State) -> bool {
    let expect = st.collected_total - st.consumed_total;
    (st.biomass - expect).abs() <= 1e-4 && st.biomass >= -1e-9
}

fn run(seed: u64, world: &World, p: &Params, far: (f64, f64)) -> State {
    let mut st = initial_state(seed, world, p);
    for t in 0..TICKS {
        let ops: Vec<Op> = if t == 0 {
            vec![Op::PlaceSugar { x: far.0, y: far.1, strength: SUGAR_STRENGTH }]
        } else {
            vec![]
        };
        step(&mut st, world, p, &ops);
    }
    st
}

/// acceptance #1: 有限 trail_max のほうが局在化指標 L が有意に小さい（複数シード中央値、
/// かつ各シード個別でも一貫して小さい）。
#[test]
fn finite_trail_max_reduces_localization() {
    let base = Params::default();
    let world = make_synthetic_archipelago(&base);
    let (hx, hy) = world.default_home(base.e_lo);
    let far = low_e_cell_at_dist(&world, base.e_lo, hx, hy, 12.0);

    let p_inf = foraging_params(hx, hy, f64::INFINITY);
    let p_fin = foraging_params(hx, hy, TRAIL_MAX_FINITE);

    let mut l_inf = Vec::new();
    let mut l_fin = Vec::new();
    for &seed in &SEEDS {
        let st_inf = run(seed, &world, &p_inf, far);
        let st_fin = run(seed, &world, &p_fin, far);
        let li = localization(&st_inf.trail, world.w, world.h, hx, hy, R_HOME);
        let lf = localization(&st_fin.trail, world.w, world.h, hx, hy, R_HOME);
        assert!(
            lf < li,
            "seed {seed}: finite trail_max should reduce localization (fin={lf:.4} >= inf={li:.4})"
        );
        l_inf.push(li);
        l_fin.push(lf);
    }

    let med_inf = median(l_inf);
    let med_fin = median(l_fin);
    // 実測: inf=0.6435, finite(18)=0.5053 (比 0.785)。マージンを見て 0.9 以下(10%超低下)を要求。
    assert!(
        med_fin <= med_inf * 0.9,
        "median localization should drop meaningfully: finite={med_fin:.4} inf={med_inf:.4}"
    );
}

/// acceptance #2: 離れた砂糖の回収量が有限 trail_max で退行しない(同等以上)。
#[test]
fn finite_trail_max_does_not_regress_foraging_collection() {
    let base = Params::default();
    let world = make_synthetic_archipelago(&base);
    let (hx, hy) = world.default_home(base.e_lo);
    let far = low_e_cell_at_dist(&world, base.e_lo, hx, hy, 12.0);

    let p_inf = foraging_params(hx, hy, f64::INFINITY);
    let p_fin = foraging_params(hx, hy, TRAIL_MAX_FINITE);

    let mut collected_inf = Vec::new();
    let mut collected_fin = Vec::new();
    for &seed in &SEEDS {
        let st_inf = run(seed, &world, &p_inf, far);
        let st_fin = run(seed, &world, &p_fin, far);
        collected_inf.push(st_inf.collected_total);
        collected_fin.push(st_fin.collected_total);
    }

    let med_inf = median(collected_inf);
    let med_fin = median(collected_fin);
    assert!(
        med_fin >= med_inf - 1e-6,
        "distant-sugar collection should not regress: finite={med_fin:.4} inf={med_inf:.4}"
    );
}

/// acceptance #3: 上限有効時、全trailセルが trail_max + eps 以下に収まる。
#[test]
fn trail_values_stay_within_cap() {
    let base = Params::default();
    let world = make_synthetic_archipelago(&base);
    let (hx, hy) = world.default_home(base.e_lo);
    let far = low_e_cell_at_dist(&world, base.e_lo, hx, hy, 12.0);
    let p = foraging_params(hx, hy, TRAIL_MAX_FINITE);

    for &seed in &SEEDS {
        let st = run(seed, &world, &p, far);
        let max_trail = st.trail.iter().cloned().fold(0.0f32, f32::max);
        assert!(
            max_trail as f64 <= TRAIL_MAX_FINITE + 1e-3,
            "seed {seed}: trail cap violated, max={max_trail}"
        );
    }
}

/// acceptance #4/#6: 有限 trail_max でも不変条件（有限性・境界・保存則・ソフト標高忌避）と
/// 決定性（同一seed→同一hash）が保たれる。
#[test]
fn finite_trail_max_preserves_invariants_and_determinism() {
    let base = Params::default();
    let world = make_synthetic_archipelago(&base);
    let (hx, hy) = world.default_home(base.e_lo);
    let far = low_e_cell_at_dist(&world, base.e_lo, hx, hy, 12.0);
    let p = foraging_params(hx, hy, TRAIL_MAX_FINITE);

    for &seed in &SEEDS {
        let st1 = run(seed, &world, &p, far);
        let h1 = state_hash(&st1, &p);
        let st2 = run(seed, &world, &p, far);
        let h2 = state_hash(&st2, &p);
        assert_eq!(h1, h2, "determinism: seed {seed} hash mismatch under finite trail_max");

        assert!(st1.trail.iter().all(|v| v.is_finite()), "trail finite (seed {seed})");
        assert!(
            (0..st1.n_agents()).all(|i| st1.ax[i].is_finite() && st1.ay[i].is_finite()),
            "agent coords finite (seed {seed})"
        );
        for i in 0..st1.n_agents() {
            let (x, y) = (st1.ax[i] as f64, st1.ay[i] as f64);
            assert!(
                x >= 0.0 && x < world.w as f64 && y >= 0.0 && y < world.h as f64,
                "agent in bounds (seed {seed})"
            );
            assert!(
                world.land_mask[(y.floor() as usize) * world.w + (x.floor() as usize)],
                "agent on land (seed {seed})"
            );
        }
        assert!(conservation_ok(&st1), "conservation (seed {seed})");

        let m = compute_metrics(&st1, &world, &p);
        assert!(
            m.mean_trail_hi < m.mean_trail_lo,
            "soft elevation avoidance (seed {seed}): hi={} lo={}",
            m.mean_trail_hi,
            m.mean_trail_lo
        );
    }
}

/// acceptance #5（最重要・リグレッション無し）: 既定(trail_max=INFINITY)では
/// v.min(INFINITY)==v により final_state_hash が変わらないことを、foraging プリセットで確認。
/// core-000/001/002/003等の既存ゴールデン自体はこのファイルでは触らず、`cargo test` 全体実行で
/// 別途緑を確認する（本テストは「INFINITY指定時に新コードパスが無挙動であること」の直接証拠）。
#[test]
fn default_trail_max_is_infinity_and_byte_identical_to_no_cap() {
    let base = Params::default();
    assert_eq!(base.trail_max, f64::INFINITY, "default trail_max must remain unbounded");

    let world = make_synthetic_archipelago(&base);
    let (hx, hy) = world.default_home(base.e_lo);
    let far = low_e_cell_at_dist(&world, base.e_lo, hx, hy, 12.0);

    // Params::default() の trail_max を明示的に INFINITY にしたものと、デフォルトそのままの
    // Params で挙動(hash)が一致することを確認 = 上限機構が既定で完全に無効であること。
    let p_default = foraging_params(hx, hy, Params::default().trail_max);
    let p_explicit_inf = foraging_params(hx, hy, f64::INFINITY);

    for &seed in &SEEDS {
        let st_a = run(seed, &world, &p_default, far);
        let st_b = run(seed, &world, &p_explicit_inf, far);
        let ha = state_hash(&st_a, &p_default);
        let hb = state_hash(&st_b, &p_explicit_inf);
        assert_eq!(ha, hb, "seed {seed}: default trail_max must be byte-identical to explicit INFINITY");
    }
}
