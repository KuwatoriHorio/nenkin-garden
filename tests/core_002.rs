//! core-002 受け入れテスト（tasks/task-core-002.md）。
//!
//! ホーム凝集スタート＋誘引物質勾配コホージョンを有効化した foraging プリセットで、
//! 「群れが凝集したままホームから砂糖へ触手を伸ばして到達し（伸び）、餌のない方向の枝は
//! 退縮する（縮み）」挙動を、正準9シードの中央値/計数で検証する。
//!
//! 既定（init_cluster_sigma=0, w_trail_cohesion=0）は現行挙動をバイト維持するため、
//! 既存テスト（core-000/001, analysis-00x, harness, thresholds）は本ファイルと独立に緑を保つ
//! （＝規約 §7: 既存ゴールデンを一切弱めない。本ファイルは新規挙動の追加検証のみ）。

use nenkin_garden::hash::state_hash;
use nenkin_garden::metrics::compute_metrics;
use nenkin_garden::params::Params;
use nenkin_garden::state::{initial_state, Op, State};
use nenkin_garden::step::step;
use nenkin_garden::world::{make_synthetic_archipelago, World};

const S9: [u64; 9] = [1, 7, 13, 42, 99, 256, 1337, 2024, 31337];
const TICKS: u64 = 220; // 不変条件テスト用の実行長
const T_REACH: u64 = 160; // foraging: 砂糖ありでトンネル形成
const T_AFTER: u64 = 140; // foraging: 砂糖除去後の退縮観察
const WARMUP: u64 = 40;
const SUGAR_STRENGTH: f64 = 600.0;

/// foraging プリセット（人間定義の固定値・実測 core-002 probe で選定）。
fn foraging_params(hx: f64, hy: f64) -> Params {
    let mut p = Params::default();
    p.home_x = hx;
    p.home_y = hy;
    p.init_cluster_sigma = 3.0;
    p.w_trail_cohesion = 1.0;
    p
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// 低標高陸セルのうち home から距離 target に最も近いセル中心（決定的）。
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

fn trail_disk_sum(trail: &[f32], w: usize, h: usize, cx: f64, cy: f64, r: f64) -> f64 {
    let mut s = 0.0;
    let ri = r.ceil() as i64;
    let (ix, iy) = (cx.floor() as i64, cy.floor() as i64);
    for dy in -ri..=ri {
        for dx in -ri..=ri {
            if (dx * dx + dy * dy) as f64 > r * r {
                continue;
            }
            let (nx, ny) = (ix + dx, iy + dy);
            if nx < 0 || nx >= w as i64 || ny < 0 || ny >= h as i64 {
                continue;
            }
            s += trail[ny as usize * w + nx as usize] as f64;
        }
    }
    s
}

fn agent_dist_stats(state: &State, hx: f64, hy: f64) -> (f64, f64) {
    let mut d: Vec<f64> = (0..state.n_agents())
        .map(|i| {
            let dx = state.ax[i] as f64 - hx;
            let dy = state.ay[i] as f64 - hy;
            (dx * dx + dy * dy).sqrt()
        })
        .collect();
    d.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let med = d[d.len() / 2];
    let max = *d.last().unwrap();
    (med, max)
}

/// 凝集群がホームから砂糖A(距離D)へ触手を伸ばして到達し（伸び）、
/// 砂糖を除去すると補強を失ったトンネルが減衰する（縮み）。
#[test]
fn forage_home_cluster_reaches_and_retracts() {
    let base = Params::default();
    let world = make_synthetic_archipelago(&base);
    let (hx, hy) = world.default_home(base.e_lo);
    let p = foraging_params(hx, hy);

    let (ax, ay) = low_e_cell_at_dist(&world, base.e_lo, hx, hy, 12.0);
    let real_d = ((ax - hx).powi(2) + (ay - hy).powi(2)).sqrt();
    assert!(real_d >= 9.0, "sugar A should be a genuine distance away, got {real_d:.1}");

    let mut spread_warm = Vec::new();
    let mut max_dist_peak = Vec::new();
    let mut retention = Vec::new();
    let mut reached = 0usize;

    for &seed in &S9 {
        let mut st = initial_state(seed, &world, &p);
        // --- 伸び: 砂糖ありでトンネル形成 ---
        for t in 0..T_REACH {
            let ops: Vec<Op> = if t == 0 {
                vec![Op::PlaceSugar { x: ax, y: ay, strength: SUGAR_STRENGTH }]
            } else {
                vec![]
            };
            step(&mut st, &world, &p, &ops);
            if t + 1 == WARMUP {
                let (med, _) = agent_dist_stats(&st, hx, hy);
                spread_warm.push(med);
            }
        }
        // 到達（伸び）: A の砂糖が実際に回収された（エージェントが半径内に来た）。
        if st.sugar_remaining[0] < SUGAR_STRENGTH - 1e-9 {
            reached += 1;
        }
        let (_, max_peak) = agent_dist_stats(&st, hx, hy);
        max_dist_peak.push(max_peak);
        let peak = trail_disk_sum(&st.trail, world.w, world.h, ax, ay, 3.0);

        // --- 縮み: 砂糖除去 → 補強停止 → トンネル減衰 ---
        for t in 0..T_AFTER {
            let ops: Vec<Op> = if t == 0 { vec![Op::RemoveSugar { id: 0 }] } else { vec![] };
            step(&mut st, &world, &p, &ops);
        }
        let after = trail_disk_sum(&st.trail, world.w, world.h, ax, ay, 3.0);
        retention.push(after / (peak + 1e-9));
    }

    let spread_med = median(spread_warm);
    let maxd_med = median(max_dist_peak);
    let reten_med = median(retention);

    // ① 初期凝集: warmup 後もエージェントはホーム近傍にまとまる（cohesion=0 なら ~24 に四散する）。
    assert!(
        spread_med <= 15.0,
        "colony should stay cohesive near home; spread median = {spread_med:.1}"
    );
    // ② 到達（伸び）: 距離Dの砂糖へ複数シード中央値で到達（9本中5以上）。
    assert!(reached >= 5, "colony should forage-reach sugar A; reached {reached}/9");
    // ③ 伸展の実在: 砂糖到達時点でホームからの最大到達距離が距離Dの相当分に届く。
    assert!(
        maxd_med >= 8.0,
        "a tendril should physically extend from home; max-dist median = {maxd_med:.1}"
    );
    // ④ 退縮（縮み）: 砂糖除去後、餌方向トンネルの trail が有意に減衰する（補強を失った枝は縮む）。
    assert!(
        reten_med <= 0.6,
        "fed tube should decay after sugar removed (retraction); retention median = {reten_med:.2}"
    );
}

/// foraging プリセットでも Tier0 不変条件と決定性が保たれる。
#[test]
fn forage_preset_preserves_invariants_and_determinism() {
    let base = Params::default();
    let world = make_synthetic_archipelago(&base);
    let (hx, hy) = world.default_home(base.e_lo);
    let p = foraging_params(hx, hy);
    let (ax, ay) = low_e_cell_at_dist(&world, base.e_lo, hx, hy, 12.0);

    for &seed in &S9 {
        // 2回走らせてハッシュ一致（決定性）。
        let run = |()| -> (State, u64) {
            let mut st = initial_state(seed, &world, &p);
            for t in 0..TICKS {
                let ops: Vec<Op> = if t == 0 {
                    vec![Op::PlaceSugar { x: ax, y: ay, strength: SUGAR_STRENGTH }]
                } else {
                    vec![]
                };
                step(&mut st, &world, &p, &ops);
            }
            let hsh = state_hash(&st, &p);
            (st, hsh)
        };
        let (st, h1) = run(());
        let (_, h2) = run(());
        assert_eq!(h1, h2, "determinism: seed {seed} hash mismatch");

        // 有限性: trail・座標に NaN/Inf が無い。
        assert!(st.trail.iter().all(|v| v.is_finite()), "trail finite (seed {seed})");
        assert!(
            (0..st.n_agents()).all(|i| st.ax[i].is_finite() && st.ay[i].is_finite()),
            "agent coords finite (seed {seed})"
        );
        // 境界: 全エージェントが範囲内・陸上。
        for i in 0..st.n_agents() {
            let (x, y) = (st.ax[i] as f64, st.ay[i] as f64);
            assert!(
                x >= 0.0 && x < world.w as f64 && y >= 0.0 && y < world.h as f64,
                "agent in bounds (seed {seed})"
            );
            assert!(
                world.land_mask[(y.floor() as usize) * world.w + (x.floor() as usize)],
                "agent on land (seed {seed})"
            );
        }
        // 保存則: biomass == collected - consumed, 非負。
        let expect = st.collected_total - st.consumed_total;
        assert!(
            (st.biomass - expect).abs() <= 1e-4 && st.biomass >= -1e-9,
            "conservation (seed {seed}): biomass={} expect={}",
            st.biomass,
            expect
        );
        // ソフト標高忌避: 網は低標高に偏る（mean_trail_hi < mean_trail_lo）。
        let m = compute_metrics(&st, &world, &p);
        assert!(
            m.mean_trail_hi < m.mean_trail_lo,
            "soft elevation avoidance (seed {seed}): hi={} lo={}",
            m.mean_trail_hi,
            m.mean_trail_lo
        );
    }
}
