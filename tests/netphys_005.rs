//! netphys-005: 誘引の標高依存化（高標高の砂糖は通常予算で「見捨てる」・ソフト忌避を強める）
//! 受け入れテスト。Jones でも tree でもない第3の独立モデル。netphys-001/002/003 の緑は
//! それぞれの既存ファイルが引き続き検証する（本ファイルは無変更・別ファイル追加）。
//!
//! 背景: render-net-003 の実測で、砂糖が誘引半径(attract_radius=40)内にあると誘引方向が
//! 支配的になり w_elev の方向バイアスだけでは登坂を止められないことが判明。本タスクは
//! 誘引そのものを標高依存にし（`NetParams::attract_e_hi`/`attract_e_falloff`）、高標高
//! (e>=attract_e_hi)の砂糖への引力を強く減衰させる。ただし壁ではない（ソフト・§0）。
//!
//! S9（正準9シード）の部分集合 [1, 42, 1337] の中央値で判定する（規約 §4 の集計法を踏襲）。

use nenkin_garden::netphys::{
    initial_net_state, netphys_kirchhoff_solve, netphys_step, run_netphys_headless, NetParams,
    NetState,
};
use nenkin_garden::params::Params;
use nenkin_garden::state::{Op, ScriptEntry};
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

/// home からほぼ距離 d にある陸セルのうち最も標高が高い/低いものを探す
/// （netphys_001 accept5・netphys_003 find_high_low_targets と同趣旨）。
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

fn e_at(world: &World, x: f64, y: f64) -> f64 {
    let cix = (x.floor() as usize).min(world.w - 1);
    let ciy = (y.floor() as usize).min(world.h - 1);
    world.e[ciy * world.w + cix] as f64
}

fn nearest_node(s: &NetState, x: f64, y: f64) -> Option<(usize, f64)> {
    let mut best: Option<(usize, f64)> = None;
    for (i, nd) in s.nodes.iter().enumerate() {
        let d = ((nd.x - x).powi(2) + (nd.y - y).powi(2)).sqrt();
        if best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((i, d));
        }
    }
    best
}

/// home と目標座標へ砂糖(strength=2000)を置き ticks 回した末、home 近傍ノードと目標近傍
/// (半径5)ノードが `flow_connected` かどうかを3シード返す（0/1 flags）。
fn measure_connected_flags(
    world: &World,
    hx: f64,
    hy: f64,
    target_xy: (f64, f64),
    np: &NetParams,
    ticks: u64,
) -> Vec<f64> {
    let mut flags = Vec::new();
    for &seed in &SEEDS {
        let script = vec![ScriptEntry {
            tick: 0,
            op: Op::PlaceSugar { x: target_xy.0, y: target_xy.1, strength: 2000.0 },
        }];
        let r = run_netphys_headless(seed, &script, ticks, np, world);
        let s = &r.final_state;
        let n_home = nearest_node(s, hx, hy);
        let n_target = nearest_node(s, target_xy.0, target_xy.1);
        let connected = match (n_home, n_target) {
            (Some((i1, _)), Some((i2, d2))) if i1 != i2 && d2 <= 5.0 => {
                let flow = netphys_kirchhoff_solve(&s.nodes, &s.edges, world, np.net_alpha, i1, i2);
                flow.connected
            }
            _ => false,
        };
        flags.push(if connected { 1.0 } else { 0.0 });
    }
    flags
}

// ---------- ① 高標高の砂糖を見捨てる（主判定） ----------
//
// home からほぼ同距離(d=18)にある最高標高セル(実測 e≈0.979、既定 attract_e_hi=0.9 以上)へ
// 砂糖を置き、通常予算(NetParams::default, initial_budget=1200)で720tick(=60 consolidation
// サイクル)走らせても、3シード中央値で home と連結しない（見捨てる）ことを確認する。
//
// §7 exemplary: 同一 world/seed で `attract_e_falloff=0`（誘引の標高依存減衰なし＝現状相当）
// にすると、同じ高標高目標へも連結してしまう(赤)ことを対比で確認したうえで、既定
// `attract_e_falloff`（効く値）で見捨てる(緑)ことを見る。
#[test]
fn accept1_high_elevation_sugar_abandoned() {
    let (world, hx, hy) = world_and_home();
    let np = NetParams::default();
    let (high_xy, _) = find_high_low_targets(&world, hx, hy, 18.0, 2.0);
    assert!(
        e_at(&world, high_xy.0, high_xy.1) >= np.attract_e_hi,
        "高標高ターゲットの実測標高が attract_e_hi 未満（world/しきい前提が崩れている）"
    );

    let ticks = 720u64;
    let flags = measure_connected_flags(&world, hx, hy, high_xy, &np, ticks);
    let m = median3(flags.clone());
    assert!(
        m < 1.0,
        "既定パラメータ(誘引の標高依存減衰あり)でも高標高の砂糖が中央値で連結してしまっている \
         (見捨てられていない): flags={:?}",
        flags
    );
}

/// §7 exemplary: 誘引の標高依存減衰を無効化(attract_e_falloff=0)すると、同一 world/seed・
/// 同一高標高目標へは（現状相当の挙動として）中央値で連結してしまう（赤）ことを確認する。
#[test]
fn accept1_baseline_falloff_disabled_connects_red() {
    let (world, hx, hy) = world_and_home();
    let mut np0 = NetParams::default();
    np0.attract_e_falloff = 0.0;
    let (high_xy, _) = find_high_low_targets(&world, hx, hy, 18.0, 2.0);

    let ticks = 720u64;
    let flags = measure_connected_flags(&world, hx, hy, high_xy, &np0, ticks);
    let m = median3(flags.clone());
    assert!(
        m >= 1.0,
        "attract_e_falloff=0(誘引の標高依存減衰なし)でも高標高の砂糖が中央値で連結しなくなって \
         しまっている(期待は連結=赤): flags={:?}",
        flags
    );
}

// ---------- ② 低〜中標高の砂糖は連結する（回帰の明示） ----------
//
// netphys-001 accept2 と同じ2砂糖座標（低〜中標高: 実測 e≈0.712, e≈0.012、いずれも
// attract_e_hi=0.9 未満）を使い、通常予算で従来どおり連結することを確認する
// （netphys-001 ①②自体は tests/netphys_001.rs が引き続き緑を担保）。
#[test]
fn accept2_low_mid_elevation_sugar_still_connected() {
    let (world, hx, hy) = world_and_home();
    let np = NetParams::default();

    // netphys_001.rs の nearest_land と同じ座標決定（低〜中標高）。
    fn nearest_land(world: &World, x: f64, y: f64) -> (f64, f64) {
        let fx = x.floor();
        let fy = y.floor();
        let inb = fx >= 0.0 && fx < world.w as f64 && fy >= 0.0 && fy < world.h as f64;
        if inb && world.land_mask[(fy as usize) * world.w + (fx as usize)] {
            return (x, y);
        }
        let mut best: Option<(f64, usize)> = None;
        for cy in 0..world.h {
            for cx in 0..world.w {
                let i = cy * world.w + cx;
                if !world.land_mask[i] {
                    continue;
                }
                let cxf = cx as f64 + 0.5;
                let cyf = cy as f64 + 0.5;
                let d = (cxf - x).powi(2) + (cyf - y).powi(2);
                if best.map(|(bd, _)| d < bd).unwrap_or(true) {
                    best = Some((d, i));
                }
            }
        }
        let i = best.expect("world has no land cells").1;
        ((i % world.w) as f64 + 0.5, (i / world.w) as f64 + 0.5)
    }

    let mid_xy = nearest_land(&world, hx + 12.0, hy);
    assert!(
        e_at(&world, mid_xy.0, mid_xy.1) < np.attract_e_hi,
        "低〜中標高ターゲットの実測標高が attract_e_hi 以上（world/しきい前提が崩れている）"
    );

    let ticks = 400u64;
    let flags = measure_connected_flags(&world, hx, hy, mid_xy, &np, ticks);
    let m = median3(flags.clone());
    assert!(
        m >= 1.0,
        "低〜中標高の砂糖が中央値で連結しない(回帰): flags={:?}",
        flags
    );
}

// ---------- ③ ソフト（壁でない） ----------
//
// 高標高セルへの伸長が物理的に禁止されていないこと。誘引に頼らない純粋な探索
// （砂糖なし・w_rand + 既存の w_elev 方向バイアス）でも、十分な tick を与えれば
// 高標高帯(e>=attract_e_hi)へ「一度でも」ノードが到達しうることを弱く確認する
// （tests/netphys_003.rs の改定版 accept2 と同趣旨・同じ判定方式）。
#[test]
fn accept3_soft_not_wall_high_elevation_reachable() {
    let (world, _hx, _hy) = world_and_home();
    let np = NetParams::default();

    let mut reached_flags = Vec::new();
    for &seed in &SEEDS {
        let mut s = initial_net_state(seed, &world, &np);
        let mut ever_reached = false;
        for _ in 0..3000u64 {
            netphys_step(&mut s, &world, &np, &[]); // 砂糖なし: 誘引に頼らない純粋な探索
            if !ever_reached {
                ever_reached = s.nodes.iter().any(|nd| e_at(&world, nd.x, nd.y) >= np.attract_e_hi);
            }
        }
        reached_flags.push(if ever_reached { 1.0 } else { 0.0 });
    }
    let m = median3(reached_flags.clone());
    assert!(
        m >= 1.0,
        "標高忌避が壁化しており、砂糖誘引なしの十分なサイクルでも高標高帯(e>=attract_e_hi)へ \
         到達できない(ソフト性違反の疑い): flags={:?}",
        reached_flags
    );
}
