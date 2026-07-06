//! 流れを1回だけ解く（analysis-001 パイプライン 後段, 反復なし）。
//!
//! 砂糖源を source/sink とするキルヒホッフ線形系 L·v = I を1回解く。
//! 端子は「砂糖源ノード」= 最近傍の骨格ノードへ tap エッジで接続（graph 本体は不変）。
//! source = 砂糖 id 最小, sink = 砂糖 id 最大（正準）。1本の単位電流を流す。

use crate::analysis::graph::NetworkGraph;
use crate::params::Params;
use crate::state::State;
use crate::world::World;

#[derive(Clone, Debug, Default)]
pub struct FlowResult {
    pub connected: bool,        // source/sink が同一成分か
    pub effective_resistance: f64,
    pub total_conductance: f64,
    pub transport_efficiency: f64, // 幹線集約度（HHI, 後述）
}

/// 密行列ガウス消去（部分ピボット, 決定的）。特異なら None。
fn solve_dense(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        // 部分ピボット（最大絶対値, 同値は先の行）
        let mut piv = col;
        let mut best = a[col][col].abs();
        for r in (col + 1)..n {
            let v = a[r][col].abs();
            if v > best {
                best = v;
                piv = r;
            }
        }
        if best < 1e-12 {
            return None; // 特異（source/sink 非連結など）
        }
        if piv != col {
            a.swap(col, piv);
            b.swap(col, piv);
        }
        let d = a[col][col];
        for r in (col + 1)..n {
            let f = a[r][col] / d;
            if f != 0.0 {
                for c in col..n {
                    a[r][c] -= f * a[col][c];
                }
                b[r] -= f * b[col];
            }
        }
    }
    // 後退代入
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut s = b[i];
        for c in (i + 1)..n {
            s -= a[i][c] * x[c];
        }
        x[i] = s / a[i][i];
    }
    Some(x)
}

/// 砂糖源を端子に、キルヒホッフ系を1回解く。
pub fn solve(
    g: &NetworkGraph,
    state: &State,
    world: &World,
    p: &Params,
) -> FlowResult {
    let ns = g.node_px.len();
    // 砂糖源を id 昇順に整列（正準）
    let mut order: Vec<usize> = (0..state.sugar_id.len()).collect();
    order.sort_by_key(|&i| state.sugar_id[i]);
    let nsug = order.len();
    if ns == 0 || nsug < 2 {
        return FlowResult::default();
    }

    // 各砂糖源を最近傍の骨格ノードへ対応付け（tap）
    let nearest_node = |sx: f64, sy: f64| -> Option<(usize, f64)> {
        let mut best = usize::MAX;
        let mut bestd = f64::INFINITY;
        for (nid, &pix) in g.node_px.iter().enumerate() {
            let ny = (pix / world.w) as f64 + 0.5;
            let nx = (pix % world.w) as f64 + 0.5;
            let d = ((nx - sx).powi(2) + (ny - sy).powi(2)).sqrt();
            if d < bestd {
                bestd = d;
                best = nid;
            }
        }
        if best == usize::MAX { None } else { Some((best, bestd)) }
    };

    // 拡張ノード数 = 骨格ノード + 砂糖ノード
    let ntot = ns + nsug;
    let mut lap = vec![vec![0.0f64; ntot]; ntot];

    let add_cond = |lap: &mut Vec<Vec<f64>>, i: usize, j: usize, gc: f64| {
        lap[i][i] += gc;
        lap[j][j] += gc;
        lap[i][j] -= gc;
        lap[j][i] -= gc;
    };

    // 骨格エッジのコンダクタンス g = 1/L_eff, L_eff = L*(1+alpha*meanE)
    let mut skel_conds: Vec<(usize, usize, f64)> = Vec::with_capacity(g.edges.len());
    for e in &g.edges {
        let l_eff = e.length * (1.0 + p.net_alpha * e.mean_e);
        let gc = 1.0 / l_eff.max(1e-9);
        add_cond(&mut lap, e.a, e.b, gc);
        skel_conds.push((e.a, e.b, gc));
    }

    // 砂糖ノードの tap: 半径内の全骨格ノードへ接続（無ければ最近傍1点にフォールバック）。
    // 唯一の最近傍でなく近傍網全体へ繋ぐことで、孤立ビーコンスパイクに吸着せず
    // 近傍の実ネットワークに接続する（analysis-002）。走査は node id 昇順で正準。
    let mut tap_edges: Vec<(usize, usize)> = Vec::new();
    for (k, &si) in order.iter().enumerate() {
        let sug_node = ns + k;
        let (sx, sy) = (state.sugar_x[si], state.sugar_y[si]);
        let mut within: Vec<(usize, f64)> = Vec::new();
        for (nid, &pix) in g.node_px.iter().enumerate() {
            let ny = (pix / world.w) as f64 + 0.5;
            let nx = (pix % world.w) as f64 + 0.5;
            let d = ((nx - sx).powi(2) + (ny - sy).powi(2)).sqrt();
            if d < p.tap_radius {
                within.push((nid, d));
            }
        }
        if within.is_empty() {
            if let Some((nn, dist)) = nearest_node(sx, sy) {
                within.push((nn, dist));
            }
        }
        for (nn, dist) in within {
            let gc = 1.0 / dist.max(p.tap_min_len);
            add_cond(&mut lap, sug_node, nn, gc);
            tap_edges.push((sug_node, nn));
        }
    }

    let source = ns; // 砂糖 id 最小
    let sink = ntot - 1; // 砂糖 id 最大

    // 連結判定は拡張グラフ（骨格エッジ + tap エッジ）の実接続で行う（union-find, 決定的）。
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
    let mut uf: Vec<usize> = (0..ntot).collect();
    for e in &g.edges {
        uf_union(&mut uf, e.a, e.b);
    }
    for &(s, n) in &tap_edges {
        uf_union(&mut uf, s, n);
    }
    let connected = uf_find(&mut uf, source) == uf_find(&mut uf, sink);

    if !connected {
        return FlowResult {
            connected: false,
            effective_resistance: -1.0, // 非連結の番兵
            total_conductance: 0.0,
            transport_efficiency: 0.0,
        };
    }

    // sink を接地（電位0）: sink 行/列を除いた縮約系を解く
    let mut map: Vec<usize> = (0..ntot).filter(|&i| i != sink).collect();
    let m = map.len();
    let mut a = vec![vec![0.0f64; m]; m];
    let mut b = vec![0.0f64; m];
    for (ri, &oi) in map.iter().enumerate() {
        for (ci, &oj) in map.iter().enumerate() {
            a[ri][ci] = lap[oi][oj];
        }
        if oi == source {
            b[ri] = 1.0; // 単位電流を source へ注入
        }
    }

    let sol = solve_dense(a, b);
    let mut v = vec![0.0f64; ntot]; // sink = 0
    match sol {
        Some(x) => {
            for (ri, &oi) in map.iter().enumerate() {
                v[oi] = x[ri];
            }
        }
        None => {
            return FlowResult {
                connected: false,
                effective_resistance: -1.0,
                total_conductance: 0.0,
                transport_efficiency: 0.0,
            };
        }
    }
    let _ = &mut map;

    let eff_res = v[source] - v[sink];
    let total_conductance = if eff_res > 1e-12 { 1.0 / eff_res } else { 0.0 };

    // transport_efficiency: 骨格エッジ電流 I_e = g_e*(v_a - v_b) の集約度（HHI）
    let mut currents: Vec<f64> = Vec::with_capacity(skel_conds.len());
    let mut sum_abs = 0.0;
    for (a, b, gc) in &skel_conds {
        let ie = gc * (v[*a] - v[*b]);
        let mag = ie.abs();
        currents.push(mag);
        sum_abs += mag;
    }
    let transport_efficiency = if sum_abs > 1e-12 {
        currents.iter().map(|&c| (c / sum_abs).powi(2)).sum()
    } else {
        0.0
    };

    FlowResult {
        connected: true,
        effective_resistance: eff_res,
        total_conductance,
        transport_efficiency,
    }
}
