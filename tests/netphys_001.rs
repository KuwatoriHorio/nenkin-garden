//! netphys-001: 網 Physarum（担体A・Tero純, Stage 1）の受け入れテスト。
//! Jones でも tree でもない第3の独立モデル。現行 Jones/tree の受け入れテスト・ゴールデンとは
//! 無関係・独立（新モデルは src/netphys/ に実装: NetState/NetParams/netphys_step/netphys_state_hash）。
//!
//! Stage 1（本タスクの合否対象）= 受け入れ①②⑤⑥のみ。③前進波移動・④効率改善は netphys-002 へ
//! 繰り延べ（本ファイルでは検証しない）。
//!
//! S9（正準9シード）の部分集合 [1, 42, 1337] の中央値で判定する（規約 §4 の集計法を踏襲）。

use nenkin_garden::netphys::{
    initial_net_state, netphys_kirchhoff_solve, netphys_step, run_netphys_headless, total_mass,
    NetParams, NetState,
};
use nenkin_garden::params::Params;
use nenkin_garden::state::{Op, ScriptEntry};
use nenkin_garden::world::{make_synthetic_archipelago, World};

const SEEDS: [u64; 3] = [1, 42, 1337];

fn median3(mut xs: Vec<f64>) -> f64 {
    assert_eq!(xs.len(), 3);
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs[1]
}

/// 提案座標が海/範囲外なら最も近い陸セル中心へ決定的にスナップする（tree_growth_001 と同趣旨）。
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

fn world_and_params() -> (World, NetParams, f64, f64) {
    let wp = Params::default();
    let world = make_synthetic_archipelago(&wp);
    let np = NetParams::default();
    let (hx, hy) = world.default_home(np.e_lo);
    (world, np, hx, hy)
}

/// 連結成分ごとに「辺数 > ノード数-1」（＝冗長度>1＝ループを持つ）かどうかを判定し、
/// 少なくとも1成分がループを持つなら true。
fn has_loop_component(s: &NetState) -> bool {
    let n = s.nodes.len();
    if n == 0 {
        return false;
    }
    let mut uf: Vec<usize> = (0..n).collect();
    fn find(uf: &mut [usize], a: usize) -> usize {
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
    fn union(uf: &mut [usize], a: usize, b: usize) {
        let (ra, rb) = (find(uf, a), find(uf, b));
        if ra != rb {
            if ra < rb {
                uf[rb] = ra;
            } else {
                uf[ra] = rb;
            }
        }
    }
    for e in &s.edges {
        union(&mut uf, e.a, e.b);
    }
    let mut node_count: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut edge_count: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for i in 0..n {
        let r = find(&mut uf, i);
        *node_count.entry(r).or_insert(0) += 1;
    }
    for e in &s.edges {
        let r = find(&mut uf, e.a);
        *edge_count.entry(r).or_insert(0) += 1;
    }
    for (r, nc) in &node_count {
        let ec = edge_count.get(r).copied().unwrap_or(0);
        if ec > nc.saturating_sub(1) {
            return true;
        }
    }
    false
}

fn nearest_node(s: &NetState, x: f64, y: f64) -> Option<(usize, f64)> {
    let mut best: Option<(usize, f64)> = None;
    for (i, nd) in s.nodes.iter().enumerate() {
        let d = ((nd.x - x).powi(2) + (nd.y - y).powi(2)).sqrt();
        if best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((i, d));
        }
    }
    best
}

// ---------- ① 網化（ループ形成） ----------

#[test]
fn accept1_network_has_loop() {
    let (world, np, hx, hy) = world_and_params();
    // フロントが自然に扇状に広がり衝突する配置: 砂糖を複数置いて複数方向へ探索を誘発する。
    let (s1x, s1y) = nearest_land(&world, hx + 12.0, hy);
    let (s2x, s2y) = nearest_land(&world, hx - 8.0, hy + 8.0);
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: s1x, y: s1y, strength: 400.0 } },
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: s2x, y: s2y, strength: 300.0 } },
    ];
    let ticks = 400;

    let mut has_loop = Vec::new();
    for &seed in &SEEDS {
        let r = run_netphys_headless(seed, &script, ticks, &np, &world);
        has_loop.push(if has_loop_component(&r.final_state) { 1.0 } else { 0.0 });
    }
    let m = median3(has_loop.clone());
    assert!(
        m >= 1.0,
        "少なくとも1成分がループを持つことが中央値で確認できない: has_loop={:?}",
        has_loop
    );
}

// ---------- ② 餌を結ぶ ----------

#[test]
fn accept2_sugar_connected_after_consolidation() {
    let (world, np, hx, hy) = world_and_params();
    let (s1x, s1y) = nearest_land(&world, hx + 12.0, hy);
    let (s2x, s2y) = nearest_land(&world, hx - 8.0, hy + 8.0);
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: s1x, y: s1y, strength: 400.0 } },
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: s2x, y: s2y, strength: 300.0 } },
    ];
    let ticks = 400;

    let mut connected_flags = Vec::new();
    let mut conductances = Vec::new();
    for &seed in &SEEDS {
        let r = run_netphys_headless(seed, &script, ticks, &np, &world);
        let s = &r.final_state;
        let n1 = nearest_node(s, s1x, s1y);
        let n2 = nearest_node(s, s2x, s2y);
        match (n1, n2) {
            (Some((i1, _)), Some((i2, _))) if i1 != i2 => {
                let flow = netphys_kirchhoff_solve(&s.nodes, &s.edges, &world, np.net_alpha, i1, i2);
                connected_flags.push(if flow.connected { 1.0 } else { 0.0 });
                conductances.push(if flow.connected { flow.total_conductance } else { 0.0 });
            }
            _ => {
                connected_flags.push(0.0);
                conductances.push(0.0);
            }
        }
    }
    let m = median3(connected_flags.clone());
    assert!(
        m >= 1.0,
        "砂糖2箇所が同一連結成分で結ばれる(flow_connected)ことが中央値で確認できない: flags={:?}",
        connected_flags
    );
    let mc = median3(conductances.clone());
    assert!(
        mc.is_finite() && mc > 0.0,
        "端子間コンダクタンスが有限正であることが中央値で確認できない: conductances={:?}",
        conductances
    );
}

// ---------- ⑤ 不変条件 ----------

#[test]
fn accept5_invariants() {
    let (world, np, hx, hy) = world_and_params();
    let (s1x, s1y) = nearest_land(&world, hx + 12.0, hy);
    let (s2x, s2y) = nearest_land(&world, hx - 8.0, hy + 8.0);
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: s1x, y: s1y, strength: 400.0 } },
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: s2x, y: s2y, strength: 300.0 } },
    ];
    let ticks = 300;
    let eps = 1.0e-3;

    for &seed in &SEEDS {
        let r = run_netphys_headless(seed, &script, ticks, &np, &world);
        let s = &r.final_state;

        // 有限性
        assert!(s.free_budget.is_finite(), "seed {seed}: free_budget 非有限");
        for (i, nd) in s.nodes.iter().enumerate() {
            assert!(nd.x.is_finite() && nd.y.is_finite(), "seed {seed}: node {i} 座標非有限");
        }
        for (i, e) in s.edges.iter().enumerate() {
            assert!(e.d.is_finite() && e.l.is_finite(), "seed {seed}: edge {i} D/L 非有限");
        }

        // 保存則: total_mass == collected_total - consumed_total、非負、free超過なし
        assert!(s.free_budget >= -eps, "seed {seed}: free_budget 負 ({})", s.free_budget);
        let tm = total_mass(s);
        assert!(tm >= -eps, "seed {seed}: total_mass 負");
        let rhs = s.collected_total - s.consumed_total;
        assert!(
            (tm - rhs).abs() <= eps,
            "seed {seed}: 保存則破れ total_mass={tm} collected-consumed={rhs}"
        );
        assert!(s.free_budget <= tm + eps, "seed {seed}: free_budget が total_mass を超過");
        for (i, e) in s.edges.iter().enumerate() {
            assert!(e.d >= 0.0, "seed {seed}: edge {i} D 負");
            assert!(e.l >= 0.0, "seed {seed}: edge {i} L 負");
        }

        // 境界（全ノードが陸・範囲内）
        for (i, nd) in s.nodes.iter().enumerate() {
            let (x, y) = (nd.x, nd.y);
            assert!(
                x >= 0.0 && x < world.w as f64 && y >= 0.0 && y < world.h as f64,
                "seed {seed}: node {i} 範囲外 ({x},{y})"
            );
            let cix = (x.floor() as usize).min(world.w - 1);
            let ciy = (y.floor() as usize).min(world.h - 1);
            assert!(world.land_mask[ciy * world.w + cix], "seed {seed}: node {i} 海上");
        }

        // 再現性
        let r2 = run_netphys_headless(seed, &script, ticks, &np, &world);
        assert_eq!(r.final_state_hash, r2.final_state_hash, "seed {seed}: hash 再現性違反");
    }

    // ソフト標高忌避: home からほぼ同距離(D=14)の高標高/低標高ターゲットへ、予算を絞った
    // regime(標高コストが支配的)で伸ばし、構造質量(=Σ D*L)を比較する。
    // 高標高側は低標高側より有意に構造成長が抑制される（tree_growth_001 accept5 と同趣旨）。
    let mut np_scarce = np;
    np_scarce.initial_budget = 20.0;
    let (high_xy, low_xy) = find_high_low_targets(&world, hx, hy, 14.0, 2.0);
    let mut high_struct = Vec::new();
    let mut low_struct = Vec::new();
    for &seed in &SEEDS {
        let mut sh = initial_net_state(seed, &world, &np_scarce);
        for t in 0..60u64 {
            let ops = if t == 0 {
                vec![Op::PlaceSugar { x: high_xy.0, y: high_xy.1, strength: 200.0 }]
            } else {
                Vec::new()
            };
            netphys_step(&mut sh, &world, &np_scarce, &ops);
        }
        let mass_h = total_mass(&sh);
        high_struct.push(mass_h - sh.free_budget);

        let mut sl = initial_net_state(seed, &world, &np_scarce);
        for t in 0..60u64 {
            let ops = if t == 0 {
                vec![Op::PlaceSugar { x: low_xy.0, y: low_xy.1, strength: 200.0 }]
            } else {
                Vec::new()
            };
            netphys_step(&mut sl, &world, &np_scarce, &ops);
        }
        let mass_l = total_mass(&sl);
        low_struct.push(mass_l - sl.free_budget);
    }
    let mh = median3(high_struct.clone());
    let ml = median3(low_struct.clone());
    assert!(
        mh < ml * 0.95,
        "ソフト標高忌避が確認できない: high_median={mh} low_median={ml} (high={:?} low={:?})",
        high_struct,
        low_struct
    );
}

fn find_high_low_targets(world: &World, hx: f64, hy: f64, d: f64, tol: f64) -> ((f64, f64), (f64, f64)) {
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
    (
        ((hi % world.w) as f64 + 0.5, (hi / world.w) as f64 + 0.5),
        ((li % world.w) as f64 + 0.5, (li / world.w) as f64 + 0.5),
    )
}

// ---------- ⑥ 有界（性能） ----------

#[test]
fn accept6_bounded_node_edge_count() {
    let (world, np, hx, hy) = world_and_params();
    let (s1x, s1y) = nearest_land(&world, hx + 12.0, hy);
    let (s2x, s2y) = nearest_land(&world, hx - 8.0, hy + 8.0);
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: s1x, y: s1y, strength: 400.0 } },
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: s2x, y: s2y, strength: 300.0 } },
    ];
    // cap に到達しうるほど長時間（節目である N の倍数を多数含む）走らせても爆発しないことを見る。
    let ticks = 800;
    for &seed in &SEEDS {
        let r = run_netphys_headless(seed, &script, ticks, &np, &world);
        let s = &r.final_state;
        assert!(
            s.n_nodes() <= np.node_cap,
            "seed {seed}: node_cap 超過 nodes={} cap={}",
            s.n_nodes(),
            np.node_cap
        );
        assert!(
            s.n_edges() <= np.edge_cap,
            "seed {seed}: edge_cap 超過 edges={} cap={}",
            s.n_edges(),
            np.edge_cap
        );
    }
}
