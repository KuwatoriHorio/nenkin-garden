//! netphys-004: 網 Physarum の Phase1 探索を「扇状拡散」に変える受け入れテスト。
//! Jones でも tree でもない第3の独立モデル。netphys-001/002/003 の緑は
//! `tests/netphys_001.rs`・`tests/netphys_002.rs`・`tests/netphys_003.rs` が引き続き検証する
//! （本ファイルは無変更・別ファイル追加）。
//!
//! 主判定: `NetParams::default()`（fan_count>1・誘引の無い純粋探索前線でのみ扇状 probe を張る）で、
//! 探索の広がりが**面的**であることを、home からの角度カバレッジ／占有グリッドセル数で定量する。
//! §7 exemplary: 同一 world/seed で `fan_count=1`（現状＝単一方向の線的伸長）にすると同じマージンが
//! 成立しないことを赤として確認したうえで、既定（扇状が効く値）で成立することを見る。
//!
//! S9（正準9シード）の部分集合 [1, 42, 1337] の中央値で判定する（規約 §4 の集計法を踏襲）。

use nenkin_garden::netphys::{initial_net_state, netphys_step, NetParams, NetState};
use nenkin_garden::params::Params;
use nenkin_garden::world::{make_synthetic_archipelago, World};

const SEEDS: [u64; 3] = [1, 42, 1337];
const TICKS: u64 = 400;
const CELL: f64 = 2.0;
// 探索用一時テストで実測: fan_count=1(線)では占有セル数中央値が~30、既定(fan_count=2)では~54
// (同一 world/seed, ticks=400)。1.3倍マージンは固有ノイズに対して十分な余裕を持つ値として採用。
const MARGIN: f64 = 1.3;

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

/// 占有グリッドセル数（幅 CELL）: ノード座標をセルへ量子化し、ユニークなセル数を数える。
/// 「線状に伸びる」なら少数の細い帯に収まり、面的に開けば多くのセルへ広がる。
fn occupied_cells(s: &NetState, cell: f64) -> usize {
    let mut set = std::collections::HashSet::new();
    for nd in &s.nodes {
        let cx = (nd.x / cell).floor() as i64;
        let cy = (nd.y / cell).floor() as i64;
        set.insert((cx, cy));
    }
    set.len()
}

/// 角度カバレッジ: home から見た各ノードの方位角を 36 bins(10度刻み)に量子化し、
/// 1ノード以上を含むビン数を返す（home から極端に近いノードは方位が不安定なので除外）。
fn angle_coverage_bins(s: &NetState, hx: f64, hy: f64, min_r: f64) -> usize {
    let mut bins = [false; 36];
    for nd in &s.nodes {
        let dx = nd.x - hx;
        let dy = nd.y - hy;
        let r = (dx * dx + dy * dy).sqrt();
        if r < min_r {
            continue;
        }
        let ang = dy.atan2(dx);
        let mut idx = ((ang + std::f64::consts::PI) / (2.0 * std::f64::consts::PI) * 36.0) as isize;
        if idx == 36 {
            idx = 35;
        }
        bins[idx.clamp(0, 35) as usize] = true;
    }
    bins.iter().filter(|&&b| b).count()
}

/// 砂糖なし（純粋な探索・誘引なしの純粋前線を扇状化の対象にする netphys-004 の主眼と一致させる）で
/// TICKS 進めた末の (占有セル数, 角度カバレッジ) を、指定 NetParams で 3 シード分測る。
fn measure(world: &World, np: &NetParams) -> (Vec<f64>, Vec<f64>) {
    let (hx, hy) = world.default_home(np.e_lo);
    let mut cells = Vec::new();
    let mut cov = Vec::new();
    for &seed in &SEEDS {
        let mut s = initial_net_state(seed, world, np);
        for _ in 0..TICKS {
            netphys_step(&mut s, world, np, &[]);
        }
        cells.push(occupied_cells(&s, CELL) as f64);
        cov.push(angle_coverage_bins(&s, hx, hy, 3.0) as f64);
    }
    (cells, cov)
}

// ---------- 主判定: 扇状拡散＝面的に広がる ----------

#[test]
fn accept1_fan_diffusion_is_areal_not_linear() {
    let (world, _hx, _hy) = world_and_home();
    let np_default = NetParams::default();
    assert!(np_default.fan_count > 1, "既定 fan_count は扇状が効く値(>1)でなければならない");

    let mut np_baseline = np_default;
    np_baseline.fan_count = 1;

    let (cells_d, cov_d) = measure(&world, &np_default);
    let (cells_b, cov_b) = measure(&world, &np_baseline);

    let mcd = median3(cells_d.clone());
    let mcb = median3(cells_b.clone());
    assert!(
        mcd > mcb * MARGIN,
        "扇状拡散(既定)で占有セル数が線的(fan_count=1)より有意に増えていない: \
         default_median={mcd} baseline_median={mcb} (default={:?} baseline={:?})",
        cells_d,
        cells_b
    );

    let mvd = median3(cov_d.clone());
    let mvb = median3(cov_b.clone());
    assert!(
        mvd > mvb * MARGIN,
        "扇状拡散(既定)で角度カバレッジが線的(fan_count=1)より有意に増えていない: \
         default_median={mvd} baseline_median={mvb} (default={:?} baseline={:?})",
        cov_d,
        cov_b
    );
}

/// §7 exemplary: 同一 world/seed で fan_count=1（現状＝線的伸長）にすると、上と同じマージンが
/// （自分自身との比較として）成立しない（赤）ことを確認する。
#[test]
fn accept1_baseline_fan_count_1_fails_margin_red() {
    let (world, _hx, _hy) = world_and_home();
    let mut np0 = NetParams::default();
    np0.fan_count = 1;

    let (cells_c, cov_c) = measure(&world, &np0);
    let (cells_b, cov_b) = measure(&world, &np0);

    let mcc = median3(cells_c.clone());
    let mcb = median3(cells_b.clone());
    assert!(
        !(mcc > mcb * MARGIN),
        "fan_count=1(現状の線的伸長)でも占有セル数マージンが成立してしまっている \
         (期待は不成立=赤): candidate_median={mcc} baseline_median={mcb}"
    );

    let mvc = median3(cov_c.clone());
    let mvb = median3(cov_b.clone());
    assert!(
        !(mvc > mvb * MARGIN),
        "fan_count=1(現状の線的伸長)でも角度カバレッジマージンが成立してしまっている \
         (期待は不成立=赤): candidate_median={mvc} baseline_median={mvb}"
    );
}

// ---------- 回帰: 有界性（面的拡散でも cap 内） ----------

#[test]
fn accept2_bounded_under_fan_diffusion() {
    let (world, _hx, _hy) = world_and_home();
    let np = NetParams::default();
    for &seed in &SEEDS {
        let mut s = initial_net_state(seed, &world, &np);
        for _ in 0..TICKS {
            netphys_step(&mut s, &world, &np, &[]);
        }
        assert!(
            s.n_nodes() <= np.node_cap,
            "seed {seed}: node_cap 超過 nodes={} cap={}",
            s.n_nodes(),
            np.node_cap
        );
        assert!(
            s.n_edges() <= np.edge_cap,
            "seed {seed}: edge_cap 超過 edges={} cap={}",
            s.n_edges(),
            np.edge_cap
        );
    }
}
