//! tree-growth-001: α成長木モデルの遷移 `tree_step`（設計メモ §2-α, タスク仕様の1〜6）。
//! 決定論契約: 乱数は state.rng の単一 PRNG のみ・ノード/砂糖は id(index) 昇順で処理。
//! 現行 Jones モデルの step.rs とは独立（並置・無関係、world.rs のみ読み取り共有）。

use crate::state::Op;
use crate::world::World;

use super::state::{apply_tree_op, path_len, remove_depleted_sugar, Node, TreeParams, TreeState};

/// 対象セルが陸かつ範囲内なら標高を返す。海/範囲外は None（境界不変条件・棄却対象）。
fn sample_land_e(world: &World, x: f64, y: f64) -> Option<f64> {
    let (h, w) = (world.h, world.w);
    let fx = x.floor();
    let fy = y.floor();
    let inb = fx >= 0.0 && fx < w as f64 && fy >= 0.0 && fy < h as f64;
    if !inb {
        return None;
    }
    let cix = fx as usize;
    let ciy = fy as usize;
    let idx = ciy * w + cix;
    if !world.land_mask[idx] {
        return None;
    }
    Some(world.e[idx] as f64)
}

/// tip から見た誘引方向（単位ベクトル）と重み（近さ×残量）・距離。砂糖 id 昇順で走査（決定的）。
fn attractor_dirs(state: &TreeState, tip: usize, p: &TreeParams) -> Vec<(f64, f64, f64, f64)> {
    let (tx, ty) = (state.nodes[tip].x as f64, state.nodes[tip].y as f64);
    let mut order: Vec<usize> = (0..state.sugar_id.len()).collect();
    order.sort_by_key(|&j| state.sugar_id[j]);
    let mut out = Vec::new();
    for j in order {
        if state.sugar_remaining[j] <= 0.0 {
            continue;
        }
        let dx = state.sugar_x[j] - tx;
        let dy = state.sugar_y[j] - ty;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > p.attract_radius || dist < 1.0e-9 {
            continue;
        }
        let w = state.sugar_remaining[j] / (dist * dist).max(1.0e-6);
        out.push((dx / dist, dy / dist, w, dist));
    }
    out
}

/// 誘引方向を1〜2クラスタへ分ける（角度差が閾値超なら2クラスタ＝分岐候補）。
/// 各クラスタは (unit_dx, unit_dy, total_weight, min_dist)。min_dist はオーバーシュート防止の
/// 上限（そのクラスタで最も近い誘引点より先へは1tickで進まない）。決定的（sugar id 昇順由来）。
fn cluster_dirs(dirs: &[(f64, f64, f64, f64)], angle_threshold: f64) -> Vec<(f64, f64, f64, f64)> {
    if dirs.is_empty() {
        return Vec::new();
    }
    if dirs.len() == 1 {
        return vec![dirs[0]];
    }
    // 最大角度差のペアを探す（走査順固定で決定的）。
    let mut best_i = 0usize;
    let mut best_j = 1usize;
    let mut best_cos = 1.0f64;
    for i in 0..dirs.len() {
        for j in (i + 1)..dirs.len() {
            let cosang = dirs[i].0 * dirs[j].0 + dirs[i].1 * dirs[j].1;
            if cosang < best_cos {
                best_cos = cosang;
                best_i = i;
                best_j = j;
            }
        }
    }
    let angle = best_cos.clamp(-1.0, 1.0).acos();
    if angle <= angle_threshold {
        // 単一クラスタ: 重み付き平均方向、最小距離を上限に採用。
        let (mut sx, mut sy, mut sw) = (0.0, 0.0, 0.0);
        let mut min_d = f64::INFINITY;
        for &(dx, dy, w, d) in dirs {
            sx += dx * w;
            sy += dy * w;
            sw += w;
            min_d = min_d.min(d);
        }
        let norm = (sx * sx + sy * sy).sqrt().max(1.0e-9);
        return vec![(sx / norm, sy / norm, sw, min_d)];
    }
    // 2クラスタ: best_i / best_j を核に、各誘引を近い方へ割り当て（分岐）。
    let anchor_a = (dirs[best_i].0, dirs[best_i].1);
    let anchor_b = (dirs[best_j].0, dirs[best_j].1);
    let (mut ax, mut ay, mut aw, mut ad) = (0.0, 0.0, 0.0, f64::INFINITY);
    let (mut bx, mut by, mut bw, mut bd) = (0.0, 0.0, 0.0, f64::INFINITY);
    for &(dx, dy, w, d) in dirs {
        let ca = dx * anchor_a.0 + dy * anchor_a.1;
        let cb = dx * anchor_b.0 + dy * anchor_b.1;
        if ca >= cb {
            ax += dx * w;
            ay += dy * w;
            aw += w;
            ad = ad.min(d);
        } else {
            bx += dx * w;
            by += dy * w;
            bw += w;
            bd = bd.min(d);
        }
    }
    let mut out = Vec::new();
    if aw > 0.0 {
        let n = (ax * ax + ay * ay).sqrt().max(1.0e-9);
        out.push((ax / n, ay / n, aw, ad));
    }
    if bw > 0.0 {
        let n = (bx * bx + by * by).sqrt().max(1.0e-9);
        out.push((bx / n, by / n, bw, bd));
    }
    out
}

/// 1tick 進める（in-place）。決定論・ノード/砂糖 id 昇順・単一 PRNG（乱数は現状未使用だが
/// state.rng は保持し、将来の確率的タイブレークに使えるよう契約だけ守る）。
pub fn tree_step(state: &mut TreeState, world: &World, p: &TreeParams, ops: &[Op]) {
    for op in ops {
        apply_tree_op(state, op);
    }

    let n0 = state.nodes.len();

    // tick開始時点の子数（=leaf/tip 判定に使う。tick中に増えるノードは対象外）。
    let mut child_count = vec![0u32; n0];
    for i in 0..n0 {
        if let Some(par) = state.nodes[i].parent {
            child_count[par] += 1;
        }
    }
    let tips: Vec<usize> = (0..n0).filter(|&i| child_count[i] == 0).collect();

    // --- 1. 回収（砂糖 id 昇順） ---
    let m0 = state.sugar_id.len();
    if m0 > 0 && n0 > 0 {
        let mut order: Vec<usize> = (0..m0).collect();
        order.sort_by_key(|&j| state.sugar_id[j]);
        let r2 = p.sugar_radius * p.sugar_radius;
        for j in order {
            if state.sugar_remaining[j] <= 0.0 {
                continue;
            }
            let (sx, sy) = (state.sugar_x[j], state.sugar_y[j]);
            let mut reached = false;
            for i in 0..n0 {
                let dx = state.nodes[i].x as f64 - sx;
                let dy = state.nodes[i].y as f64 - sy;
                if dx * dx + dy * dy <= r2 {
                    reached = true;
                    break;
                }
            }
            if reached {
                let gain = p.collect_rate.min(state.sugar_remaining[j]);
                state.b_free += gain;
                state.collected_total += gain;
                state.sugar_remaining[j] -= gain;
            }
        }
    }

    // --- 2/3/4. 配分（space colonization）・伸長・分岐（tip id 昇順） ---
    struct TipPlan {
        idx: usize,
        weight: f64,
        clusters: Vec<(f64, f64, f64, f64)>,
    }
    let mut plans: Vec<TipPlan> = Vec::new();
    for &ti in &tips {
        let dirs = attractor_dirs(state, ti, p);
        if dirs.is_empty() {
            continue;
        }
        let clusters = cluster_dirs(&dirs, p.branch_angle_threshold);
        let weight: f64 = clusters.iter().map(|c| c.2).sum();
        if weight > 0.0 {
            plans.push(TipPlan { idx: ti, weight, clusters });
        }
    }

    let total_plan_w: f64 = plans.iter().map(|pl| pl.weight).sum();
    let budget = (p.growth_rate * state.b_free).max(0.0);
    if budget > 0.0 && total_plan_w > 0.0 {
        for pl in &plans {
            let ti = pl.idx;
            let share = budget * (pl.weight / total_plan_w);
            let cluster_total_w: f64 = pl.clusters.iter().map(|c| c.2).sum();
            if cluster_total_w <= 0.0 {
                continue;
            }
            let is_root = state.nodes[ti].parent.is_none();
            let cur_d = path_len(state, ti);
            let branching = pl.clusters.len() >= 2;
            let must_spawn = is_root || branching || cur_d >= p.max_path_len;

            for &(cdx, cdy, cw, cmin_dist) in &pl.clusters {
                let cshare_budget = share * (cw / cluster_total_w);
                if cshare_budget <= 0.0 {
                    continue;
                }
                let (fromx, fromy) = (state.nodes[ti].x as f64, state.nodes[ti].y as f64);
                // 目標セルの標高で実効コストを決める（標高が高いほどcost大＝ソフト忌避）。
                // まず単位方向先のセルでサンプルし、それを使って Δd を決める。
                let probe_x = fromx + cdx;
                let probe_y = fromy + cdy;
                let e_probe = match sample_land_e(world, probe_x, probe_y) {
                    Some(e) => e,
                    None => continue, // 海/範囲外へ向かう伸長は棄却（tip は動かさない）
                };
                let eff_k = p.k * (1.0 + p.c_elev * e_probe);
                if eff_k <= 0.0 {
                    continue;
                }
                // 暴走オーバーシュート防止: 1tickの伸長は速度上限・誘引点距離の両方でキャップする。
                // キャップにより実支出(cshare)は予算枠(cshare_budget)以下になり、余りは
                // b_free に残って次tickへ持ち越される（保存的）。
                let dd_uncapped = cshare_budget / eff_k;
                let dd = dd_uncapped.min(p.max_step_per_tick).min(cmin_dist);
                if dd <= 0.0 {
                    continue;
                }
                let cshare = dd * eff_k;
                let (tx2, ty2) = (fromx + cdx * dd, fromy + cdy * dd);
                if sample_land_e(world, tx2, ty2).is_none() {
                    continue; // 目標が海/範囲外なら棄却
                }

                let tax = (eff_k - p.k) * dd; // 標高割増分は consumed（構造には乗らない）
                if must_spawn {
                    let total_cost = cshare + p.c_branch;
                    if state.b_free < total_cost {
                        continue; // 予算不足なら棄却（ソフト・不変条件維持）
                    }
                    state.nodes.push(Node {
                        parent: Some(ti),
                        x: tx2 as f32,
                        y: ty2 as f32,
                    });
                    state.b_free -= total_cost;
                    state.consumed_total += tax + p.c_branch;
                } else {
                    if state.b_free < cshare {
                        continue;
                    }
                    state.nodes[ti].x = tx2 as f32;
                    state.nodes[ti].y = ty2 as f32;
                    state.b_free -= cshare;
                    state.consumed_total += tax;
                }
            }
        }
    }

    // --- 5. 退縮: 誘引を失った tip（plans に無い、根でない）は毎tick 微小に縮む ---
    for &ti in &tips {
        if state.nodes[ti].parent.is_none() {
            continue; // 根は伸縮対象外
        }
        if plans.iter().any(|pl| pl.idx == ti) {
            continue; // 今tick誘引ありは退縮しない
        }
        let cur_d = path_len(state, ti);
        if cur_d <= 0.0 {
            continue;
        }
        let dd = p.retreat_rate.min(cur_d);
        let par = state.nodes[ti].parent.unwrap();
        let (px, py) = (state.nodes[par].x as f64, state.nodes[par].y as f64);
        let (nx, ny) = (state.nodes[ti].x as f64, state.nodes[ti].y as f64);
        let (dxr, dyr) = (nx - px, ny - py);
        let dist = (dxr * dxr + dyr * dyr).sqrt().max(1.0e-9);
        let (ux, uy) = (dxr / dist, dyr / dist);
        let new_d = (cur_d - dd).max(0.0);
        state.nodes[ti].x = (px + ux * new_d) as f32;
        state.nodes[ti].y = (py + uy * new_d) as f32;
        // 保存的: 退縮分はそのまま b_free へ戻す（標高税なし＝純粋な巻き戻し）。
        state.b_free += p.k * dd;
    }

    // --- prune: 縮みきった葉（d<=eps）を除去し index を詰め直す（親参照を再マップ） ---
    compact_pruned(state, p.prune_eps);

    // --- 砂糖の枯渇自動削除（id昇順、決定論） ---
    remove_depleted_sugar(state);

    state.tick += 1;
}

/// d<=eps まで縮みきった葉ノードを取り除き、index を再採番する（親参照も付け替え）。
/// 決定的（走査は index=id 昇順固定）。
fn compact_pruned(state: &mut TreeState, eps: f64) {
    let n = state.nodes.len();
    let mut child_count = vec![0u32; n];
    for i in 0..n {
        if let Some(par) = state.nodes[i].parent {
            child_count[par] += 1;
        }
    }
    let mut remove = vec![false; n];
    let mut any = false;
    for i in 0..n {
        if state.nodes[i].parent.is_some() && child_count[i] == 0 {
            if path_len(state, i) <= eps {
                remove[i] = true;
                any = true;
            }
        }
    }
    if !any {
        return;
    }
    let mut new_index = vec![usize::MAX; n];
    let mut new_nodes: Vec<Node> = Vec::with_capacity(n);
    for i in 0..n {
        if remove[i] {
            continue;
        }
        new_index[i] = new_nodes.len();
        new_nodes.push(state.nodes[i].clone());
    }
    for node in new_nodes.iter_mut() {
        if let Some(par) = node.parent {
            node.parent = Some(new_index[par]);
        }
    }
    state.nodes = new_nodes;
}
