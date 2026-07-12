//! netphys-001: `netphys_step`（設計メモ §2）。決定論契約: 乱数は state.rng の単一 PRNG のみ・
//! ノード/前線/砂糖は id(index) 昇順で処理。Kirchhoff・Tero・prune は乱数不要で決定的。
//! 現行 Jones/tree モデルとは独立（並置・無関係、world.rs のみ読み取り共有）。

use crate::state::Op;
use crate::world::World;

use super::kirchhoff::{sample_e, solve as kirchhoff_solve};
use super::state::{apply_net_op, remove_depleted_sugar, NEdge, NNode, NetParams, NetState};

/// 対象セルが陸かつ範囲内なら標高を返す。海/範囲外は None（境界不変条件・棄却対象）。
fn sample_land_e(world: &World, x: f64, y: f64) -> Option<f64> {
    let (h, w) = (world.h, world.w);
    let fx = x.floor();
    let fy = y.floor();
    let inb = fx >= 0.0 && fx < w as f64 && fy >= 0.0 && fy < h as f64;
    if !inb {
        return None;
    }
    let idx = (fy as usize) * w + (fx as usize);
    if !world.land_mask[idx] {
        return None;
    }
    Some(world.e[idx] as f64)
}

/// フロントノード ti から見た誘引方向（単位ベクトル・重み付き合成、砂糖 id 昇順で走査＝決定的）。
fn attractor_dir(state: &NetState, ti: usize, p: &NetParams) -> Option<(f64, f64, f64)> {
    let (tx, ty) = (state.nodes[ti].x, state.nodes[ti].y);
    let mut order: Vec<usize> = (0..state.sugar_id.len()).collect();
    order.sort_by_key(|&j| state.sugar_id[j]);
    let (mut sx, mut sy, mut sw) = (0.0, 0.0, 0.0);
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
        sx += (dx / dist) * w;
        sy += (dy / dist) * w;
        sw += w;
    }
    if sw <= 0.0 {
        None
    } else {
        let n = (sx * sx + sy * sy).sqrt().max(1.0e-9);
        Some((sx / n, sy / n, sw))
    }
}

/// 距離2乗（一般グラフの座標間）。
#[inline]
fn dist2(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

/// 既存の辺 (a,b) が既に存在するか（無向・順不同）。
fn edge_exists(edges: &[NEdge], a: usize, b: usize) -> bool {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    edges.iter().any(|e| e.a == lo && e.b == hi)
}

/// Phase1（探索・毎tick）+ Phase2（anastomosis・探索中判定）。
/// 前線ノードを id 昇順で処理し、各ノードにつき state.rng を最大1回だけ引く。
fn phase1_search_and_anastomosis(state: &mut NetState, world: &World, p: &NetParams) {
    let mut order = state.frontier.clone();
    order.sort_unstable();
    order.dedup();

    let mut new_frontier: Vec<usize> = Vec::with_capacity(order.len());

    for ti in order {
        if ti >= state.nodes.len() {
            continue; // consolidation 等で消えた前線idは無視(安全弁)
        }
        let (fx, fy) = (state.nodes[ti].x, state.nodes[ti].y);

        // 探索方向: 誘引(重み付き) と ランダム を w_rand でブレンド。
        let attract = attractor_dir(state, ti, p);
        let raw_ang = state.rng.next_f64() * std::f64::consts::TAU;
        let (rdx, rdy) = (raw_ang.cos(), raw_ang.sin());

        let dir = match attract {
            Some((adx, ady, aw)) => {
                let bx = p.w_rand * rdx + aw * adx;
                let by = p.w_rand * rdy + aw * ady;
                let n = (bx * bx + by * by).sqrt();
                if n <= 1.0e-9 {
                    None
                } else {
                    Some((bx / n, by / n))
                }
            }
            None => {
                if p.w_rand > 0.0 {
                    Some((rdx, rdy))
                } else {
                    None
                }
            }
        };

        let (dx, dy) = match dir {
            Some(v) => v,
            None => {
                new_frontier.push(ti); // 探索方向なし: 前線として留まる（次tickに再試行）
                continue;
            }
        };

        if state.nodes.len() >= p.node_cap || state.edges.len() >= p.edge_cap {
            new_frontier.push(ti); // 有界性: cap 到達で成長停止（前線は留まる）
            continue;
        }

        let probe_x = fx + dx * p.search_step;
        let probe_y = fy + dy * p.search_step;
        let e_target = match sample_land_e(world, probe_x, probe_y) {
            Some(e) => e,
            None => {
                new_frontier.push(ti); // 海/範囲外への伸長は棄却（境界不変条件）
                continue;
            }
        };

        // anastomosis: probe 近傍(fusion_dist以内)の既存ノード(ti自身・既に隣接済み=直近の親含む、を除く)
        // があれば辺で接続してループを閉じる。直近の親を除外しないと、ti が生成された時点で
        // 親との距離は search_step しかないため、search_step*sqrt(2) など緩い角度の探索方向でも
        // 常に親が最近傍にヒットして自己融合（＝実質何もせず打ち止め）してしまい網が育たない。
        let fd2 = p.fusion_dist * p.fusion_dist;
        let mut nearest: Option<(usize, f64)> = None;
        for (nid, nd) in state.nodes.iter().enumerate() {
            if nid == ti || edge_exists(&state.edges, ti, nid) {
                continue;
            }
            let d2 = dist2(probe_x, probe_y, nd.x, nd.y);
            if d2 <= fd2 {
                if nearest.map(|(_, bd)| d2 < bd).unwrap_or(true) {
                    nearest = Some((nid, d2));
                }
            }
        }

        if let Some((nid, d2)) = nearest {
            let actual_len = d2.sqrt().max(1.0e-6);
            let structural = p.d0 * actual_len;
            let tax = structural * p.c_elev * e_target;
            let total_cost = structural + tax;
            if state.free_budget < total_cost {
                new_frontier.push(ti);
                continue;
            }
            state.free_budget -= total_cost;
            state.consumed_total += tax;
            let (a, b) = if ti <= nid { (ti, nid) } else { (nid, ti) };
            state.edges.push(NEdge { a, b, d: p.d0, l: actual_len });
            // このアームは既存網に融合したため打ち止め（次前線には含めない）。
            continue;
        }

        // 新規ノード生成（前線を先端へ進める）。
        let structural = p.d0 * p.search_step;
        let tax = structural * p.c_elev * e_target;
        let total_cost = structural + tax;
        if state.free_budget < total_cost {
            new_frontier.push(ti);
            continue;
        }
        state.free_budget -= total_cost;
        state.consumed_total += tax;
        let new_id = state.nodes.len();
        state.nodes.push(NNode { x: probe_x, y: probe_y });
        state.edges.push(NEdge { a: ti, b: new_id, d: p.d0, l: p.search_step });
        new_frontier.push(new_id);
    }

    new_frontier.sort_unstable();
    new_frontier.dedup();
    state.frontier = new_frontier;
}

/// 砂糖回収（id 昇順・全ノード走査）。
fn collect_sugar(state: &mut NetState, p: &NetParams) {
    let n0 = state.nodes.len();
    let m0 = state.sugar_id.len();
    if m0 == 0 || n0 == 0 {
        return;
    }
    let mut order: Vec<usize> = (0..m0).collect();
    order.sort_by_key(|&j| state.sugar_id[j]);
    let r2 = p.sugar_radius * p.sugar_radius;
    for j in order {
        if state.sugar_remaining[j] <= 0.0 {
            continue;
        }
        let (sx, sy) = (state.sugar_x[j], state.sugar_y[j]);
        let mut reached = false;
        for nd in &state.nodes {
            if dist2(nd.x, nd.y, sx, sy) <= r2 {
                reached = true;
                break;
            }
        }
        if reached {
            let gain = p.collect_rate.min(state.sugar_remaining[j]);
            state.free_budget += gain;
            state.collected_total += gain;
            state.sugar_remaining[j] -= gain;
        }
    }
}

/// consolidation（Phase3・周期 N tick ごと）。
fn phase3_consolidation(state: &mut NetState, world: &World, p: &NetParams) {
    let n = state.nodes.len();
    if n == 0 {
        return;
    }

    // 重心
    let (mut cx, mut cy) = (0.0, 0.0);
    for nd in &state.nodes {
        cx += nd.x;
        cy += nd.y;
    }
    cx /= n as f64;
    cy /= n as f64;

    // 次数（葉/末端優先で最外周を選ぶ）。
    let mut deg = vec![0u32; n];
    for e in &state.edges {
        deg[e.a] += 1;
        deg[e.b] += 1;
    }

    // 最外周候補: 葉(deg<=1)を重心からの距離降順・id昇順タイブレークで上位k、不足分は全ノードから補充。
    let mut leaves: Vec<usize> = (0..n).filter(|&i| deg[i] <= 1).collect();
    leaves.sort_by(|&a, &b| {
        let da = dist2(state.nodes[a].x, state.nodes[a].y, cx, cy);
        let db = dist2(state.nodes[b].x, state.nodes[b].y, cx, cy);
        db.partial_cmp(&da).unwrap().then(a.cmp(&b))
    });
    let mut frontier_candidates: Vec<usize> = leaves.into_iter().take(p.k_frontier).collect();
    if frontier_candidates.len() < p.k_frontier.min(n) {
        let mut all: Vec<usize> = (0..n).collect();
        all.sort_by(|&a, &b| {
            let da = dist2(state.nodes[a].x, state.nodes[a].y, cx, cy);
            let db = dist2(state.nodes[b].x, state.nodes[b].y, cx, cy);
            db.partial_cmp(&da).unwrap().then(a.cmp(&b))
        });
        for i in all {
            if frontier_candidates.len() >= p.k_frontier.min(n) {
                break;
            }
            if !frontier_candidates.contains(&i) {
                frontier_candidates.push(i);
            }
        }
    }
    frontier_candidates.sort_unstable();

    // 砂糖端子: 各アクティブ砂糖を tap 半径内の最近傍ノードへ対応付け（id昇順で決定的）。
    let mut sugar_order: Vec<usize> = (0..state.sugar_id.len()).collect();
    sugar_order.sort_by_key(|&j| state.sugar_id[j]);
    let mut sugar_terminals: Vec<usize> = Vec::new();
    for j in sugar_order {
        if state.sugar_remaining[j] <= 0.0 {
            continue;
        }
        let (sx, sy) = (state.sugar_x[j], state.sugar_y[j]);
        let mut best: Option<(usize, f64)> = None;
        for (nid, nd) in state.nodes.iter().enumerate() {
            let d2 = dist2(nd.x, nd.y, sx, sy);
            if d2 <= p.sugar_tap_radius * p.sugar_tap_radius {
                if best.map(|(_, bd)| d2 < bd).unwrap_or(true) {
                    best = Some((nid, d2));
                }
            }
        }
        if let Some((nid, _)) = best {
            sugar_terminals.push(nid);
        }
    }

    let mut terminals: Vec<usize> = frontier_candidates.clone();
    terminals.extend(sugar_terminals.iter().copied());
    terminals.sort_unstable();
    terminals.dedup();

    // Kirchhoff + Tero 適応（端子2つ以上のときのみ）。
    if terminals.len() >= 2 {
        let source = terminals[0];
        let sink = *terminals.last().unwrap();
        let flow = kirchhoff_solve(&state.nodes, &state.edges, world, p.net_alpha, source, sink);
        if flow.connected {
            // Tero 適応は質量保存的に行う: D の増分(=辺の構造質量の増分)は free_budget から
            // 差し引き、減分は free_budget へ戻す（担体A: 質量は free_budget + Σ D*L）。
            // 増分が現在の free_budget を超える場合は budget 律速でその分だけ増分を切り詰める
            // （ソフト・不変条件維持）。辺は決定的な既存順（挿入順＝seed由来で決定的）で処理。
            for (i, e) in state.edges.iter_mut().enumerate() {
                let q = flow.edge_currents[i];
                let old_mass = e.d * e.l;
                let target_d = ((1.0 - p.tero_decay) * e.d + p.tero_gain * q).max(0.0);
                let mut new_mass = target_d * e.l;
                let delta = new_mass - old_mass;
                if delta > 0.0 {
                    let spend = delta.min(state.free_budget.max(0.0));
                    state.free_budget -= spend;
                    new_mass = old_mass + spend;
                } else {
                    state.free_budget += -delta;
                }
                e.d = if e.l > 1.0e-12 { new_mass / e.l } else { 0.0 };
            }
        }
    }

    // prune: D<eps の辺を除去し、Σ(旧d*l)を free_budget へ保存的に返す。
    let mut kept_edges: Vec<NEdge> = Vec::with_capacity(state.edges.len());
    let mut returned = 0.0f64;
    for e in state.edges.drain(..) {
        if e.d < p.prune_eps {
            returned += e.d * e.l;
        } else {
            kept_edges.push(e);
        }
    }
    state.edges = kept_edges;
    state.free_budget += returned;

    // 孤立ノード(deg0)の prune（根=ノード0は常に保護。frontier_candidates/端子も保護し、次前線を保つ）。
    let mut deg2 = vec![0u32; state.nodes.len()];
    for e in &state.edges {
        deg2[e.a] += 1;
        deg2[e.b] += 1;
    }
    let protect: std::collections::HashSet<usize> =
        terminals.iter().copied().chain(std::iter::once(0usize)).collect();
    let keep: Vec<bool> = (0..state.nodes.len())
        .map(|i| deg2[i] > 0 || protect.contains(&i))
        .collect();

    if keep.iter().any(|&k| !k) {
        let mut new_index = vec![usize::MAX; state.nodes.len()];
        let mut new_nodes: Vec<NNode> = Vec::new();
        for i in 0..state.nodes.len() {
            if keep[i] {
                new_index[i] = new_nodes.len();
                new_nodes.push(state.nodes[i].clone());
            }
        }
        for e in state.edges.iter_mut() {
            e.a = new_index[e.a];
            e.b = new_index[e.b];
        }
        frontier_candidates.retain(|&i| new_index[i] != usize::MAX);
        for i in frontier_candidates.iter_mut() {
            *i = new_index[*i];
        }
        state.nodes = new_nodes;
    }

    frontier_candidates.sort_unstable();
    frontier_candidates.dedup();
    if frontier_candidates.is_empty() {
        // フォールバック: 常に保護されているノード0(根)を新前線にする。
        frontier_candidates.push(0);
    }
    state.frontier = frontier_candidates;
    let _ = sample_e; // (kirchhoff モジュールの標高サンプラを共有していることの明示・未使用抑制)
}

/// 1tick 進める（in-place）。決定論・ノード/前線/砂糖 id 昇順・単一 PRNG。
pub fn netphys_step(state: &mut NetState, world: &World, p: &NetParams, ops: &[Op]) {
    for op in ops {
        apply_net_op(state, op);
    }

    phase1_search_and_anastomosis(state, world, p);
    // Phase1 で全アームが同tick内に既存網へ融合(anastomosis)して打ち止めになると前線が空になり、
    // 次の consolidation(周期N tick後)まで探索が完全停止してしまう。ノード0(根)は常に存在し
    // consolidation で保護されているため、空になったら即座にそこへフォールバックして毎tick探索を保つ。
    if state.frontier.is_empty() && !state.nodes.is_empty() {
        state.frontier.push(0);
    }
    collect_sugar(state, p);
    remove_depleted_sugar(state);

    state.tick += 1;

    if p.period_n > 0 && state.tick % p.period_n == 0 {
        phase3_consolidation(state, world, p);
    }
}
