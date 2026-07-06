//! analysis-003 受け入れテスト（tasks/task-analysis-003.md）。
//! flow ソルバが浮遊成分を含んでも source/sink 成分だけを解いて連結を正しく報告する。

use nenkin_garden::analysis::analyze;
use nenkin_garden::hash::state_hash;
use nenkin_garden::params::Params;
use nenkin_garden::state::{apply_op, initial_state, Op, State};
use nenkin_garden::world::World;

fn flat_world(h: usize, w: usize) -> World {
    World { h, w, land_mask: vec![true; h * w], e: vec![0.0f32; h * w] }
}

fn base_state(p: &Params, world: &World) -> State {
    let mut st = initial_state(1, world, p);
    for v in st.trail.iter_mut() {
        *v = 0.0;
    }
    st
}

/// #1 浮遊成分があっても解ける: source/sink を繋ぐ網A + 無関係な孤立網B → connected。
#[test]
fn floating_component_does_not_break_solve() {
    let mut p = Params::default();
    p.h = 12;
    p.w = 20;
    let world = flat_world(12, 20);
    let mut st = base_state(&p, &world);
    // 網A: y=4, x=1..=8（2源を繋ぐ本体）
    for x in 1..=8 {
        st.trail[4 * 20 + x] = 1.0;
    }
    // 浮遊網B: y=8, x=12..=16（source/sink と非連結・以前はこれが Laplacian を特異化させた）
    for x in 12..=16 {
        st.trail[8 * 20 + x] = 1.0;
    }
    apply_op(&mut st, &Op::PlaceSugar { x: 1.5, y: 4.5, strength: 100.0 });
    apply_op(&mut st, &Op::PlaceSugar { x: 8.5, y: 4.5, strength: 100.0 });

    let a = analyze(&st, &world, &p).metrics;
    assert!(a.flow_connected, "浮遊成分により特異化して連結が false になった");
    assert!(a.total_conductance > 0.0, "conductance 非正");
    assert!(a.effective_resistance > 0.0, "実効抵抗 非正");
}

/// #2 真に別成分の2源は依然 false（過剰連結を作らない）。
#[test]
fn genuinely_separate_stays_disconnected() {
    let mut p = Params::default();
    p.h = 12;
    p.w = 24;
    let world = flat_world(12, 24);
    let mut st = base_state(&p, &world);
    // 網1: x=1..=3, 網2: x=19..=22（源の tap 半径外で分離）
    for x in 1..=3 {
        st.trail[4 * 24 + x] = 1.0;
    }
    for x in 19..=22 {
        st.trail[4 * 24 + x] = 1.0;
    }
    apply_op(&mut st, &Op::PlaceSugar { x: 1.5, y: 4.5, strength: 100.0 });
    apply_op(&mut st, &Op::PlaceSugar { x: 21.5, y: 4.5, strength: 100.0 });

    let a = analyze(&st, &world, &p).metrics;
    assert!(!a.flow_connected, "離れた独立網を誤って連結した");
}

/// #3 決定性・非侵襲。
#[test]
fn deterministic_non_invasive() {
    let mut p = Params::default();
    p.h = 12;
    p.w = 20;
    let world = flat_world(12, 20);
    let mut st = base_state(&p, &world);
    for x in 1..=8 {
        st.trail[4 * 20 + x] = 1.0;
    }
    for x in 12..=16 {
        st.trail[8 * 20 + x] = 1.0;
    }
    apply_op(&mut st, &Op::PlaceSugar { x: 1.5, y: 4.5, strength: 100.0 });
    apply_op(&mut st, &Op::PlaceSugar { x: 8.5, y: 4.5, strength: 100.0 });

    let h0 = state_hash(&st, &p);
    let a = analyze(&st, &world, &p).metrics;
    let b = analyze(&st, &world, &p).metrics;
    assert_eq!(h0, state_hash(&st, &p), "解析が state_hash を変えた");
    assert_eq!(a.total_conductance.to_bits(), b.total_conductance.to_bits());
    assert_eq!(a.effective_resistance.to_bits(), b.effective_resistance.to_bits());
}
