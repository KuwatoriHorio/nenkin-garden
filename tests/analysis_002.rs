//! analysis-002 受け入れテスト（tasks/task-analysis-002.md）。
//! 砂糖源を近傍の実ネットワークへ tap し、拡張グラフ上で連結判定することを検証する。
//! core 非変更・非侵襲・決定的。

use nenkin_garden::analysis::analyze;
use nenkin_garden::hash::state_hash;
use nenkin_garden::params::Params;
use nenkin_garden::state::{apply_op, initial_state, Op};
use nenkin_garden::world::World;

/// 平坦・全陸の世界（E=0）。
fn flat_world(h: usize, w: usize) -> World {
    World { h, w, land_mask: vec![true; h * w], e: vec![0.0f32; h * w] }
}

/// #2 近傍網への接続: 本体網 + その半径内に置いた砂糖源（孤立スパイク）→ 連結。
/// 旧規則（唯一の最近傍snap）では sugarB が孤立スパイクに吸着し false だった。
#[test]
fn sugar_taps_nearby_network() {
    let mut p = Params::default();
    p.h = 9;
    p.w = 14;
    let world = flat_world(9, 14);
    let mut st = initial_state(1, &world, &p);
    for v in st.trail.iter_mut() {
        *v = 0.0;
    }
    // 本体網: y=4 の水平線 x=1..=6
    for x in 1..=6 {
        st.trail[4 * 14 + x] = 1.0;
    }
    // 砂糖Bのビーコン孤立スパイク: (x=9,y=4) 単独（本体網の端 x=6 とは 3 セル）
    st.trail[4 * 14 + 9] = 1.0;

    // 砂糖A=本体網の端, 砂糖B=スパイク位置（本体網の半径内）
    apply_op(&mut st, &Op::PlaceSugar { x: 1.5, y: 4.5, strength: 100.0 });
    apply_op(&mut st, &Op::PlaceSugar { x: 9.5, y: 4.5, strength: 100.0 });

    let a = analyze(&st, &world, &p).metrics;
    assert!(a.flow_connected, "近傍網に tap できず flow_connected=false");
    assert!(a.total_conductance > 0.0, "conductance 非正");
    assert!(a.effective_resistance > 0.0, "実効抵抗 非正");
}

/// #3 過剰連結の防止: 半径を超えて離れた2つの独立網 → 連結しない。
#[test]
fn no_over_connection() {
    let mut p = Params::default();
    p.h = 9;
    p.w = 16;
    let world = flat_world(9, 16);
    let mut st = initial_state(1, &world, &p);
    for v in st.trail.iter_mut() {
        *v = 0.0;
    }
    // 網1: x=1..=3, 網2: x=11..=14（ギャップ x=4..10 は 0, 端 x3↔x11 は 8 セル > tap_radius）
    for x in 1..=3 {
        st.trail[4 * 16 + x] = 1.0;
    }
    for x in 11..=14 {
        st.trail[4 * 16 + x] = 1.0;
    }
    apply_op(&mut st, &Op::PlaceSugar { x: 1.5, y: 4.5, strength: 100.0 });
    apply_op(&mut st, &Op::PlaceSugar { x: 13.5, y: 4.5, strength: 100.0 });

    let a = analyze(&st, &world, &p).metrics;
    assert!(!a.flow_connected, "離れた独立網を誤って連結した");
}

/// #1 決定性/非侵襲（#2 シナリオで確認）。
#[test]
fn deterministic_and_non_invasive() {
    let mut p = Params::default();
    p.h = 9;
    p.w = 14;
    let world = flat_world(9, 14);
    let mut st = initial_state(1, &world, &p);
    for v in st.trail.iter_mut() {
        *v = 0.0;
    }
    for x in 1..=6 {
        st.trail[4 * 14 + x] = 1.0;
    }
    st.trail[4 * 14 + 9] = 1.0;
    apply_op(&mut st, &Op::PlaceSugar { x: 1.5, y: 4.5, strength: 100.0 });
    apply_op(&mut st, &Op::PlaceSugar { x: 9.5, y: 4.5, strength: 100.0 });

    let h_before = state_hash(&st, &p);
    let a = analyze(&st, &world, &p).metrics;
    let b = analyze(&st, &world, &p).metrics;
    let h_after = state_hash(&st, &p);

    assert_eq!(h_before, h_after, "解析が state_hash を変えた");
    assert_eq!(a.total_conductance.to_bits(), b.total_conductance.to_bits());
    assert_eq!(a.transport_efficiency.to_bits(), b.transport_efficiency.to_bits());
    assert_eq!(a.effective_resistance.to_bits(), b.effective_resistance.to_bits());
}
