//! netphys-003: 網 Physarum の標高忌避を「方向バイアス」で効かせる受け入れテスト。
//! Jones でも tree でもない第3の独立モデル。netphys-001/002（Stage1/2）の緑は
//! `tests/netphys_001.rs`・`tests/netphys_002.rs` が引き続き検証する（本ファイルは無変更・別ファイル追加）。
//!
//! 主判定: `NetParams::default()`（通常予算 initial_budget=1200）でも、高標高側の目標へ向かう
//! 構造成長が低標高側の目標へ向かう構造成長より有意に抑制される（＝低地選好が方向バイアスにより
//! 通常予算でも観測できる）。§7 exemplary: 同一 world/seed で `w_elev=0`（現状・方向バイアス無し）
//! では抑制が成立しない（赤）ことを確認したうえで、既定 `w_elev`（効く値）で成立（緑）することを見る。
//!
//! S9（正準9シード）の部分集合 [1, 42, 1337] の中央値で判定する（規約 §4 の集計法を踏襲）。

use nenkin_garden::netphys::{initial_net_state, netphys_step, total_mass, NetParams};
use nenkin_garden::params::Params;
use nenkin_garden::state::Op;
use nenkin_garden::world::{make_synthetic_archipelago, World};

const SEEDS: [u64; 3] = [1, 42, 1337];

fn median3(mut xs: Vec<f64>) -> f64 {
    assert_eq!(xs.len(), 3);
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs[1]
}

fn world_and_home() -> (World, f64, f64) {
    let wp = Params::default();
    let world = make_synthetic_archipelago(&wp);
    let np = NetParams::default();
    let (hx, hy) = world.default_home(np.e_lo);
    (world, hx, hy)
}

/// home からほぼ同距離(D=14)にある陸セルのうち最も標高が高い/低いものを探す
/// （netphys_001 accept5・find_high_low_targets と同趣旨）。
fn find_high_low_targets(world: &World, hx: f64, hy: f64, d: f64, tol: f64) -> ((f64, f64), (f64, f64)) {
    let mut best_high: Option<(f64, usize)> = None;
    let mut best_low: Option<(f64, usize)> = None;
    for cy in 0..world.h {
        for cx in 0..world.w {
            let i = cy * world.w + cx;
            if !world.land_mask[i] {
                continue;
            }
            let cxf = cx as f64 + 0.5;
            let cyf = cy as f64 + 0.5;
            let dist = ((cxf - hx).powi(2) + (cyf - hy).powi(2)).sqrt();
            if (dist - d).abs() > tol {
                continue;
            }
            let e = world.e[i] as f64;
            if best_high.map(|(be, _)| e > be).unwrap_or(true) {
                best_high = Some((e, i));
            }
            if best_low.map(|(be, _)| e < be).unwrap_or(true) {
                best_low = Some((e, i));
            }
        }
    }
    let (_, hi) = best_high.expect("高標高候補が見つからない（world固定なので通常発生しない）");
    let (_, li) = best_low.expect("低標高候補が見つからない（world固定なので通常発生しない）");
    (
        ((hi % world.w) as f64 + 0.5, (hi / world.w) as f64 + 0.5),
        ((li % world.w) as f64 + 0.5, (li / world.w) as f64 + 0.5),
    )
}

/// 高標高目標 / 低標高目標へそれぞれ砂糖を1つ置き、通常予算で K サイクル走らせた末の
/// 構造質量（total_mass - free_budget = Σ D*L）の (高側, 低側) 中央値ペアを返す。
fn measure_high_low_struct(world: &World, hx: f64, hy: f64, np: &NetParams) -> (Vec<f64>, Vec<f64>) {
    let (high_xy, low_xy) = find_high_low_targets(world, hx, hy, 14.0, 2.0);
    let ticks = 720u64; // = 60 consolidation サイクル分（period_n=12）。潤沢予算で目標到達が
                         // 十分落ち着く尺度（探索用一時テストで確認・削除済み）。

    let mut high_struct = Vec::new();
    let mut low_struct = Vec::new();
    for &seed in &SEEDS {
        let mut sh = initial_net_state(seed, world, np);
        for t in 0..ticks {
            let ops = if t == 0 {
                vec![Op::PlaceSugar { x: high_xy.0, y: high_xy.1, strength: 2000.0 }]
            } else {
                Vec::new()
            };
            netphys_step(&mut sh, world, np, &ops);
        }
        high_struct.push(total_mass(&sh) - sh.free_budget);

        let mut sl = initial_net_state(seed, world, np);
        for t in 0..ticks {
            let ops = if t == 0 {
                vec![Op::PlaceSugar { x: low_xy.0, y: low_xy.1, strength: 2000.0 }]
            } else {
                Vec::new()
            };
            netphys_step(&mut sl, world, np, &ops);
        }
        low_struct.push(total_mass(&sl) - sl.free_budget);
    }
    (high_struct, low_struct)
}

// ---------- 主判定: 通常予算でも低地選好（方向バイアス）が効く ----------

#[test]
fn accept1_normal_budget_low_elevation_preference() {
    let (world, hx, hy) = world_and_home();
    let np = NetParams::default();
    assert!(np.w_elev > 0.0, "既定 w_elev は方向バイアスが効く正値でなければならない");

    let (high_struct, low_struct) = measure_high_low_struct(&world, hx, hy, &np);
    let mh = median3(high_struct.clone());
    let ml = median3(low_struct.clone());
    assert!(
        mh < ml * 0.95,
        "通常予算(initial_budget={})でのソフト標高忌避(方向バイアス)が確認できない: \
         high_median={mh} low_median={ml} (high={:?} low={:?})",
        np.initial_budget,
        high_struct,
        low_struct
    );
}

/// §7 exemplary: 同一 world/seed で w_elev=0（方向バイアス無し＝現状）にすると、
/// 通常予算では上と同じマージンが成立しない（赤）ことを確認する。
#[test]
fn accept1_baseline_w_elev_zero_fails_margin_red() {
    let (world, hx, hy) = world_and_home();
    let mut np0 = NetParams::default();
    np0.w_elev = 0.0;

    let (high_struct, low_struct) = measure_high_low_struct(&world, hx, hy, &np0);
    let mh = median3(high_struct.clone());
    let ml = median3(low_struct.clone());
    assert!(
        !(mh < ml * 0.95),
        "w_elev=0(現状の方向バイアス無し)でも通常予算で低地選好マージンが成立してしまっている \
         (期待は不成立=赤): high_median={mh} low_median={ml} (high={:?} low={:?})",
        high_struct,
        low_struct
    );
}

// ---------- ソフト性: 高標高セルにもノードを持ちうる（完全排除でない） ----------

#[test]
fn accept2_soft_not_wall_high_elevation_reachable() {
    let (world, hx, hy) = world_and_home();
    let np = NetParams::default();
    let (high_xy, _low_xy) = find_high_low_targets(&world, hx, hy, 14.0, 2.0);

    let mut reached_flags = Vec::new();
    for &seed in &SEEDS {
        let mut s = initial_net_state(seed, &world, &np);
        for t in 0..720u64 {
            let ops = if t == 0 {
                vec![Op::PlaceSugar { x: high_xy.0, y: high_xy.1, strength: 2000.0 }]
            } else {
                Vec::new()
            };
            netphys_step(&mut s, &world, &np, &ops);
        }
        // 高標高目標近傍(半径5)にノードを持てたか＝壁で完全排除されていないか。
        let reached = s
            .nodes
            .iter()
            .any(|nd| ((nd.x - high_xy.0).powi(2) + (nd.y - high_xy.1).powi(2)).sqrt() <= 5.0);
        reached_flags.push(if reached { 1.0 } else { 0.0 });
    }
    let m = median3(reached_flags.clone());
    assert!(
        m >= 1.0,
        "標高忌避が壁化しており、十分なサイクル・砂糖誘引があっても高標高セル付近へ到達できない \
         (ソフト性違反の疑い): flags={:?}",
        reached_flags
    );
}
