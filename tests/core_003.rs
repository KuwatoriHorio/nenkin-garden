//! core-003 受け入れテスト（tasks/task-core-003.md）。
//!
//! 「残量0の砂糖を tick 末尾で決定論的に自動削除する」を検証する。
//! - 枯渇（sugar_remaining<=0）した砂糖源は sugar_id/x/y/strength/remaining から消える。
//! - 枯渇前（remaining>0）の砂糖源はリストに残る。
//! - 削除は保存則（biomass == collected_total - consumed_total, 非負）を壊さない。
//! - 同一tickに複数枯渇しても決定的に処理される。
//! - 現実的な foraging シナリオでも不変条件・決定性（同一seed→同一hash）を保つ。
//!
//! 既存テスト（core-000/001/002, analysis-00x, harness, thresholds）とは独立に緑を保つ
//! （§7: 既存ゴールデンを一切弱めない。本ファイルは新規挙動の追加検証のみ）。

use nenkin_garden::hash::state_hash;
use nenkin_garden::metrics::compute_metrics;
use nenkin_garden::params::Params;
use nenkin_garden::state::{initial_state, Op, State};
use nenkin_garden::step::step;
use nenkin_garden::world::{make_synthetic_archipelago, World};

const SEEDS: [u64; 3] = [1, 42, 1337];

/// 低標高陸セルのうち home から距離 target に最も近いセル中心（決定的）。
/// core_002 の同名ヘルパと同じ選定法: 実測で「群れが伸びて到達しトンネルを作る」ことが
/// 確認済みの距離・強度なので、本ファイルの持続砂糖（elev-avoidance 検証用）にも流用する。
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

/// 与座標に最も近い陸セルの中心を返す（決定的・全探索）。
fn nearest_land(world: &World, cy0: f64, cx0: f64) -> (f64, f64) {
    let (h, w) = (world.h, world.w);
    let mut best: Option<(usize, usize)> = None;
    let mut best_d = f64::INFINITY;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if !world.land_mask[i] {
                continue;
            }
            let d = (x as f64 - cx0).powi(2) + (y as f64 - cy0).powi(2);
            if d < best_d {
                best_d = d;
                best = Some((x, y));
            }
        }
    }
    let (bx, by) = best.expect("world has no land cells");
    (bx as f64 + 0.5, by as f64 + 0.5)
}

fn conservation_ok(st: &State) -> bool {
    let expect = st.collected_total - st.consumed_total;
    (st.biomass - expect).abs() <= 1e-6 && st.biomass >= -1e-9
}

fn arrays_consistent(st: &State) -> bool {
    let n = st.sugar_id.len();
    st.sugar_x.len() == n
        && st.sugar_y.len() == n
        && st.sugar_strength.len() == n
        && st.sugar_remaining.len() == n
}

/// 1tick で完全回収（remaining<=0）になった砂糖は、その tick 末尾で
/// sugar_id/x/y/strength/remaining から消える。離れた大 strength の砂糖は残る。
#[test]
fn depleted_sugar_removed_deterministically() {
    let base = Params::default();
    let world = make_synthetic_archipelago(&base);
    let mut p = base;
    p.collect_rate = 10.0; // strength=2.0 を1tickで完全回収させる

    let a = nearest_land(&world, 0.30 * world.h as f64, 0.28 * world.w as f64);
    let c = nearest_land(&world, 0.62 * world.h as f64, 0.40 * world.w as f64);
    assert!(
        (a.0 - c.0).powi(2) + (a.1 - c.1).powi(2) > (p.sugar_radius * 4.0).powi(2),
        "test cells A/C must be far apart relative to sugar_radius"
    );

    let mut st = initial_state(1, &world, &p);
    // 単一エージェントを A に直置き（探索の確率性を排除し、回収を確定させる）。
    st.ax = vec![a.0 as f32];
    st.ay = vec![a.1 as f32];
    st.ah = vec![0.0];

    let ops = vec![
        Op::PlaceSugar { x: a.0, y: a.1, strength: 2.0 }, // id0: 1tickで枯渇
        Op::PlaceSugar { x: c.0, y: c.1, strength: 600.0 }, // id1: 離れていて無傷
    ];
    step(&mut st, &world, &p, &ops);

    assert_eq!(st.sugar_id, vec![1], "depleted id0 should be auto-removed, id1 should remain");
    assert_eq!(st.sugar_remaining, vec![600.0], "surviving sugar remaining untouched");
    assert!(arrays_consistent(&st), "parallel sugar arrays must stay same length");
    assert!(
        (st.collected_total - (p.initial_biomass + 2.0)).abs() <= 1e-9,
        "collected_total should reflect exactly the one collection event: {}",
        st.collected_total
    );
    assert!(conservation_ok(&st), "conservation must hold after auto-removal");
}

/// remaining>0 のうちは砂糖源はリストに残る（枯渇前は消えない）。
#[test]
fn sugar_above_zero_remains_in_list() {
    let base = Params::default();
    let world = make_synthetic_archipelago(&base);
    let p = base; // collect_rate=0.5 (既定): 1tickでは strength=2.0 を使い切らない

    let a = nearest_land(&world, 0.30 * world.h as f64, 0.28 * world.w as f64);
    let c = nearest_land(&world, 0.62 * world.h as f64, 0.40 * world.w as f64);

    let mut st = initial_state(1, &world, &p);
    st.ax = vec![a.0 as f32];
    st.ay = vec![a.1 as f32];
    st.ah = vec![0.0];

    let ops = vec![
        Op::PlaceSugar { x: a.0, y: a.1, strength: 2.0 },
        Op::PlaceSugar { x: c.0, y: c.1, strength: 600.0 },
    ];
    step(&mut st, &world, &p, &ops);

    assert_eq!(st.sugar_id, vec![0, 1], "both sugars should still be present (neither depleted)");
    assert!(st.sugar_remaining[0] > 0.0 && st.sugar_remaining[0] < 2.0, "id0 partially collected");
    assert!((st.sugar_remaining[1] - 600.0).abs() < 1e-9, "id1 untouched");
    assert!(arrays_consistent(&st));
    assert!(conservation_ok(&st));
}

/// 同一tickに複数の砂糖が同時に枯渇しても、決定的に両方削除され、無関係な砂糖は残る。
#[test]
fn multiple_simultaneous_depletions_removed_together() {
    let base = Params::default();
    let world = make_synthetic_archipelago(&base);
    let mut p = base;
    p.collect_rate = 10.0;

    let a = nearest_land(&world, 0.30 * world.h as f64, 0.28 * world.w as f64);
    let b = nearest_land(&world, 0.75 * world.h as f64, 0.72 * world.w as f64);
    let c = nearest_land(&world, 0.62 * world.h as f64, 0.40 * world.w as f64);
    // C は A・B いずれからも sugar_radius を大きく超えて離れていること。
    let r2min = (p.sugar_radius * 4.0).powi(2);
    assert!((a.0 - c.0).powi(2) + (a.1 - c.1).powi(2) > r2min);
    assert!((b.0 - c.0).powi(2) + (b.1 - c.1).powi(2) > r2min);

    let mut st = initial_state(1, &world, &p);
    st.ax = vec![a.0 as f32, b.0 as f32];
    st.ay = vec![a.1 as f32, b.1 as f32];
    st.ah = vec![0.0, 0.0];

    let ops = vec![
        Op::PlaceSugar { x: a.0, y: a.1, strength: 2.0 },   // id0: 枯渇
        Op::PlaceSugar { x: b.0, y: b.1, strength: 3.0 },   // id1: 枯渇
        Op::PlaceSugar { x: c.0, y: c.1, strength: 600.0 }, // id2: 無傷
    ];
    step(&mut st, &world, &p, &ops);

    assert_eq!(st.sugar_id, vec![2], "both depleted sugars removed; id2 remains");
    assert_eq!(st.sugar_remaining, vec![600.0]);
    assert!(arrays_consistent(&st));
    assert!(conservation_ok(&st));
}

/// 現実的な foraging シナリオ（ホーム凝集＋trailコホージョン）で、
/// 小strengthの砂糖が回収され枯渇・自動削除される一方、離れた大strengthの砂糖は残り、
/// 不変条件（有限性・境界・保存則・ソフト標高忌避）と決定性（同一seed→同一hash）が保たれる。
#[test]
fn foraging_regression_depletion_invariants_and_determinism() {
    let base = Params::default();
    let world = make_synthetic_archipelago(&base);
    let (hx, hy) = world.default_home(base.e_lo);
    let mut p = base;
    p.home_x = hx;
    p.home_y = hy;
    p.init_cluster_sigma = 3.0;
    p.w_trail_cohesion = 1.0;

    // core_002 で実測済みの「距離12・強度600」設定を流用（持続的なトンネル形成が確認済み）。
    // これで elev-avoidance の検証が信頼できる信号を持つ。ホームの極小砂糖(strength=2)は
    // 別途枯渇・自動削除の検証専用（このsugarは core_002 には存在しない、core-003固有）。
    let far = low_e_cell_at_dist(&world, base.e_lo, hx, hy, 12.0);
    const TICKS: u64 = 220; // core_002 と同じ長さ（トンネル形成に十分な長さ、実測済み）

    for &seed in &SEEDS {
        let run = || {
            let mut st = initial_state(seed, &world, &p);
            let mut removed_small = false;
            for t in 0..TICKS {
                let ops: Vec<Op> = if t == 0 {
                    vec![
                        Op::PlaceSugar { x: hx, y: hy, strength: 2.0 }, // id0: ホームの極小砂糖
                        Op::PlaceSugar { x: far.0, y: far.1, strength: 600.0 }, // id1: 離れた大砂糖
                    ]
                } else {
                    vec![]
                };
                step(&mut st, &world, &p, &ops);
                assert!(arrays_consistent(&st), "seed {seed} tick {t}: array length mismatch");
                if !st.sugar_id.contains(&0) {
                    removed_small = true;
                }
            }
            (st, removed_small)
        };

        let (st1, removed) = run();
        assert!(removed, "seed {seed}: home sugar should deplete and be auto-removed within {TICKS} ticks");
        assert!(st1.sugar_id.contains(&1), "seed {seed}: distant large sugar should survive");
        assert!(conservation_ok(&st1), "seed {seed}: conservation must hold");

        assert!(st1.trail.iter().all(|v| v.is_finite()), "seed {seed}: trail finite");
        for i in 0..st1.n_agents() {
            let (x, y) = (st1.ax[i] as f64, st1.ay[i] as f64);
            assert!(x.is_finite() && y.is_finite(), "seed {seed}: agent coords finite");
            assert!(
                x >= 0.0 && x < world.w as f64 && y >= 0.0 && y < world.h as f64,
                "seed {seed}: agent in bounds"
            );
            assert!(
                world.land_mask[(y.floor() as usize) * world.w + (x.floor() as usize)],
                "seed {seed}: agent on land"
            );
        }
        let m = compute_metrics(&st1, &world, &p);
        assert!(
            m.mean_trail_hi < m.mean_trail_lo,
            "seed {seed}: soft elevation avoidance must hold (hi={} lo={})",
            m.mean_trail_hi,
            m.mean_trail_lo
        );

        // 決定性: 同一 (seed, script, ticks) → 同一 final_state_hash。
        let h1 = state_hash(&st1, &p);
        let (st2, _) = run();
        let h2 = state_hash(&st2, &p);
        assert_eq!(h1, h2, "seed {seed}: determinism (final_state_hash) must hold");
    }
}
