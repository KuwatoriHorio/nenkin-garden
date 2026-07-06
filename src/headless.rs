//! run_headless（設計メモ §7）。core を呼ぶだけ。描画は一切呼ばない。
//! 描画ON/OFFで final_state_hash が変わらないことを「描画を呼ばない」構造で担保する。

use std::time::Instant;

use crate::hash::state_hash;
use crate::metrics::{compute_metrics, Metrics};
use crate::params::Params;
use crate::state::{initial_state, ScriptEntry, State};
use crate::step::step;
use crate::world::{make_synthetic_archipelago, World};

pub struct RunResult {
    pub metrics: Metrics,
    pub final_state_hash: u64,
    pub final_state: State,
}

/// ヘッドレス実行。metrics と final_state_hash を返す。
/// world を省略（None）した場合は既定の合成列島を生成する。
pub fn run_headless(
    seed: u64,
    input_script: &[ScriptEntry],
    ticks: u64,
    params: &Params,
    world: Option<&World>,
) -> RunResult {
    let owned_world;
    let w: &World = match world {
        Some(w) => w,
        None => {
            owned_world = make_synthetic_archipelago(params);
            &owned_world
        }
    };

    let mut state = initial_state(seed, w, params);

    // tick ごとの op を引くための索引（同 tick 内はスクリプト順を保持）
    let mut t_accum = 0.0f64;
    for _ in 0..ticks {
        let cur = state.tick;
        let ops: Vec<crate::state::Op> = input_script
            .iter()
            .filter(|e| e.tick == cur)
            .map(|e| e.op.clone())
            .collect();
        let t0 = Instant::now();
        step(&mut state, w, params, &ops);
        t_accum += t0.elapsed().as_secs_f64();
    }

    let mut metrics = compute_metrics(&state, w, params);
    metrics.tick_ms = if ticks > 0 {
        t_accum / ticks as f64 * 1000.0
    } else {
        0.0
    };
    let final_state_hash = state_hash(&state, params);

    RunResult {
        metrics,
        final_state_hash,
        final_state: state,
    }
}
