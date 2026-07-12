//! netphys-001: 一般グラフ（NetState の nodes/edges）向けの最小決定論 Kirchhoff ソルバ。
//!
//! `analysis::flow` はラスタ骨格由来の `NetworkGraph`（node_px=画素index, 砂糖tap前提）に
//! 強く結合しているため、座標が自由な一般グラフの netphys にそのまま被せるのは不自然。
//! そこで密行列の線形代数部分だけ `analysis::flow::solve_dense`（可視性のみ pub(crate) 化,
//! 挙動無変更）を再利用し、Laplacian の組み立て・端子選択は netphys 側で独自に行う。
//! 乱数は使わない（決定的）。

use crate::analysis::flow::solve_dense;
use crate::world::World;

use super::state::{NEdge, NNode};

#[derive(Clone, Debug, Default)]
pub struct NetFlowResult {
    pub connected: bool,
    pub effective_resistance: f64,
    pub total_conductance: f64,
    pub edge_currents: Vec<f64>, // edges と同じ順・同じ長さ（非連結時は 0 埋め）
}

fn uf_find(uf: &mut [usize], a: usize) -> usize {
    let mut r = a;
    while uf[r] != r {
        r = uf[r];
    }
    let mut c = a;
    while uf[c] != r {
        let nx = uf[c];
        uf[c] = r;
        c = nx;
    }
    r
}
fn uf_union(uf: &mut [usize], a: usize, b: usize) {
    let (ra, rb) = (uf_find(uf, a), uf_find(uf, b));
    if ra != rb {
        if ra < rb {
            uf[rb] = ra;
        } else {
            uf[ra] = rb;
        }
    }
}

/// 辺のコンダクタンス g_e = d_e / L_eff_e, L_eff = l*(1+alpha*meanE)（標高はソフト忌避＝抵抗増）。
fn edge_conductance(e: &NEdge, nodes: &[NNode], world: &World, alpha: f64) -> f64 {
    let mean_e = mean_elevation_of_edge(e, nodes, world);
    let l_eff = e.l * (1.0 + alpha * mean_e);
    if l_eff <= 1.0e-9 {
        0.0
    } else {
        e.d / l_eff
    }
}

/// 辺の両端点セルの標高平均（直線補間はせず両端のみ・決定的で十分な近似）。
fn mean_elevation_of_edge(e: &NEdge, nodes: &[NNode], world: &World) -> f64 {
    let ea = sample_e(world, nodes[e.a].x, nodes[e.a].y);
    let eb = sample_e(world, nodes[e.b].x, nodes[e.b].y);
    (ea + eb) * 0.5
}

pub fn sample_e(world: &World, x: f64, y: f64) -> f64 {
    let (h, w) = (world.h, world.w);
    let fx = x.floor();
    let fy = y.floor();
    let inb = fx >= 0.0 && fx < w as f64 && fy >= 0.0 && fy < h as f64;
    if !inb {
        return 0.0;
    }
    world.e[(fy as usize) * w + (fx as usize)] as f64
}

/// 一般グラフの Kirchhoff を1回解く（source→sink に単位電流）。乱数不要・決定的。
pub fn solve(
    nodes: &[NNode],
    edges: &[NEdge],
    world: &World,
    alpha: f64,
    source: usize,
    sink: usize,
) -> NetFlowResult {
    let n = nodes.len();
    if n == 0 || source >= n || sink >= n || source == sink {
        return NetFlowResult::default();
    }

    let mut uf: Vec<usize> = (0..n).collect();
    for e in edges {
        uf_union(&mut uf, e.a, e.b);
    }
    let connected = uf_find(&mut uf, source) == uf_find(&mut uf, sink);
    if !connected {
        return NetFlowResult {
            connected: false,
            effective_resistance: -1.0,
            total_conductance: 0.0,
            edge_currents: vec![0.0; edges.len()],
        };
    }

    let conds: Vec<f64> = edges.iter().map(|e| edge_conductance(e, nodes, world, alpha)).collect();

    let mut lap = vec![vec![0.0f64; n]; n];
    for (e, &gc) in edges.iter().zip(conds.iter()) {
        if gc <= 0.0 {
            continue;
        }
        lap[e.a][e.a] += gc;
        lap[e.b][e.b] += gc;
        lap[e.a][e.b] -= gc;
        lap[e.b][e.a] -= gc;
    }

    let root = uf_find(&mut uf, source);
    let map: Vec<usize> = (0..n).filter(|&i| i != sink && uf_find(&mut uf, i) == root).collect();
    let m = map.len();
    let mut a = vec![vec![0.0f64; m]; m];
    let mut b = vec![0.0f64; m];
    for (ri, &oi) in map.iter().enumerate() {
        for (ci, &oj) in map.iter().enumerate() {
            a[ri][ci] = lap[oi][oj];
        }
        if oi == source {
            b[ri] = 1.0;
        }
    }

    let sol = solve_dense(a, b);
    let mut v = vec![0.0f64; n]; // sink = 0 基準
    match sol {
        Some(x) => {
            for (ri, &oi) in map.iter().enumerate() {
                v[oi] = x[ri];
            }
        }
        None => {
            return NetFlowResult {
                connected: false,
                effective_resistance: -1.0,
                total_conductance: 0.0,
                edge_currents: vec![0.0; edges.len()],
            };
        }
    }

    let eff_res = v[source] - v[sink];
    let total_conductance = if eff_res > 1.0e-12 { 1.0 / eff_res } else { 0.0 };

    let edge_currents: Vec<f64> = edges
        .iter()
        .zip(conds.iter())
        .map(|(e, &gc)| (gc * (v[e.a] - v[e.b])).abs())
        .collect();

    NetFlowResult { connected: true, effective_resistance: eff_res, total_conductance, edge_currents }
}
