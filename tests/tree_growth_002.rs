//! tree-growth-002: 砂糖なしでもランダム伸長（探索）＋近くの砂糖へ誘引、の受け入れテスト。
//! 現行 Jones モデルの受け入れテスト・tree_growth_001.rs とは無関係・独立（新規ファイル）。
//! 対象は src/tree/{state,step}.rs に追加した `w_rand`（既定 0.0 = 探索オフ・現行挙動）。
//!
//! S9（正準9シード）の部分集合 [1, 42, 1337] の中央値で判定する（規約 §4 の集計法を踏襲）。

use nenkin_garden::params::Params;
use nenkin_garden::state::{Op, ScriptEntry};
use nenkin_garden::tree::{
    initial_tree_state, run_tree_headless, total_volume, tree_step, TreeParams, TreeState,
};
use nenkin_garden::world::{make_synthetic_archipelago, World};

/// task 指定のシード部分集合。
const SEEDS: [u64; 3] = [1, 42, 1337];

/// テストで使う探索パラメータ（実測で選定。§実測根拠は末尾コメント参照）。
/// - w_rand: 探索がsole claimantの場合、その大きさ自体は伸長量に効かない（他tipとの重み競合にのみ影響）
///   ので、宿主の誘引と混在しても誘引が支配しやすい程度の値として 0.3 を採用。
/// - explore_persistence: TreeParams::default() と同じ 0.45。0.5 を超えると、tip が海方向で
///   動けなくなった際に方向が固定され続け、ブレンド角度域が狭まって**恒久的にデッドロック**しうる
///   ことを実測で確認したため、0.5 以下（安全マージンを見て 0.45）を採用した。
const W_RAND: f64 = 0.3;
const EXPLORE_PERSISTENCE: f64 = 0.45;

fn median3(mut xs: Vec<f64>) -> f64 {
    assert_eq!(xs.len(), 3);
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs[1]
}

/// 提案座標が海/範囲外なら最も近い陸セル中心へ決定的にスナップする（tree_growth_001.rs と同趣旨）。
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

/// 構造として蓄積された体積（b_free を除く Σk*d_i 部分）。
fn structural_len(s: &TreeState, k: f64) -> f64 {
    total_volume(s, k) - s.b_free
}

/// world・TreeParams（w_rand 指定）・ホーム座標。world は tree_growth_001.rs と同じ合成列島。
fn world_and_params(w_rand: f64) -> (World, TreeParams, f64, f64) {
    let wp = Params::default();
    let world = make_synthetic_archipelago(&wp);
    let mut tp = TreeParams::default();
    tp.w_rand = w_rand;
    tp.explore_persistence = EXPLORE_PERSISTENCE;
    let (hx, hy) = world.default_home(tp.e_lo);
    (world, tp, hx, hy)
}

// ---------- ① 砂糖なし探索（新挙動） ----------

#[test]
fn accept1_explores_without_sugar_more_than_baseline() {
    let (world, tp_explore, _hx, _hy) = world_and_params(W_RAND);
    let (_, tp_baseline, _, _) = world_and_params(0.0);
    let ticks = 150u64;

    let mut nodes_explore = Vec::new();
    let mut len_explore = Vec::new();
    let mut nodes_baseline = Vec::new();
    let mut len_baseline = Vec::new();
    for &seed in &SEEDS {
        let re = run_tree_headless(seed, &[], ticks, &tp_explore, &world);
        nodes_explore.push(re.final_state.n_nodes() as f64);
        len_explore.push(structural_len(&re.final_state, tp_explore.k));

        let rb = run_tree_headless(seed, &[], ticks, &tp_baseline, &world);
        nodes_baseline.push(rb.final_state.n_nodes() as f64);
        len_baseline.push(structural_len(&rb.final_state, tp_baseline.k));
    }

    let mne = median3(nodes_explore.clone());
    let mle = median3(len_explore.clone());
    let mnb = median3(nodes_baseline.clone());
    let mlb = median3(len_baseline.clone());

    // 砂糖なしの現行(w_rand=0)は伸びない（根のみ）ことを前提として確認。
    assert!(mnb <= 1.0, "baseline(w_rand=0)で砂糖なしなのに伸びている: nodes={:?}", nodes_baseline);
    assert!(mlb <= 1.0e-9, "baseline(w_rand=0)で砂糖なしなのに構造長が正: len={:?}", len_baseline);

    // 探索(w_rand>0)は初期(根のみ)より増える。
    assert!(mne > 1.0, "探索でノード数が増えていない: nodes={:?}", nodes_explore);
    assert!(mle > 0.0, "探索で構造長が増えていない: len={:?}", len_explore);

    // baseline より有意に大きい。
    assert!(
        mne > mnb + 1.0,
        "探索のノード数中央値がbaselineより有意に大きくない: explore={:?} baseline={:?}",
        nodes_explore,
        nodes_baseline
    );
    assert!(
        mle > mlb + 1.0,
        "探索の構造長中央値がbaselineより有意に大きくない: explore={:?} baseline={:?}",
        len_explore,
        len_baseline
    );
}

// ---------- ② 予算で頭打ち（保存則） ----------

#[test]
fn accept2_explore_bounded_by_budget_and_conserves() {
    let (world, tp, _hx, _hy) = world_and_params(W_RAND);

    // 頭打ち: T を大きくしても構造長が initial_budget 由来の上限内でプラトーする。
    let mut len_mid = Vec::new();
    let mut len_large = Vec::new();
    for &seed in &SEEDS {
        let r_mid = run_tree_headless(seed, &[], 400, &tp, &world);
        len_mid.push(structural_len(&r_mid.final_state, tp.k));
        let r_large = run_tree_headless(seed, &[], 900, &tp, &world);
        len_large.push(structural_len(&r_large.final_state, tp.k));
    }
    let m_mid = median3(len_mid.clone());
    let m_large = median3(len_large.clone());
    assert!(
        m_large <= m_mid * 1.05 + 1.0e-6,
        "T を伸ばしても構造長が増え続けている(プラトーしない): mid(T=400)={:?} large(T=900)={:?}",
        len_mid,
        len_large
    );
    // 予算 initial_budget の範囲内に収まっている（構造長は b_free 分を除いた保存量なので上限は初期予算）。
    assert!(
        m_large <= tp.initial_budget + 1.0e-6,
        "構造長が initial_budget を超過: m_large={} budget={}",
        m_large,
        tp.initial_budget
    );

    // 保存則: 砂糖なし探索でも全 tick で collected-consumed==total_volume・非負・b_free超過なし。
    let eps = 1.0e-6;
    for &seed in &SEEDS {
        let mut state = initial_tree_state(seed, &world, &tp);
        for _ in 0..200u64 {
            tree_step(&mut state, &world, &tp, &[]);
            assert!(state.b_free >= -eps, "seed {seed}: b_free 負 (tick={})", state.tick);
            let tv = total_volume(&state, tp.k);
            assert!(tv >= -eps, "seed {seed}: total_volume 負 (tick={})", state.tick);
            let rhs = state.collected_total - state.consumed_total;
            assert!(
                (tv - rhs).abs() <= 1.0e-3,
                "seed {seed} tick {}: 保存則破れ total_volume={} collected-consumed={}",
                state.tick,
                tv,
                rhs
            );
            assert!(
                state.b_free <= tv + eps,
                "seed {seed} tick {}: b_free が total_volume を超過",
                state.tick
            );
        }
    }
}

// ---------- ③ 近くの砂糖へ誘引が支配 ----------

#[test]
fn accept3_still_reaches_nearby_sugar_with_explore() {
    let (world, tp, hx, hy) = world_and_params(W_RAND);
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
        "w_rand>0でも距離Dの砂糖へ中央値で到達できていない: dists={:?} median={} radius={}",
        dists,
        m,
        tp.sugar_radius
    );
}

// ---------- ④ 決定性 ----------

#[test]
fn accept4_deterministic_with_explore() {
    let (world, tp, hx, hy) = world_and_params(W_RAND);
    let (sx, sy) = nearest_land(&world, hx + 10.0, hy);
    let script = vec![ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx, y: sy, strength: 200.0 } }];
    for &seed in &SEEDS {
        let r1 = run_tree_headless(seed, &script, 120, &tp, &world);
        let r2 = run_tree_headless(seed, &script, 120, &tp, &world);
        assert_eq!(
            r1.final_state_hash, r2.final_state_hash,
            "seed {seed}: 探索ありでも同一(seed,script,ticks,params)でhashが再現しない"
        );
    }
    // 砂糖なし（探索のみ）でも再現すること。
    for &seed in &SEEDS {
        let r1 = run_tree_headless(seed, &[], 120, &tp, &world);
        let r2 = run_tree_headless(seed, &[], 120, &tp, &world);
        assert_eq!(
            r1.final_state_hash, r2.final_state_hash,
            "seed {seed}: 砂糖なし探索でhashが再現しない"
        );
    }
}

// ---------- ⑤ 不変条件 ----------

#[test]
fn accept5_invariants_with_explore() {
    let (world, tp, hx, hy) = world_and_params(W_RAND);
    let (sx, sy) = nearest_land(&world, hx + 12.0, hy);
    let script = vec![ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx, y: sy, strength: 300.0 } }];

    for &seed in &SEEDS {
        let r = run_tree_headless(seed, &script, 150, &tp, &world);
        let s = &r.final_state;
        let n = s.nodes.len();

        // 有限性
        assert!(s.b_free.is_finite(), "seed {seed}: b_free 非有限");
        for (i, node) in s.nodes.iter().enumerate() {
            assert!(node.x.is_finite() && node.y.is_finite(), "seed {seed}: node {i} 座標非有限");
        }

        // 境界（全ノードが陸・範囲内）
        for (i, node) in s.nodes.iter().enumerate() {
            let (x, y) = (node.x as f64, node.y as f64);
            assert!(
                x >= 0.0 && x < world.w as f64 && y >= 0.0 && y < world.h as f64,
                "seed {seed}: node {i} 範囲外"
            );
            let cix = (x.floor() as usize).min(world.w - 1);
            let ciy = (y.floor() as usize).min(world.h - 1);
            assert!(world.land_mask[ciy * world.w + cix], "seed {seed}: node {i} 海上");
        }

        // 木性: 根1・親高々1・閉路なし連結
        let roots: Vec<usize> = (0..n).filter(|&i| s.nodes[i].parent.is_none()).collect();
        assert_eq!(roots.len(), 1, "seed {seed}: 根が1つでない roots={:?}", roots);
        let root = roots[0];
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

    // ソフト標高忌避（探索でも）: tree_growth_001 と同じ手法。予算を絞り、home からほぼ同距離の
    // 高標高/低標高ターゲットへの構造進捗を比較する（w_rand>0 を混ぜても抑制が保たれること）。
    let mut tp_scarce = tp;
    tp_scarce.initial_budget = 6.0;
    let mut high_lens = Vec::new();
    let mut low_lens = Vec::new();
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
        "探索ありでもソフト標高忌避が確認できない: high_median={mh} low_median={ml} (high={:?} low={:?})",
        high_lens,
        low_lens
    );
}

/// home からほぼ距離 d（許容 tol）にある陸セルのうち、標高最大/最小のものを1つずつ探す
/// （tree_growth_001.rs の同名関数と同趣旨）。
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

// ---------- ⑥ 既定オフ（最重要・リグレッション） ----------

#[test]
fn accept6_default_is_explore_off() {
    // 既定は探索オフ(w_rand=0.0)＝現行挙動。この既定値自体を守る（誤って書き換えられていないか）。
    assert_eq!(
        TreeParams::default().w_rand,
        0.0,
        "既定の w_rand が 0.0 でない（探索が既定でオンになっている＝規約違反）"
    );
    // 実際の hash バイト不変性・全6件緑は tests/tree_growth_001.rs（本タスクでは編集しない・
    // 既存のまま）で検証済み（本ファイルは w_rand>0 の新挙動のみを担当する）。
}
