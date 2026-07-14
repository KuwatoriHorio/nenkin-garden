//! netphys-006: forage（餌探索・標高忌避）に「蜘蛛の巣バイアス」（放射スポーク＋同心リング）を
//! 加算的に重ねる受け入れテスト（B案）。Jones でも tree でもない第3の独立モデル。
//! netphys-001〜005 の緑はそれぞれの既存ファイルが引き続き検証する（本ファイルは無変更・別ファイル追加）。
//!
//! 新パラメータ `w_radial`（放射スポーク係数）・`ring_period`/`ring_reach`（同心リング形成）は
//! `NetParams::default()` で既定オフ（0）。蜘蛛の巣は本ファイルが定義する「web 構成」
//! （`w_radial=2.0, ring_period=6, ring_reach=10.0` を default に追加設定したもの）でのみ発現させ、
//! (1) 放射整列 (2) 同心リング (3) forage（餌探索・標高忌避）併存 を実証する。
//!
//! S9（正準9シード）の部分集合 [1, 42, 1337] の中央値で判定する（規約 §4 の集計法を踏襲）。
//!
//! web 構成の実測根拠（探索用一時バイナリ `src/bin/explore_web.rs`・削除予定）:
//!   - w_radial を 0.5/1.0/1.5/2.0/3.0 で走査、alignment(|cos|平均) が単調に上がることを確認。
//!   - w_radial=2.0 固定で ring_period×ring_reach を走査し、ring_period=6・ring_reach=10 で
//!     ring_off(ring_period=0)比 ring_edges 中央値 2.0→11.0（5.5倍）、redundancy(冗長度)は
//!     22→40（>1でループを持つ）と有意に増えることを確認。
//!   - 同構成で netphys-005 accept1/accept2 相当のシナリオ（home からの距離18の最高標高砂糖・
//!     home+12の中標高砂糖）を実行し、高標高は3シード中央値で見捨てられ(flags=[0,0,0])、
//!     中標高は3シード中央値で連結する(flags=[1,1,1])ことを確認（forage 併存＝B案の核）。

use nenkin_garden::netphys::{
    initial_net_state, netphys_kirchhoff_solve, netphys_step, run_netphys_headless, NetParams,
    NetState,
};
use nenkin_garden::params::Params;
use nenkin_garden::state::{Op, ScriptEntry};
use nenkin_garden::world::{make_synthetic_archipelago, World};

const SEEDS: [u64; 3] = [1, 42, 1337];
const TICKS: u64 = 400;

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

/// 蜘蛛の巣（web）構成: forage/標高忌避は維持したまま放射スポーク＋同心リングを有効化する。
/// 実測根拠は本ファイル冒頭コメント参照。
fn web_params() -> NetParams {
    let mut p = NetParams::default();
    p.w_radial = 2.0;
    p.ring_period = 6;
    p.ring_reach = 10.0;
    p
}

// ---------- 共通ヘルパ ----------

/// 各新規辺 (a,b) の方向と a の r_hat（ホームから外向きの単位ベクトル）の |cos| 平均。
/// 半径方向に整列しているほど 1 に近づく（乱数不使用・決定的な幾何指標）。
fn radial_alignment(s: &NetState, hx: f64, hy: f64) -> f64 {
    let mut sum = 0.0;
    let mut n = 0.0;
    for e in &s.edges {
        let (ax, ay) = (s.nodes[e.a].x, s.nodes[e.a].y);
        let (bx, by) = (s.nodes[e.b].x, s.nodes[e.b].y);
        let (rvx, rvy) = (ax - hx, ay - hy);
        let rvn = (rvx * rvx + rvy * rvy).sqrt();
        if rvn <= 1.0e-6 {
            continue; // ホーム自身が端点の辺は方位不定のため除外
        }
        let (rhx, rhy) = (rvx / rvn, rvy / rvn);
        let (dx, dy) = (bx - ax, by - ay);
        let dn = (dx * dx + dy * dy).sqrt();
        if dn <= 1.0e-6 {
            continue;
        }
        let cos = ((dx / dn) * rhx + (dy / dn) * rhy).abs();
        sum += cos;
        n += 1.0;
    }
    if n <= 0.0 {
        0.0
    } else {
        sum / n
    }
}

/// リング辺の数: 両端の半径差が小さく(<3.0・同心帯)、かつ角度差が一定以上(>0.3rad・別スポーク間)
/// を繋ぐ辺の本数。
fn ring_edge_count(s: &NetState, hx: f64, hy: f64) -> usize {
    let mut cnt = 0;
    for e in &s.edges {
        let (ax, ay) = (s.nodes[e.a].x, s.nodes[e.a].y);
        let (bx, by) = (s.nodes[e.b].x, s.nodes[e.b].y);
        let ra = ((ax - hx).powi(2) + (ay - hy).powi(2)).sqrt();
        let rb = ((bx - hx).powi(2) + (by - hy).powi(2)).sqrt();
        if ra <= 1.0e-6 || rb <= 1.0e-6 {
            continue;
        }
        let aa = (ay - hy).atan2(ax - hx);
        let ab = (by - hy).atan2(bx - hx);
        let mut dang = (aa - ab).abs();
        if dang > std::f64::consts::PI {
            dang = 2.0 * std::f64::consts::PI - dang;
        }
        if (ra - rb).abs() < 3.0 && dang > 0.3 {
            cnt += 1;
        }
    }
    cnt
}

/// 冗長度（目安）: edges - (nodes - 1)。>1 ならループ（ツリーでない）を持つ。
fn redundancy(s: &NetState) -> i64 {
    s.n_edges() as i64 - (s.n_nodes() as i64 - 1)
}

/// 砂糖なしで TICKS 進めた末の (alignment, ring_edge_count) を 3 シード分測る。
fn measure_geometry(world: &World, np: &NetParams) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let (hx, hy) = world.default_home(np.e_lo);
    let mut aligns = Vec::new();
    let mut rings = Vec::new();
    let mut reds = Vec::new();
    for &seed in &SEEDS {
        let mut s = initial_net_state(seed, world, np);
        for _ in 0..TICKS {
            netphys_step(&mut s, world, np, &[]);
        }
        aligns.push(radial_alignment(&s, hx, hy));
        rings.push(ring_edge_count(&s, hx, hy) as f64);
        reds.push(redundancy(&s) as f64);
    }
    (aligns, rings, reds)
}

// ---------- ① 放射スポーク（web on で半径方向に整列） ----------

#[test]
fn accept1_radial_spokes_align_with_home_direction() {
    let (world, _hx, _hy) = world_and_home();
    let web = web_params();
    let mut web_off = web;
    web_off.w_radial = 0.0; // 放射バイアスのみ無効化（リングは on のまま・分離比較）

    let (align_web, _, _) = measure_geometry(&world, &web);
    let (align_off, _, _) = measure_geometry(&world, &web_off);

    let m_web = median3(align_web.clone());
    let m_off = median3(align_off.clone());
    const MARGIN: f64 = 1.15;
    assert!(
        m_web > m_off * MARGIN,
        "web構成(w_radial>0)で放射整列度が w_radial=0 より有意に高くなっていない: \
         web_median={m_web} off_median={m_off} (web={:?} off={:?})",
        align_web,
        align_off
    );
}

/// §7 exemplary: w_radial=0 同士（差なし）では当然マージンが成立しない（赤）ことを確認する。
#[test]
fn accept1_baseline_w_radial_zero_fails_margin_red() {
    let (world, _hx, _hy) = world_and_home();
    let mut web_off = web_params();
    web_off.w_radial = 0.0;

    let (align_c, _, _) = measure_geometry(&world, &web_off);
    let (align_b, _, _) = measure_geometry(&world, &web_off);

    let mc = median3(align_c.clone());
    let mb = median3(align_b.clone());
    const MARGIN: f64 = 1.15;
    assert!(
        !(mc > mb * MARGIN),
        "w_radial=0(放射バイアスなし)同士の比較でもマージンが成立してしまっている \
         (期待は不成立=赤): candidate_median={mc} baseline_median={mb}"
    );
}

// ---------- ② 同心リング（web on でリング辺・冗長度が増える） ----------

#[test]
fn accept2_ring_formation_adds_circumferential_edges() {
    let (world, _hx, _hy) = world_and_home();
    let web = web_params();
    let mut ring_off = web;
    ring_off.ring_period = 0; // リングのみ無効化（放射は on のまま・分離比較）

    let (_, rings_web, red_web) = measure_geometry(&world, &web);
    let (_, rings_off, _) = measure_geometry(&world, &ring_off);

    let m_web = median3(rings_web.clone());
    let m_off = median3(rings_off.clone());
    const MARGIN: f64 = 2.0;
    assert!(
        m_web > m_off * MARGIN,
        "web構成(ring_period>0)でリング辺(円周方向の辺)が ring_period=0 より有意に多くなって \
         いない: web_median={m_web} off_median={m_off} (web={:?} off={:?})",
        rings_web,
        rings_off
    );

    let m_red = median3(red_web.clone());
    assert!(
        m_red > 1.0,
        "web構成で冗長度(edges-(nodes-1))が1を超えていない(ループを持たない): median={m_red} \
         (values={:?})",
        red_web
    );
}

/// §7 exemplary: ring_period=0 同士（差なし）では当然マージンが成立しない（赤）ことを確認する。
#[test]
fn accept2_baseline_ring_period_zero_fails_margin_red() {
    let (world, _hx, _hy) = world_and_home();
    let mut ring_off = web_params();
    ring_off.ring_period = 0;

    let (_, rings_c, _) = measure_geometry(&world, &ring_off);
    let (_, rings_b, _) = measure_geometry(&world, &ring_off);

    let mc = median3(rings_c.clone());
    let mb = median3(rings_b.clone());
    const MARGIN: f64 = 2.0;
    assert!(
        !(mc > mb * MARGIN),
        "ring_period=0(リングなし)同士の比較でもマージンが成立してしまっている \
         (期待は不成立=赤): candidate_median={mc} baseline_median={mb}"
    );
}

// ---------- ③ forage 併存（B案の核: web構成でも餌探索・標高忌避が保たれる） ----------

/// home からほぼ距離 d にある陸セルのうち最も標高が高い/低いものを探す
/// （netphys_005 find_high_low_targets と同趣旨・同じ座標決定を踏襲）。
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

/// netphys_001/005 の nearest_land と同じ座標決定（低〜中標高ターゲット用）。
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

/// home と目標座標へ砂糖(strength=2000)を置き ticks 回した末、home 近傍ノードと目標近傍
/// (半径5)ノードが `flow_connected` かどうかを3シード返す（netphys_005 と同趣旨）。
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

/// 高標高(e≈0.979, home からほぼ距離18)の砂糖が、web構成でも中央値で見捨てられる
/// （netphys-005 accept1 相当が web でも保たれる＝標高忌避が残っている）。
#[test]
fn accept3a_web_still_abandons_high_elevation_sugar() {
    let (world, hx, hy) = world_and_home();
    let web = web_params();
    let (high_xy, _) = find_high_low_targets(&world, hx, hy, 18.0, 2.0);
    assert!(
        e_at(&world, high_xy.0, high_xy.1) >= web.attract_e_hi,
        "高標高ターゲットの実測標高が attract_e_hi 未満（world/しきい前提が崩れている）"
    );

    let ticks = 720u64;
    let flags = measure_connected_flags(&world, hx, hy, high_xy, &web, ticks);
    let m = median3(flags.clone());
    assert!(
        m < 1.0,
        "web構成でも高標高の砂糖が中央値で連結してしまっている(標高忌避が壊れている・B案不成立の \
         疑い): flags={:?}",
        flags
    );
}

/// 低〜中標高(e≈0.712, home+12)の砂糖は、web構成でも中央値で連結する
/// （netphys-005 accept2 相当が web でも保たれる＝蜘蛛の巣バイアスが餌探索を壊していない）。
#[test]
fn accept3b_web_still_connects_low_mid_elevation_sugar() {
    let (world, hx, hy) = world_and_home();
    let web = web_params();
    let mid_xy = nearest_land(&world, hx + 12.0, hy);
    assert!(
        e_at(&world, mid_xy.0, mid_xy.1) < web.attract_e_hi,
        "低〜中標高ターゲットの実測標高が attract_e_hi 以上（world/しきい前提が崩れている）"
    );

    let ticks = 400u64;
    let flags = measure_connected_flags(&world, hx, hy, mid_xy, &web, ticks);
    let m = median3(flags.clone());
    assert!(
        m >= 1.0,
        "web構成で低〜中標高の砂糖が中央値で連結しない(forage併存が壊れている): flags={:?}",
        flags
    );
}

// ---------- 回帰: 有界性（web構成でも cap 内） ----------

#[test]
fn accept4_bounded_under_web_config() {
    let (world, _hx, _hy) = world_and_home();
    let web = web_params();
    for &seed in &SEEDS {
        let mut s = initial_net_state(seed, &world, &web);
        for _ in 0..TICKS {
            netphys_step(&mut s, &world, &web, &[]);
        }
        assert!(
            s.n_nodes() <= web.node_cap,
            "seed {seed}: node_cap 超過 nodes={} cap={}",
            s.n_nodes(),
            web.node_cap
        );
        assert!(
            s.n_edges() <= web.edge_cap,
            "seed {seed}: edge_cap 超過 edges={} cap={}",
            s.n_edges(),
            web.edge_cap
        );
    }
}

// ---------- 回帰: 既定オフで従来と厳密一致（後方互換） ----------

#[test]
fn accept5_default_params_are_unaffected() {
    let np = NetParams::default();
    assert_eq!(np.w_radial, 0.0, "既定 w_radial は 0（後方互換）でなければならない");
    assert_eq!(np.ring_period, 0, "既定 ring_period は 0（後方互換）でなければならない");
}
