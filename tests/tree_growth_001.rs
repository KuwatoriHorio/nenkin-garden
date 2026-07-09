//! tree-growth-001: α成長木モデル（space colonization・全体予算B）の受け入れテスト。
//! 現行 Jones モデルの受け入れテスト（test_harness_001.rs 等）とは無関係・独立。
//! 新モデルは src/tree/ に実装（TreeState/TreeParams/tree_step/tree_state_hash）。
//!
//! S9（正準9シード）の部分集合 [1, 42, 1337] の中央値で判定する（規約 §4 の集計法を踏襲）。

use nenkin_garden::params::Params;
use nenkin_garden::state::{Op, ScriptEntry};
use nenkin_garden::tree::{run_tree_headless, total_volume, TreeParams, TreeState};
use nenkin_garden::world::{make_synthetic_archipelago, World};

/// task 指定のシード部分集合。
const SEEDS: [u64; 3] = [1, 42, 1337];

fn median3(mut xs: Vec<f64>) -> f64 {
    assert_eq!(xs.len(), 3);
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs[1]
}

/// 提案座標が海/範囲外なら最も近い陸セル中心へ決定的にスナップする（境界不変条件を壊さない
/// テスト用シナリオ構築のためのヘルパ。run_tree.rs の同名ロジックと同趣旨）。
fn nearest_land(world: &World, x: f64, y: f64) -> (f64, f64) {
    let fx = x.floor();
    let fy = y.floor();
    let inb = fx >= 0.0 && fx < world.w as f64 && fy >= 0.0 && fy < world.h as f64;
    if inb && world.land_mask[(fy as usize) * world.w + (fx as usize)] {
        return (x, y);
    }
    let mut best: Option<(f64, usize)> = None;
    for cy in 0..world.h {
        for cx in 0..world.w {
            let i = cy * world.w + cx;
            if !world.land_mask[i] {
                continue;
            }
            let cxf = cx as f64 + 0.5;
            let cyf = cy as f64 + 0.5;
            let d = (cxf - x).powi(2) + (cyf - y).powi(2);
            if best.map(|(bd, _)| d < bd).unwrap_or(true) {
                best = Some((d, i));
            }
        }
    }
    let i = best.expect("world has no land cells").1;
    ((i % world.w) as f64 + 0.5, (i / world.w) as f64 + 0.5)
}

/// 共有 world・TreeParams・ホーム座標。world は現行 Params（h/w のみ利用・読み取り専用）から
/// 生成する既存の合成列島を流用する（現行モデルへの影響なし）。
fn world_and_params() -> (World, TreeParams, f64, f64) {
    let wp = Params::default();
    let world = make_synthetic_archipelago(&wp);
    let tp = TreeParams::default();
    let (hx, hy) = world.default_home(tp.e_lo);
    (world, tp, hx, hy)
}

fn nearest_node_dist(s: &TreeState, sx: f64, sy: f64) -> f64 {
    let mut best = f64::INFINITY;
    for n in &s.nodes {
        let dx = n.x as f64 - sx;
        let dy = n.y as f64 - sy;
        let d = (dx * dx + dy * dy).sqrt();
        if d < best {
            best = d;
        }
    }
    best
}

/// 木構造で「分岐ノード」= 子を2つ以上持つノードの数。
fn branch_node_count(s: &TreeState) -> usize {
    let n = s.nodes.len();
    let mut child_count = vec![0u32; n];
    for i in 0..n {
        if let Some(par) = s.nodes[i].parent {
            child_count[par] += 1;
        }
    }
    child_count.iter().filter(|&&c| c >= 2).count()
}

/// 構造として蓄積された体積（b_free を除く Σk*d_i 部分）。
fn structural_len(s: &TreeState, k: f64) -> f64 {
    total_volume(s, k) - s.b_free
}

// ---------- ① 到達（伸び） ----------

#[test]
fn accept1_reaches_sugar_at_distance_d() {
    let (world, tp, hx, hy) = world_and_params();
    let (sx, sy) = nearest_land(&world, hx + 12.0, hy);
    let script = vec![ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx, y: sy, strength: 300.0 } }];
    let ticks = 80;

    let mut dists = Vec::new();
    for &seed in &SEEDS {
        let r = run_tree_headless(seed, &script, ticks, &tp, &world);
        dists.push(nearest_node_dist(&r.final_state, sx, sy));
    }
    let m = median3(dists.clone());
    assert!(
        m <= tp.sugar_radius,
        "距離Dの砂糖へ中央値で到達できていない: dists={:?} median={} radius={}",
        dists,
        m,
        tp.sugar_radius
    );
}

// ---------- ② 保存則 ----------

#[test]
fn accept2_conservation_holds() {
    let (world, tp, hx, hy) = world_and_params();
    let (sx, sy) = nearest_land(&world, hx + 12.0, hy);
    let (sx2, sy2) = nearest_land(&world, hx - 8.0, hy + 8.0);
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx, y: sy, strength: 300.0 } },
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx2, y: sy2, strength: 250.0 } },
    ];
    let eps = 1.0e-6;
    for &seed in &SEEDS {
        let r = run_tree_headless(seed, &script, 80, &tp, &world);
        let s = &r.final_state;
        assert!(s.b_free >= -eps, "seed {seed}: b_free 負");
        let tv = total_volume(s, tp.k);
        assert!(tv >= -eps, "seed {seed}: total_volume 負");
        let rhs = s.collected_total - s.consumed_total;
        assert!(
            (tv - rhs).abs() <= 1.0e-3,
            "seed {seed}: 保存則破れ total_volume={tv} collected-consumed={rhs}"
        );
        // b_free を超える配分をしていないこと（構造分は非負なので b_free <= total_volume は自明だが明示）
        assert!(s.b_free <= tv + eps, "seed {seed}: b_free が total_volume を超過");
    }
}

// ---------- ③ 分岐 ----------

#[test]
fn accept3_branches_and_reaches_both() {
    let (world, tp, hx, hy) = world_and_params();
    let (sx, sy) = nearest_land(&world, hx + 12.0, hy);
    let (sx2, sy2) = nearest_land(&world, hx - 8.0, hy + 8.0);
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx, y: sy, strength: 300.0 } },
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx2, y: sy2, strength: 250.0 } },
    ];
    let mut branch_counts = Vec::new();
    let mut d1s = Vec::new();
    let mut d2s = Vec::new();
    for &seed in &SEEDS {
        let r = run_tree_headless(seed, &script, 80, &tp, &world);
        let s = &r.final_state;
        branch_counts.push(branch_node_count(s) as f64);
        d1s.push(nearest_node_dist(s, sx, sy));
        d2s.push(nearest_node_dist(s, sx2, sy2));
    }
    assert!(
        median3(branch_counts.clone()) >= 1.0,
        "分岐ノード数の中央値が1未満: {:?}",
        branch_counts
    );
    assert!(median3(d1s.clone()) <= tp.sugar_radius, "砂糖1中央値未到達: {:?}", d1s);
    assert!(median3(d2s.clone()) <= tp.sugar_radius, "砂糖2中央値未到達: {:?}", d2s);
}

// ---------- ④ 退縮 ----------

#[test]
fn accept4_retracts_after_sugar_removed() {
    let (world, tp, hx, hy) = world_and_params();
    let (sx, sy) = nearest_land(&world, hx + 10.0, hy);
    let t1 = 60u64;
    let t2 = 60u64;
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx, y: sy, strength: 300.0 } },
        ScriptEntry { tick: t1, op: Op::RemoveSugar { id: 0 } },
    ];
    let mut ratios = Vec::new();
    for &seed in &SEEDS {
        let before = run_tree_headless(seed, &script, t1, &tp, &world);
        let after = run_tree_headless(seed, &script, t1 + t2, &tp, &world);
        let len_before = structural_len(&before.final_state, tp.k);
        let len_after = structural_len(&after.final_state, tp.k);
        assert!(len_before > 0.0, "seed {seed}: 除去前に構造が育っていない");
        ratios.push(len_after / len_before);
    }
    let m = median3(ratios.clone());
    assert!(
        m <= 0.6,
        "退縮が有意でない: len_after/len_before の中央値 {} (ratios={:?})",
        m,
        ratios
    );
}

// ---------- ⑤ 不変条件 ----------

#[test]
fn accept5_invariants() {
    let (world, tp, hx, hy) = world_and_params();
    let (sx, sy) = nearest_land(&world, hx + 12.0, hy);
    let (sx2, sy2) = nearest_land(&world, hx - 8.0, hy + 8.0);
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx, y: sy, strength: 300.0 } },
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx2, y: sy2, strength: 250.0 } },
    ];
    for &seed in &SEEDS {
        let r = run_tree_headless(seed, &script, 80, &tp, &world);
        let s = &r.final_state;

        // 有限性
        assert!(s.b_free.is_finite(), "seed {seed}: b_free 非有限");
        for (i, n) in s.nodes.iter().enumerate() {
            assert!(n.x.is_finite() && n.y.is_finite(), "seed {seed}: node {i} 座標非有限");
        }
        assert!(s.collected_total.is_finite() && s.consumed_total.is_finite());

        // 境界（全ノードが陸・範囲内）
        for (i, n) in s.nodes.iter().enumerate() {
            let (x, y) = (n.x as f64, n.y as f64);
            assert!(
                x >= 0.0 && x < world.w as f64 && y >= 0.0 && y < world.h as f64,
                "seed {seed}: node {i} 範囲外"
            );
            let cix = (x.floor() as usize).min(world.w - 1);
            let ciy = (y.floor() as usize).min(world.h - 1);
            assert!(world.land_mask[ciy * world.w + cix], "seed {seed}: node {i} 海上");
        }

        // 再現性
        let r2 = run_tree_headless(seed, &script, 80, &tp, &world);
        assert_eq!(r.final_state_hash, r2.final_state_hash, "seed {seed}: hash 再現性違反");
    }

    // ソフト標高忌避: 同距離(D=14)・高標高方向 vs 低標高方向で、予算が律速する条件下
    // （初期予算を絞り、標高コストが支配的になる regime）で構造進捗を比較する。
    // 高標高方向は低標高方向より有意に伸長が抑制される（構造長が小さい）ことを見る。
    let mut tp_scarce = tp;
    tp_scarce.initial_budget = 6.0;
    let mut high_lens = Vec::new();
    let mut low_lens = Vec::new();
    // world 内で home からほぼ同距離(±1)にある「高標高陸セル」「低標高陸セル」を探す。
    let (high_xy, low_xy) = find_high_low_targets(&world, hx, hy, 14.0, 2.0);
    for &seed in &SEEDS {
        let script_high =
            vec![ScriptEntry { tick: 0, op: Op::PlaceSugar { x: high_xy.0, y: high_xy.1, strength: 200.0 } }];
        let script_low =
            vec![ScriptEntry { tick: 0, op: Op::PlaceSugar { x: low_xy.0, y: low_xy.1, strength: 200.0 } }];
        let rh = run_tree_headless(seed, &script_high, 15, &tp_scarce, &world);
        let rl = run_tree_headless(seed, &script_low, 15, &tp_scarce, &world);
        high_lens.push(structural_len(&rh.final_state, tp_scarce.k));
        low_lens.push(structural_len(&rl.final_state, tp_scarce.k));
    }
    let mh = median3(high_lens.clone());
    let ml = median3(low_lens.clone());
    assert!(
        mh < ml * 0.95,
        "ソフト標高忌避が確認できない: high_median={mh} low_median={ml} (high={:?} low={:?})",
        high_lens,
        low_lens
    );
}

/// home からほぼ距離 d（許容 tol）にある陸セルのうち、標高最大/最小のものを1つずつ探す。
fn find_high_low_targets(
    world: &World,
    hx: f64,
    hy: f64,
    d: f64,
    tol: f64,
) -> ((f64, f64), (f64, f64)) {
    let mut best_high: Option<(f64, usize)> = None;
    let mut best_low: Option<(f64, usize)> = None;
    for cy in 0..world.h {
        for cx in 0..world.w {
            let i = cy * world.w + cx;
            if !world.land_mask[i] {
                continue;
            }
            let cxf = cx as f64 + 0.5;
            let cyf = cy as f64 + 0.5;
            let dist = ((cxf - hx).powi(2) + (cyf - hy).powi(2)).sqrt();
            if (dist - d).abs() > tol {
                continue;
            }
            let e = world.e[i] as f64;
            if best_high.map(|(be, _)| e > be).unwrap_or(true) {
                best_high = Some((e, i));
            }
            if best_low.map(|(be, _)| e < be).unwrap_or(true) {
                best_low = Some((e, i));
            }
        }
    }
    let (_, hi) = best_high.expect("高標高候補が見つからない（world固定なので通常発生しない）");
    let (_, li) = best_low.expect("低標高候補が見つからない（world固定なので通常発生しない）");
    let high_xy = ((hi % world.w) as f64 + 0.5, (hi / world.w) as f64 + 0.5);
    let low_xy = ((li % world.w) as f64 + 0.5, (li / world.w) as f64 + 0.5);
    (high_xy, low_xy)
}

// ---------- ⑥ 木性 ----------

#[test]
fn accept6_tree_property_no_cycles_connected() {
    let (world, tp, hx, hy) = world_and_params();
    let (sx, sy) = nearest_land(&world, hx + 12.0, hy);
    let (sx2, sy2) = nearest_land(&world, hx - 8.0, hy + 8.0);
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx, y: sy, strength: 300.0 } },
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx2, y: sy2, strength: 250.0 } },
    ];
    for &seed in &SEEDS {
        let r = run_tree_headless(seed, &script, 80, &tp, &world);
        let s = &r.final_state;
        let n = s.nodes.len();
        assert!(n >= 1, "seed {seed}: ノードが無い");

        // 根はちょうど1つ（parent=None）。
        let roots: Vec<usize> = (0..n).filter(|&i| s.nodes[i].parent.is_none()).collect();
        assert_eq!(roots.len(), 1, "seed {seed}: 根が1つでない roots={:?}", roots);
        let root = roots[0];

        // 根以外は親をちょうど1つ持つ（Option<usize> の型自体が「高々1つ」を保証）。
        for i in 0..n {
            if i == root {
                continue;
            }
            let par = s.nodes[i].parent.expect("根以外は親を持つ");
            assert!(par < n, "seed {seed}: node {i} の親indexが範囲外");
        }

        // 閉路なし・根から全ノードへ到達可能（親を辿ると必ず根で停止する）。
        for i in 0..n {
            let mut cur = i;
            let mut steps = 0usize;
            loop {
                match s.nodes[cur].parent {
                    None => {
                        assert_eq!(cur, root, "seed {seed}: node {i} が別の根に到達");
                        break;
                    }
                    Some(par) => {
                        cur = par;
                        steps += 1;
                        assert!(steps <= n, "seed {seed}: node {i} から親を辿るループが閉路（木性違反）");
                    }
                }
            }
        }
    }
}
