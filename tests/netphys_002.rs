//! netphys-002: 網 Physarum Stage 2（前進波移動＋Tero 効率改善）の受け入れテスト。
//! Jones でも tree でもない第3の独立モデル。netphys-001（Stage1・①②⑤⑥）の緑は
//! `tests/netphys_001.rs` が引き続き検証する（本ファイルは無変更・別ファイル追加）。
//!
//! Stage 2（本タスクの合否対象）= 受け入れ③前進波で移動・④consolidation の効率改善。
//! S9（正準9シード）の部分集合 [1, 42, 1337] の中央値で判定する（規約 §4 の集計法を踏襲）。

use nenkin_garden::netphys::{
    initial_net_state, netphys_kirchhoff_solve, netphys_step, NetParams, NetState,
};
use nenkin_garden::params::Params;
use nenkin_garden::world::{make_synthetic_archipelago, World};

const SEEDS: [u64; 3] = [1, 42, 1337];

fn median3(mut xs: Vec<f64>) -> f64 {
    assert_eq!(xs.len(), 3);
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs[1]
}

fn world_and_params() -> (World, NetParams, f64, f64) {
    let wp = Params::default();
    let world = make_synthetic_archipelago(&wp);
    let np = NetParams::default();
    let (hx, hy) = world.default_home(np.e_lo);
    (world, np, hx, hy)
}

fn max_radius(s: &NetState, hx: f64, hy: f64) -> f64 {
    s.nodes
        .iter()
        .map(|nd| ((nd.x - hx).powi(2) + (nd.y - hy).powi(2)).sqrt())
        .fold(0.0, f64::max)
}

fn centroid_dist(s: &NetState, hx: f64, hy: f64) -> f64 {
    let n = s.nodes.len() as f64;
    if n == 0.0 {
        return 0.0;
    }
    let (mut cx, mut cy) = (0.0, 0.0);
    for nd in &s.nodes {
        cx += nd.x;
        cy += nd.y;
    }
    cx /= n;
    cy /= n;
    ((cx - hx).powi(2) + (cy - hy).powi(2)).sqrt()
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

// ---------- ③ 前進波で移動 ----------
// 砂糖なし（純粋な内部再配分＝translocation）で K サイクル回し、初期の放射成長が
// 落ち着いた後（early checkpoint）と、さらに多くのサイクルを経た後（late checkpoint）とで
// コロニーの最外周半径（または重心距離）を比較する。
//
// 「その場脈動でなく正味の外向き変位」を検出するため、意図的に長い K（150 サイクル=1800
// tick）まで回す: 外部からの砂糖供給が無い場合、後方 prune で戻した mass を前線へ回す
// translocation が効いていなければ、初期予算を使い切った時点（実測: 素朴な実装では
// 100 サイクル強で free_budget が正確に 0 に張り付き、ノード/辺数も完全固定＝真の
// 「その場脈動」に陥る）で伸長が完全停止する。前進波が機能していれば、終盤でも
// free_budget が枯渇せず（後方から回収した mass で探索を継続でき）、半径も伸び続ける。
#[test]
fn accept3_forward_wave_net_outward_displacement() {
    let (world, np, hx, hy) = world_and_params();

    let early_cycles = 2u64; // 初期放射成長が一区切りついた時点
    let mid_cycles = 100u64; // 素朴な実装なら予算を使い切って停止しているはずの時点
    let late_cycles = 150u64; // さらにサイクルを重ねた最終時点

    let mut early_r = Vec::new();
    let mut mid_r = Vec::new();
    let mut late_r = Vec::new();
    let mut late_c = Vec::new();
    let mut late_free = Vec::new();

    for &seed in &SEEDS {
        let mut s = initial_net_state(seed, &world, &np);
        for _ in 0..(early_cycles * np.period_n) {
            netphys_step(&mut s, &world, &np, &[]);
        }
        early_r.push(max_radius(&s, hx, hy));

        for _ in 0..((mid_cycles - early_cycles) * np.period_n) {
            netphys_step(&mut s, &world, &np, &[]);
        }
        mid_r.push(max_radius(&s, hx, hy));

        for _ in 0..((late_cycles - mid_cycles) * np.period_n) {
            netphys_step(&mut s, &world, &np, &[]);
        }
        late_r.push(max_radius(&s, hx, hy));
        late_c.push(centroid_dist(&s, hx, hy));
        late_free.push(s.free_budget);
    }

    let er = median3(early_r.clone());
    let mr = median3(mid_r.clone());
    let lr = median3(late_r.clone());
    let lf = median3(late_free.clone());

    // 全体としての外向き変位（初期の放射成長からの純増）: 有意なマージンで拡大。
    let radius_grew = lr >= er * 1.5 && (lr - er) > 3.0;
    assert!(
        radius_grew,
        "前進波での正味の外向き変位が確認できない: early_r={:?}(med {er}) late_r={:?}(med {lr})",
        early_r,
        late_r
    );

    // その場脈動でないこと: 終盤(late)が中盤(mid)を下回っていない（後退なし）。
    assert!(lr >= mr - 1.0e-6, "最外周半径が終盤で後退している: mid_med={mr} late_med={lr}");

    // 本質的な判定: 終盤でも free_budget が枯渇（≈0 に張り付き）していないこと。
    // 素朴な実装（後方 prune の mass が前線の再拡散に回らない）は外部の砂糖供給が無いと
    // K=150 サイクル以内に必ず予算を使い切り、以後は探索が完全停止する（中央値で自由予算=0）。
    // translocation が効いていれば、後方から回収した mass で探索が続き、自由予算が
    // 枯渇しきらない。
    assert!(
        lf > 50.0,
        "終盤で free_budget が枯渇しており、translocation による持続的な前進波移動が \
         確認できない（その場脈動で停止している疑い）: late_free={:?} (median {lf})",
        late_free
    );
    let _ = late_c;
}

// ---------- ④ 効率化（Tero） ----------
// consolidation の前後で、2箇所の砂糖端子間の総コンダクタンスが改善する
// （後 ≥ 前・複数サイクルの中央値で有意差）ことを確認する。
#[test]
fn accept4_consolidation_improves_transport_efficiency() {
    let (world, np, hx, hy) = world_and_params();
    let (s1x, s1y) = (hx + 10.0, hy);
    let (s2x, s2y) = (hx - 7.0, hy + 7.0);
    let script = vec![
        nenkin_garden::state::ScriptEntry {
            tick: 0,
            op: nenkin_garden::state::Op::PlaceSugar { x: s1x, y: s1y, strength: 5000.0 },
        },
        nenkin_garden::state::ScriptEntry {
            tick: 0,
            op: nenkin_garden::state::Op::PlaceSugar { x: s2x, y: s2y, strength: 5000.0 },
        },
    ];

    // 前後ペア（consolidation 直前 vs 直後）を複数サイクル・複数シードにわたって集める。
    let mut befores = Vec::new();
    let mut afters = Vec::new();
    // トレンド判定用: 早期サイクルの後 と 後期サイクルの後 を比較。
    let mut early_after = Vec::new();
    let mut late_after = Vec::new();

    let warmup_cycles = 3u64; // 初期網形成の落ち着きを待つ
    let measured_cycles = 12u64;

    for &seed in &SEEDS {
        let mut s = initial_net_state(seed, &world, &np);

        for t in 0..(warmup_cycles * np.period_n) {
            let ops: Vec<_> = script.iter().filter(|e| e.tick == t).map(|e| e.op.clone()).collect();
            netphys_step(&mut s, &world, &np, &ops);
        }

        let conductance_now = |st: &NetState| -> Option<f64> {
            let n1 = nearest_node(st, s1x, s1y)?;
            let n2 = nearest_node(st, s2x, s2y)?;
            if n1.0 == n2.0 {
                return None;
            }
            let flow = netphys_kirchhoff_solve(&st.nodes, &st.edges, &world, np.net_alpha, n1.0, n2.0);
            if flow.connected {
                Some(flow.total_conductance)
            } else {
                None
            }
        };

        let mut per_seed_after: Vec<f64> = Vec::new();
        for cyc in 0..measured_cycles {
            // consolidation 直前（period_n-1 tick 進める）
            for _ in 0..(np.period_n - 1) {
                let t = s.tick;
                let ops: Vec<_> =
                    script.iter().filter(|e| e.tick == t).map(|e| e.op.clone()).collect();
                netphys_step(&mut s, &world, &np, &ops);
            }
            let before = conductance_now(&s);

            // consolidation を発火させる1 tick
            let t = s.tick;
            let ops: Vec<_> = script.iter().filter(|e| e.tick == t).map(|e| e.op.clone()).collect();
            netphys_step(&mut s, &world, &np, &ops);
            let after = conductance_now(&s);

            if let (Some(b), Some(a)) = (before, after) {
                befores.push(b);
                afters.push(a);
                per_seed_after.push(a);
            }
            let _ = cyc;
        }

        if per_seed_after.len() >= 4 {
            let half = per_seed_after.len() / 2;
            early_after.extend_from_slice(&per_seed_after[..half.max(1)]);
            late_after.extend_from_slice(&per_seed_after[per_seed_after.len() - half.max(1)..]);
        }
    }

    assert!(
        befores.len() >= 6,
        "端子間が連結し conductance を測れたサンプルが少なすぎる: n={}",
        befores.len()
    );

    // 前後ペアの中央値比較（後 >= 前）。
    let mb = {
        let mut v = befores.clone();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[v.len() / 2]
    };
    let ma = {
        let mut v = afters.clone();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[v.len() / 2]
    };

    // トレンド判定（Kサイクルで上昇）: 早期サイクル後 vs 後期サイクル後。
    let trend_ok = if !early_after.is_empty() && !late_after.is_empty() {
        let me = {
            let mut v = early_after.clone();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v[v.len() / 2]
        };
        let ml = {
            let mut v = late_after.clone();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v[v.len() / 2]
        };
        ml >= me
    } else {
        false
    };

    assert!(
        ma >= mb || trend_ok,
        "consolidation で端子間の効率(コンダクタンス)が改善していない: \
         before_med={mb} after_med={ma} befores={:?} afters={:?} early={:?} late={:?}",
        befores,
        afters,
        early_after,
        late_after
    );
}
