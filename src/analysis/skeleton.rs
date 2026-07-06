//! しきい化 + 骨格抽出（analysis-001 パイプライン 前段）。
//!
//! 二値化は core の連結成分と整合させるため `theta_cc` を共有する（受け入れ #3）。
//! 細線化は Zhang-Suen（各サブ反復で削除候補を一括マーク→一括削除）で、
//! 反復順に依存しない決定的アルゴリズム。連結性を保つため成分数は不変（受け入れ #3）。

use crate::params::Params;
use crate::state::State;
use crate::world::World;

/// 陸上かつ trail>theta_cc の網マスク（core の cc 二値化と同一しきい）。
pub fn binarize(state: &State, world: &World, p: &Params) -> Vec<bool> {
    let n = world.h * world.w;
    (0..n)
        .map(|i| world.land_mask[i] && (state.trail[i] as f64) > p.theta_cc)
        .collect()
}

#[inline]
fn get(mask: &[bool], h: usize, w: usize, y: i64, x: i64) -> u8 {
    if y < 0 || x < 0 || y >= h as i64 || x >= w as i64 {
        0
    } else if mask[y as usize * w + x as usize] {
        1
    } else {
        0
    }
}

const DIRS8: [(i64, i64); 8] = [
    (-1, 0), (-1, 1), (0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1),
];

/// トポロジ保存: 元マスクの各 8連結成分に骨格画素が最低1つ残ることを保証する。
///
/// Zhang-Suen は極小成分（2画素等）を丸ごと削除しうるため、成分数が core の num_cc
/// と食い違う。骨格が全滅した成分には代表画素（raster最小）を1つ復元し、
/// 「骨格の成分数 == マスクの成分数 == core num_cc」を成立させる（受け入れ #3）。
pub fn preserve_components(orig_mask: &[bool], skel: &mut [bool], h: usize, w: usize) {
    // 元マスクの 8連結成分（union-find）
    let mut parent: Vec<usize> = (0..h * w).collect();
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
    for y in 0..h {
        for x in 0..w {
            if !orig_mask[y * w + x] {
                continue;
            }
            for &(dy, dx) in &DIRS8 {
                let ny = y as i64 + dy;
                let nx = x as i64 + dx;
                if ny >= 0 && ny < h as i64 && nx >= 0 && nx < w as i64 && orig_mask[ny as usize * w + nx as usize] {
                    let ra = find(&mut parent, y * w + x);
                    let rb = find(&mut parent, ny as usize * w + nx as usize);
                    if ra != rb {
                        if ra < rb { parent[rb] = ra } else { parent[ra] = rb }
                    }
                }
            }
        }
    }
    // 成分ごとに: 骨格画素の有無 と raster最小画素
    use std::collections::HashMap;
    let mut has_skel: HashMap<usize, bool> = HashMap::new();
    let mut min_px: HashMap<usize, usize> = HashMap::new();
    for i in 0..h * w {
        if !orig_mask[i] {
            continue;
        }
        let r = find(&mut parent, i);
        min_px.entry(r).and_modify(|m| { if i < *m { *m = i } }).or_insert(i);
        let e = has_skel.entry(r).or_insert(false);
        if skel[i] {
            *e = true;
        }
    }
    for (r, present) in has_skel {
        if !present {
            skel[min_px[&r]] = true;
        }
    }
}

/// Zhang-Suen 細線化。1画素幅の骨格 mask を返す（決定的）。
pub fn thin(mask_in: &[bool], h: usize, w: usize, max_iter: usize) -> Vec<bool> {
    let mut mask = mask_in.to_vec();

    // P2..P9（時計回り, P2=北）。A/B 計算にこの順序を用いる。
    let order: [(i64, i64); 8] = [
        (-1, 0),  // P2 N
        (-1, 1),  // P3 NE
        (0, 1),   // P4 E
        (1, 1),   // P5 SE
        (1, 0),   // P6 S
        (1, -1),  // P7 SW
        (0, -1),  // P8 W
        (-1, -1), // P9 NW
    ];

    for _ in 0..max_iter {
        let mut changed = false;

        for sub in 0..2 {
            let mut to_delete: Vec<usize> = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    let idx = y * w + x;
                    if !mask[idx] {
                        continue;
                    }
                    let yi = y as i64;
                    let xi = x as i64;
                    let p: [u8; 8] = [
                        get(&mask, h, w, yi + order[0].0, xi + order[0].1),
                        get(&mask, h, w, yi + order[1].0, xi + order[1].1),
                        get(&mask, h, w, yi + order[2].0, xi + order[2].1),
                        get(&mask, h, w, yi + order[3].0, xi + order[3].1),
                        get(&mask, h, w, yi + order[4].0, xi + order[4].1),
                        get(&mask, h, w, yi + order[5].0, xi + order[5].1),
                        get(&mask, h, w, yi + order[6].0, xi + order[6].1),
                        get(&mask, h, w, yi + order[7].0, xi + order[7].1),
                    ];
                    let b: u8 = p.iter().sum();
                    if b < 2 || b > 6 {
                        continue;
                    }
                    // A = 0→1 遷移数（P2,P3,...,P9,P2）
                    let mut a = 0u8;
                    for k in 0..8 {
                        let cur = p[k];
                        let nxt = p[(k + 1) % 8];
                        if cur == 0 && nxt == 1 {
                            a += 1;
                        }
                    }
                    if a != 1 {
                        continue;
                    }
                    // p index: P2=0,P4=2,P6=4,P8=6
                    let (c1, c2) = if sub == 0 {
                        // Step1: P2*P4*P6=0 かつ P4*P6*P8=0
                        (p[0] * p[2] * p[4], p[2] * p[4] * p[6])
                    } else {
                        // Step2: P2*P4*P8=0 かつ P2*P6*P8=0
                        (p[0] * p[2] * p[6], p[0] * p[4] * p[6])
                    };
                    if c1 == 0 && c2 == 0 {
                        to_delete.push(idx);
                    }
                }
            }
            if !to_delete.is_empty() {
                changed = true;
                for idx in to_delete {
                    mask[idx] = false;
                }
            }
        }

        if !changed {
            break;
        }
    }

    mask
}
