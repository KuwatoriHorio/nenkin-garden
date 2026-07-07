//! render-002: ネットワーク/グラフの静的 SVG 可視化（render レイヤ, std のみ）。
//!
//! `analysis::analyze` の出力（グラフ・MST・流量）を読むだけで SVG 文字列を生成する。
//! core / analysis を**読むだけ**・逆依存なし（設計メモ §2）。State は書き換えない。
//! 決定的: 同一 State → 同一 SVG（要素の描画順を正準化）。
//!
//! 描画: エッジ太さ=流量 |I_e| / MST=実線・冗長辺=破線 / 連結成分=色分け / 砂糖源=source·sink。

use crate::analysis::{analyze, graph::NetworkGraph};
use crate::params::Params;
use crate::state::State;
use crate::world::World;

/// 流量 → 線幅 の写像（純関数, 単調増加）。0 で最小, max で最大。
pub fn flow_width(current: f64, max_current: f64) -> f64 {
    const MIN_W: f64 = 1.0;
    const MAX_W: f64 = 6.0;
    if max_current <= 1e-12 {
        MIN_W
    } else {
        MIN_W + (MAX_W - MIN_W) * (current / max_current).clamp(0.0, 1.0)
    }
}

/// MST（最小全域森）に含まれるエッジを bool で返す（Kruskal, graph::mst_length と同順序）。
fn mst_edge_set(g: &NetworkGraph) -> Vec<bool> {
    let mut idx: Vec<usize> = (0..g.edges.len()).collect();
    idx.sort_by(|&i, &j| {
        let (e1, e2) = (&g.edges[i], &g.edges[j]);
        e1.length
            .partial_cmp(&e2.length)
            .unwrap()
            .then(e1.a.cmp(&e2.a))
            .then(e1.b.cmp(&e2.b))
    });
    let n = g.node_px.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], a: usize) -> usize {
        let mut r = a;
        while p[r] != r {
            r = p[r];
        }
        let mut c = a;
        while p[c] != r {
            let nx = p[c];
            p[c] = r;
            c = nx;
        }
        r
    }
    let mut in_mst = vec![false; g.edges.len()];
    for i in idx {
        let e = &g.edges[i];
        let (ra, rb) = (find(&mut parent, e.a), find(&mut parent, e.b));
        if ra != rb {
            parent[ra.max(rb)] = ra.min(rb);
            in_mst[i] = true;
        }
    }
    in_mst
}

// 連結成分の色分けパレット（フラット・決定的）。
const PALETTE: [&str; 8] = [
    "#7cf698", "#38f0e0", "#ffd166", "#f78c6b", "#c792ea", "#89ddff", "#a3be8c", "#e0aaff",
];

/// State（特に trail）を読むだけでグラフの SVG 文字列を生成する（非侵襲・決定的）。
pub fn graph_to_svg(state: &State, world: &World, p: &Params) -> String {
    let res = analyze(state, world, p);
    let g = &res.graph;
    let cur = &res.edge_currents; // graph.edges 順の |I_e|（非連結時は空）
    let has_flow = cur.len() == g.edges.len();
    let maxc = cur.iter().cloned().fold(0.0f64, f64::max);
    let in_mst = mst_edge_set(g);

    let cell = 6.0;
    let (wn, hn) = (world.w, world.h);
    let (width, height) = (wn as f64 * cell, hn as f64 * cell);
    let node_xy = |nid: usize| -> (f64, f64) {
        let pix = g.node_px[nid];
        let cx = ((pix % wn) as f64 + 0.5) * cell;
        let cy = ((pix / wn) as f64 + 0.5) * cell;
        (cx, cy)
    };

    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" \
width=\"{width}\" height=\"{height}\" data-nodes=\"{}\" data-edges=\"{}\">\n",
        g.node_px.len(),
        g.edges.len()
    ));
    // 海の背景
    s.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{width}\" height=\"{height}\" fill=\"#0b1420\"/>\n"
    ));
    // 陸の薄い下地（グラフを際立たせる）
    s.push_str("<g fill=\"#18202e\">\n");
    for y in 0..hn {
        for x in 0..wn {
            if world.land_mask[y * wn + x] {
                s.push_str(&format!(
                    "<rect x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\"/>",
                    x as f64 * cell,
                    y as f64 * cell,
                    cell,
                    cell
                ));
            }
        }
    }
    s.push_str("\n</g>\n");

    // エッジ（graph.edges 順で正準）: 色=成分, 太さ=流量, 破線=冗長辺（MST外）
    s.push_str("<g fill=\"none\" stroke-linecap=\"round\">\n");
    for (i, e) in g.edges.iter().enumerate() {
        let (ax, ay) = node_xy(e.a);
        let (bx, by) = node_xy(e.b);
        let color = PALETTE[g.node_comp[e.a] % PALETTE.len()];
        let w = if has_flow {
            flow_width(cur[i], maxc)
        } else {
            1.5
        };
        let dash = if in_mst[i] { "" } else { " stroke-dasharray=\"4 4\" opacity=\"0.7\"" };
        s.push_str(&format!(
            "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" stroke-width=\"{:.2}\"{}/>\n",
            ax, ay, bx, by, color, w, dash
        ));
    }
    s.push_str("</g>\n");

    // ノード（raster順）
    s.push_str("<g fill=\"#0b1420\" stroke=\"#cfe3ff\" stroke-width=\"1\">\n");
    for nid in 0..g.node_px.len() {
        let (cx, cy) = node_xy(nid);
        s.push_str(&format!("<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"2.2\"/>", cx, cy));
    }
    s.push_str("\n</g>\n");

    // 砂糖源（id昇順で正準。最小id=source, 最大id=sink を白リングで強調）
    let mut order: Vec<usize> = (0..state.sugar_id.len()).collect();
    order.sort_by_key(|&i| state.sugar_id[i]);
    for (rank, &j) in order.iter().enumerate() {
        let cx = state.sugar_x[j] * cell;
        let cy = state.sugar_y[j] * cell;
        let terminal = rank == 0 || rank == order.len() - 1;
        let r = if terminal { 6.0 } else { 4.5 };
        let sw = if terminal { 2.0 } else { 1.0 };
        s.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"#ff5a5a\" stroke=\"#fff\" stroke-width=\"{:.1}\"/>\n",
            cx, cy, r, sw
        ));
    }

    s.push_str("</svg>\n");
    s
}
