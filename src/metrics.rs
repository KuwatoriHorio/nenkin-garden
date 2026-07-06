//! メトリクス算出（設計メモ §8）。State/World を読むだけ・書き換えなし。
//! 連結成分は Union-Find（4/8近傍・params 選択・決定的）。
//! tick_ms は性能計測用で headless 側が付与（決定性ハッシュには含めない）。

use crate::params::Params;
use crate::state::State;
use crate::world::World;

#[derive(Clone, Debug)]
pub struct Metrics {
    pub coverage: f64,
    pub sugar_collected: f64,
    pub consumed_total: f64,
    pub biomass: f64,
    pub n_agents: usize,
    pub max_cc: usize,
    pub num_cc: usize,
    pub mean_trail_hi: f64,
    pub mean_trail_lo: f64,
    pub elev_trail_ratio: f64,
    // 忌避の健全性（metric-thresholds-001）: 固定帯 E_hi/E_lo に依存する elev_trail_ratio は
    // 高標高帯に網が届かないと 0 に退化する。代わりに trail 加重平均標高（常に定義される連続値）と
    // 陸地平均標高との比 elev_avoidance を健全性シグナルとする（<1 なら網が低標高に偏る＝忌避）。
    pub trail_weighted_mean_elevation: f64,
    pub land_mean_elevation: f64,
    pub elev_avoidance: f64,
    pub tick_ms: f64,
}

impl Metrics {
    /// 依存なしの手書き JSON（serde を使わない＝外部依存ゼロ）。
    pub fn to_json(&self) -> String {
        format!(
            "{{\"coverage\":{},\"sugar_collected\":{},\"consumed_total\":{},\
\"biomass\":{},\"n_agents\":{},\"max_cc\":{},\"num_cc\":{},\
\"mean_trail_hi\":{},\"mean_trail_lo\":{},\"elev_trail_ratio\":{},\
\"trail_weighted_mean_elevation\":{},\"land_mean_elevation\":{},\"elev_avoidance\":{},\
\"tick_ms\":{}}}",
            self.coverage,
            self.sugar_collected,
            self.consumed_total,
            self.biomass,
            self.n_agents,
            self.max_cc,
            self.num_cc,
            self.mean_trail_hi,
            self.mean_trail_lo,
            self.elev_trail_ratio,
            self.trail_weighted_mean_elevation,
            self.land_mean_elevation,
            self.elev_avoidance,
            self.tick_ms,
        )
    }
}

struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        UnionFind {
            parent: (0..n).collect(),
        }
    }
    fn find(&mut self, a: usize) -> usize {
        let mut root = a;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut cur = a;
        while self.parent[cur] != root {
            let next = self.parent[cur];
            self.parent[cur] = root;
            cur = next;
        }
        root
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            if ra < rb {
                self.parent[rb] = ra;
            } else {
                self.parent[ra] = rb;
            }
        }
    }
}

/// 二値 mask の (連結成分数, 最大成分セル数)。
fn connected_components(mask: &[bool], h: usize, w: usize, connectivity: u8) -> (usize, usize) {
    let mut uf = UnionFind::new(h * w);
    // 既訪セルとの結合のみで十分（左・上・左上・右上）
    let neigh: &[(i64, i64)] = if connectivity == 8 {
        &[(-1, 0), (0, -1), (-1, -1), (-1, 1)]
    } else {
        &[(-1, 0), (0, -1)]
    };
    let mut count = 0usize;
    for y in 0..h {
        for x in 0..w {
            if !mask[y * w + x] {
                continue;
            }
            count += 1;
            for &(dy, dx) in neigh {
                let ny = y as i64 + dy;
                let nx = x as i64 + dx;
                if ny >= 0 && ny < h as i64 && nx >= 0 && nx < w as i64 {
                    let nidx = ny as usize * w + nx as usize;
                    if mask[nidx] {
                        uf.union(y * w + x, nidx);
                    }
                }
            }
        }
    }
    if count == 0 {
        return (0, 0);
    }
    // ルートごとにサイズ集計
    use std::collections::HashMap;
    let mut sizes: HashMap<usize, usize> = HashMap::new();
    for y in 0..h {
        for x in 0..w {
            if mask[y * w + x] {
                let r = uf.find(y * w + x);
                *sizes.entry(r).or_insert(0) += 1;
            }
        }
    }
    let num_cc = sizes.len();
    let max_cc = *sizes.values().max().unwrap();
    (num_cc, max_cc)
}

/// §8 の全メトリクスを算出（tick_ms は 0.0、headless が上書き）。
pub fn compute_metrics(s: &State, world: &World, p: &Params) -> Metrics {
    let (h, w) = (world.h, world.w);
    let n_land: usize = world.land_mask.iter().filter(|&&b| b).count();

    // coverage
    let mut cover = 0usize;
    for i in 0..h * w {
        if world.land_mask[i] && s.trail[i] as f64 > p.theta_cov {
            cover += 1;
        }
    }
    let coverage = if n_land > 0 {
        cover as f64 / n_land as f64
    } else {
        0.0
    };

    // 連結成分（陸上で二値化）
    let cc_mask: Vec<bool> = (0..h * w)
        .map(|i| world.land_mask[i] && s.trail[i] as f64 > p.theta_cc)
        .collect();
    let (num_cc, max_cc) = connected_components(&cc_mask, h, w, p.cc_connectivity);

    // 標高別 trail 分布（忌避の健康診断）
    let mut sum_hi = 0.0;
    let mut cnt_hi = 0usize;
    let mut sum_lo = 0.0;
    let mut cnt_lo = 0usize;
    // 忌避の健全性（退化しない連続指標）: trail 加重平均標高 と 陸地平均標高
    let mut tw_sum = 0.0; // Σ trail·E
    let mut t_sum = 0.0; // Σ trail
    let mut e_sum = 0.0; // Σ E（陸）
    for i in 0..h * w {
        if !world.land_mask[i] {
            continue;
        }
        let e = world.e[i] as f64;
        let t = s.trail[i] as f64;
        tw_sum += t * e;
        t_sum += t;
        e_sum += e;
        if e >= p.e_hi {
            sum_hi += t;
            cnt_hi += 1;
        }
        if e < p.e_lo {
            sum_lo += t;
            cnt_lo += 1;
        }
    }
    let mean_hi = if cnt_hi > 0 { sum_hi / cnt_hi as f64 } else { 0.0 };
    let mean_lo = if cnt_lo > 0 { sum_lo / cnt_lo as f64 } else { 0.0 };
    let elev_trail_ratio = if mean_lo > 0.0 { mean_hi / mean_lo } else { 0.0 };

    let land_mean_elevation = if n_land > 0 { e_sum / n_land as f64 } else { 0.0 };
    let trail_weighted_mean_elevation = if t_sum > 0.0 { tw_sum / t_sum } else { 0.0 };
    // <1 なら網が陸地平均より低標高に偏る＝ソフト忌避が効いている（常に定義される連続値）。
    let elev_avoidance = if land_mean_elevation > 0.0 {
        trail_weighted_mean_elevation / land_mean_elevation
    } else {
        0.0
    };

    Metrics {
        coverage,
        sugar_collected: s.collected_total,
        consumed_total: s.consumed_total,
        biomass: s.biomass,
        n_agents: s.n_agents(),
        max_cc,
        num_cc,
        mean_trail_hi: mean_hi,
        mean_trail_lo: mean_lo,
        elev_trail_ratio,
        trail_weighted_mean_elevation,
        land_mean_elevation,
        elev_avoidance,
        tick_ms: 0.0,
    }
}
