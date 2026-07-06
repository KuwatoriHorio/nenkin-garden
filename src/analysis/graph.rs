//! グラフ化（analysis-001 パイプライン 中段）。
//!
//! ノード = 骨格の分岐点(deg>=3)・端点(deg<=1) + ループのみ成分で昇格した代表画素。
//! エッジ = ノード間の degree-2 チェーン。実距離 length と加重平均標高 mean_e を持つ。
//! 砂糖源は流れの端子として別途扱う（graph には含めず num_cc の core 整合を保つ）。
//!
//! 正準順序: ノード id は raster 走査順、エッジは (a,b,length,first_pixel) で安定ソート。
//! 自己ループは破棄、多重辺は保持（受け入れ #2 の定義）。

use std::collections::HashSet;

use crate::world::World;

const DIRS8: [(i64, i64); 8] = [
    (-1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
    (1, 0),
    (1, -1),
    (0, -1),
    (-1, -1),
];

#[inline]
fn step_len(d: (i64, i64)) -> f64 {
    if d.0 != 0 && d.1 != 0 {
        std::f64::consts::SQRT_2
    } else {
        1.0
    }
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub a: usize, // ノード id（a <= b）
    pub b: usize,
    pub length: f64,
    pub mean_e: f64,
}

#[derive(Clone, Debug)]
pub struct NetworkGraph {
    pub node_px: Vec<usize>,   // ノードの画素 index（id=Vec index, raster順）
    pub edges: Vec<Edge>,
    pub num_cc: usize,
    pub largest_cc: usize,     // 最大成分のノード数
    pub node_comp: Vec<usize>, // ノードごとの成分 id
}

struct Uf {
    p: Vec<usize>,
}
impl Uf {
    fn new(n: usize) -> Self {
        Uf { p: (0..n).collect() }
    }
    fn find(&mut self, a: usize) -> usize {
        let mut r = a;
        while self.p[r] != r {
            r = self.p[r];
        }
        let mut c = a;
        while self.p[c] != r {
            let nx = self.p[c];
            self.p[c] = r;
            c = nx;
        }
        r
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            if ra < rb {
                self.p[rb] = ra;
            } else {
                self.p[ra] = rb;
            }
        }
    }
}

/// 骨格 mask からネットワークグラフを構築する。
pub fn build(skel: &[bool], world: &World) -> NetworkGraph {
    let (h, w) = (world.h, world.w);

    // 度数（8近傍の骨格画素数）
    let mut deg = vec![0u8; h * w];
    for y in 0..h {
        for x in 0..w {
            if !skel[y * w + x] {
                continue;
            }
            let mut d = 0u8;
            for &(dy, dx) in &DIRS8 {
                let ny = y as i64 + dy;
                let nx = x as i64 + dx;
                if ny >= 0 && ny < h as i64 && nx >= 0 && nx < w as i64 && skel[ny as usize * w + nx as usize] {
                    d += 1;
                }
            }
            deg[y * w + x] = d;
        }
    }

    // 骨格成分（8連結）: num_cc は core と整合させる基準
    let mut uf = Uf::new(h * w);
    for y in 0..h {
        for x in 0..w {
            if !skel[y * w + x] {
                continue;
            }
            for &(dy, dx) in &DIRS8 {
                let ny = y as i64 + dy;
                let nx = x as i64 + dx;
                if ny >= 0 && ny < h as i64 && nx >= 0 && nx < w as i64 && skel[ny as usize * w + nx as usize] {
                    uf.union(y * w + x, ny as usize * w + nx as usize);
                }
            }
        }
    }

    // ノード判定: deg != 2
    let mut is_node = vec![false; h * w];
    for i in 0..h * w {
        if skel[i] && deg[i] != 2 {
            is_node[i] = true;
        }
    }
    // ループのみ成分（deg 全て2）には代表画素（raster最小）をノードとして昇格
    {
        let mut comp_has_node: std::collections::HashMap<usize, bool> = std::collections::HashMap::new();
        let mut comp_min: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        for i in 0..h * w {
            if !skel[i] {
                continue;
            }
            let r = uf.find(i);
            comp_min.entry(r).and_modify(|m| { if i < *m { *m = i } }).or_insert(i);
            let e = comp_has_node.entry(r).or_insert(false);
            if is_node[i] {
                *e = true;
            }
        }
        for (r, has) in comp_has_node {
            if !has {
                is_node[comp_min[&r]] = true;
            }
        }
    }

    // ノード id 割り当て（raster順）
    let mut node_px: Vec<usize> = Vec::new();
    let mut node_id = vec![usize::MAX; h * w];
    for i in 0..h * w {
        if is_node[i] {
            node_id[i] = node_px.len();
            node_px.push(i);
        }
    }

    // エッジ抽出（dart 重複排除）
    let mut edges: Vec<Edge> = Vec::new();
    let mut visited_darts: HashSet<(usize, usize)> = HashSet::new();

    for &u in &node_px {
        let uy = (u / w) as i64;
        let ux = (u % w) as i64;
        for &(dy, dx) in &DIRS8 {
            let ny = uy + dy;
            let nx = ux + dx;
            if ny < 0 || ny >= h as i64 || nx < 0 || nx >= w as i64 {
                continue;
            }
            let first = ny as usize * w + nx as usize;
            if !skel[first] {
                continue;
            }
            if visited_darts.contains(&(u, first)) {
                continue;
            }

            // u -> first からチェーンを辿り、次のノードまで進む
            let mut pixels: Vec<usize> = vec![u];
            let mut length = 0.0f64;
            let mut prev = u;
            let mut cur = first;
            loop {
                pixels.push(cur);
                let py = (prev / w) as i64;
                let px = (prev % w) as i64;
                let cy = (cur / w) as i64;
                let cx = (cur % w) as i64;
                length += step_len((cy - py, cx - px));
                if is_node[cur] {
                    break;
                }
                // deg==2 の非ノード: prev 以外の骨格隣接へ進む
                let mut next = usize::MAX;
                for &(ddy, ddx) in &DIRS8 {
                    let ty = cy + ddy;
                    let tx = cx + ddx;
                    if ty < 0 || ty >= h as i64 || tx < 0 || tx >= w as i64 {
                        continue;
                    }
                    let t = ty as usize * w + tx as usize;
                    if skel[t] && t != prev {
                        next = t;
                        break;
                    }
                }
                if next == usize::MAX {
                    // 行き止まり（理論上 deg>=2 だが安全弁）
                    break;
                }
                prev = cur;
                cur = next;
            }

            let far = cur;
            // dart と逆 dart を訪問済みに
            visited_darts.insert((u, first));
            visited_darts.insert((far, prev));

            if far == u {
                continue; // 自己ループは破棄
            }

            let mean_e = {
                let mut s = 0.0;
                for &pix in &pixels {
                    s += world.e[pix] as f64;
                }
                s / pixels.len() as f64
            };
            let (a, b) = {
                let ia = node_id[u];
                let ib = node_id[far];
                if ia <= ib { (ia, ib) } else { (ib, ia) }
            };
            edges.push(Edge { a, b, length, mean_e });
        }
    }

    // 正準ソート（a, b, length, mean_e）
    edges.sort_by(|e1, e2| {
        e1.a
            .cmp(&e2.a)
            .then(e1.b.cmp(&e2.b))
            .then(e1.length.partial_cmp(&e2.length).unwrap())
            .then(e1.mean_e.partial_cmp(&e2.mean_e).unwrap())
    });

    // ノード成分 id（骨格成分を 0.. に正規化, raster最小順）
    let mut roots: Vec<usize> = Vec::new();
    let mut root_index: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut node_comp = vec![0usize; node_px.len()];
    // 成分ルートを raster 最小画素で代表させ、出現順に id 付け
    let mut comp_size: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for (nid, &pix) in node_px.iter().enumerate() {
        let r = uf.find(pix);
        let ci = *root_index.entry(r).or_insert_with(|| {
            roots.push(r);
            roots.len() - 1
        });
        node_comp[nid] = ci;
        *comp_size.entry(ci).or_insert(0) += 1;
    }
    // num_cc は「骨格の全成分数」。ノードは各成分に最低1つあるため node 成分数と一致。
    let num_cc = roots.len();
    let largest_cc = comp_size.values().copied().max().unwrap_or(0);

    NetworkGraph {
        node_px,
        edges,
        num_cc,
        largest_cc,
        node_comp,
    }
}

/// 最小全域森の総延長（Kruskal, length 昇順・決定的）。
pub fn mst_length(g: &NetworkGraph) -> f64 {
    let mut idx: Vec<usize> = (0..g.edges.len()).collect();
    idx.sort_by(|&i, &j| {
        let e1 = &g.edges[i];
        let e2 = &g.edges[j];
        e1.length
            .partial_cmp(&e2.length)
            .unwrap()
            .then(e1.a.cmp(&e2.a))
            .then(e1.b.cmp(&e2.b))
    });
    let mut uf = Uf::new(g.node_px.len());
    let mut total = 0.0;
    for i in idx {
        let e = &g.edges[i];
        if uf.find(e.a) != uf.find(e.b) {
            uf.union(e.a, e.b);
            total += e.length;
        }
    }
    total
}
