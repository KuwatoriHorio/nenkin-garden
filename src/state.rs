//! State のデータ構造と初期化・入力適用（設計メモ §3, §7-op）。
//! agents は index 順が正準順序（Vec の並び順）。副作用なし・描画非依存。

use crate::params::Params;
use crate::rng::Rng;
use crate::world::World;

/// input_script の op（園芸型の動詞, §7）。
#[derive(Clone, Debug)]
pub enum Op {
    PlaceSugar { x: f64, y: f64, strength: f64 },
    RemoveSugar { id: u64 },
}

/// 時系列の入力エントリ（tick で発火）。
#[derive(Clone, Debug)]
pub struct ScriptEntry {
    pub tick: u64,
    pub op: Op,
}

#[derive(Clone, Debug)]
pub struct State {
    pub tick: u64,
    pub h: usize,
    pub w: usize,
    pub trail: Vec<f32>, // [h*w] 誘引物質場（海は0）

    // エージェント（index順=正準順序）
    pub ax: Vec<f32>,
    pub ay: Vec<f32>,
    pub ah: Vec<f32>, // heading (rad)

    pub biomass: f64,

    // 砂糖源（id 昇順を正準順序として維持）
    pub sugar_id: Vec<u64>,
    pub sugar_x: Vec<f64>,
    pub sugar_y: Vec<f64>,
    pub sugar_strength: Vec<f64>,
    pub sugar_remaining: Vec<f64>,
    pub next_sugar_id: u64,

    // 保存則の帳簿
    pub collected_total: f64,
    pub consumed_total: f64,

    pub rng: Rng,
}

impl State {
    #[inline]
    pub fn n_agents(&self) -> usize {
        self.ax.len()
    }
}

/// 陸セルから n 個の位置（セル中心 + 微小ジッタ）を復元可能に選ぶ。
fn random_land_positions(rng: &mut Rng, world: &World, n: usize) -> (Vec<f32>, Vec<f32>) {
    let mut land_cells: Vec<usize> = Vec::new();
    for i in 0..world.h * world.w {
        if world.land_mask[i] {
            land_cells.push(i);
        }
    }
    assert!(!land_cells.is_empty(), "world has no land cells");

    let mut xs = Vec::with_capacity(n);
    let mut ys = Vec::with_capacity(n);
    for _ in 0..n {
        let c = land_cells[rng.gen_range(land_cells.len() as u64) as usize];
        let cy = (c / world.w) as f32;
        let cx = (c % world.w) as f32;
        let jx = rng.next_f64() as f32 - 0.5;
        let jy = rng.next_f64() as f32 - 0.5;
        xs.push(cx + 0.5 + jx);
        ys.push(cy + 0.5 + jy);
    }
    (xs, ys)
}

/// core-002: ホーム座標周りのガウス分布で n 個の位置を復元可能に選ぶ。
/// 海/範囲外に落ちたセルはホームセル中心へフォールバック（境界不変条件を守る）。
/// 各エージェント一様2本（Box-Muller）で固定消費 → 決定的。
fn cluster_positions(
    rng: &mut Rng,
    world: &World,
    n: usize,
    hx: f64,
    hy: f64,
    sigma: f64,
) -> (Vec<f32>, Vec<f32>) {
    let (h, w) = (world.h, world.w);
    let home_cell_ok = {
        let (fx, fy) = (hx.floor(), hy.floor());
        let inb = fx >= 0.0 && fx < w as f64 && fy >= 0.0 && fy < h as f64;
        inb && world.land_mask[(fy as usize) * w + (fx as usize)]
    };
    let (hcx, hcy) = if home_cell_ok {
        (hx, hy)
    } else {
        // ホーム自体が陸でない場合は default_home に委ねているはずだが保険として陸重心へ
        let (dx, dy) = world.default_home(1.0);
        (dx, dy)
    };
    let mut xs = Vec::with_capacity(n);
    let mut ys = Vec::with_capacity(n);
    let two_pi = std::f64::consts::TAU;
    for _ in 0..n {
        let u1 = rng.next_f64().max(1e-12);
        let u2 = rng.next_f64();
        let r = (-2.0 * u1.ln()).sqrt();
        let z0 = r * (two_pi * u2).cos();
        let z1 = r * (two_pi * u2).sin();
        let px = hcx + sigma * z0;
        let py = hcy + sigma * z1;
        let (fx, fy) = (px.floor(), py.floor());
        let inb = fx >= 0.0 && fx < w as f64 && fy >= 0.0 && fy < h as f64;
        let onland = inb && world.land_mask[(fy as usize) * w + (fx as usize)];
        if onland {
            xs.push(px as f32);
            ys.push(py as f32);
        } else {
            xs.push(hcx as f32);
            ys.push(hcy as f32);
        }
    }
    (xs, ys)
}

/// 決定論的な初期 State（設計メモ §3）。
/// 保存則の初期整合のため、初期バイオマスは collected_total に計上する。
pub fn initial_state(seed: u64, world: &World, p: &Params) -> State {
    let mut rng = Rng::seed_from_u64(seed);
    let (h, w) = (world.h, world.w);

    let n0 = p.n_init_agents;
    // core-002: init_cluster_sigma>0 ならホーム凝集配置。=0 は従来の一様散布（既定・挙動不変）。
    let (ax, ay) = if p.init_cluster_sigma > 0.0 {
        let (hx, hy) = if p.home_x >= 0.0 && p.home_y >= 0.0 {
            (p.home_x, p.home_y)
        } else {
            world.default_home(p.e_lo)
        };
        cluster_positions(&mut rng, world, n0, hx, hy, p.init_cluster_sigma)
    } else {
        random_land_positions(&mut rng, world, n0)
    };
    let two_pi = std::f64::consts::TAU;
    let ah: Vec<f32> = (0..n0).map(|_| (rng.next_f64() * two_pi) as f32).collect();

    State {
        tick: 0,
        h,
        w,
        trail: vec![0.0f32; h * w],
        ax,
        ay,
        ah,
        biomass: p.initial_biomass,
        sugar_id: Vec::new(),
        sugar_x: Vec::new(),
        sugar_y: Vec::new(),
        sugar_strength: Vec::new(),
        sugar_remaining: Vec::new(),
        next_sugar_id: 0,
        collected_total: p.initial_biomass,
        consumed_total: 0.0,
        rng,
    }
}

/// 単一 op を適用（§7）。副作用は State のみ。
pub fn apply_op(state: &mut State, op: &Op) {
    match *op {
        Op::PlaceSugar { x, y, strength } => {
            let sid = state.next_sugar_id;
            state.next_sugar_id += 1;
            state.sugar_id.push(sid);
            state.sugar_x.push(x);
            state.sugar_y.push(y);
            state.sugar_strength.push(strength);
            state.sugar_remaining.push(strength);
        }
        Op::RemoveSugar { id } => {
            if let Some(pos) = state.sugar_id.iter().position(|&s| s == id) {
                state.sugar_id.remove(pos);
                state.sugar_x.remove(pos);
                state.sugar_y.remove(pos);
                state.sugar_strength.remove(pos);
                state.sugar_remaining.remove(pos);
            }
        }
    }
}
