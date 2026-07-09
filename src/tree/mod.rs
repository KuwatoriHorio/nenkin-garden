//! tree-growth-001: α成長木モデル（space colonization, 全体予算B）の独立モジュール。
//! 現行 Jones モデル（src/{state,step,metrics,hash}.rs 等）とは無関係・並置。
//! world.rs（陸海マスク・標高場・default_home）のみ読み取り共有する。

pub mod hash;
pub mod state;
pub mod step;

pub use hash::tree_state_hash;
pub use state::{
    apply_tree_op, initial_tree_state, path_len, remove_depleted_sugar, total_volume, Node,
    TreeParams, TreeState,
};
pub use step::tree_step;

use crate::state::ScriptEntry;
use crate::world::World;

pub struct TreeRunResult {
    pub final_state: TreeState,
    pub final_state_hash: u64,
}

/// ヘッドレス実行（run_headless.rs と同じ骨格を新モデル用に踏襲）。
pub fn run_tree_headless(
    seed: u64,
    input_script: &[ScriptEntry],
    ticks: u64,
    params: &TreeParams,
    world: &World,
) -> TreeRunResult {
    let mut state = initial_tree_state(seed, world, params);
    for _ in 0..ticks {
        let cur = state.tick;
        let ops: Vec<crate::state::Op> = input_script
            .iter()
            .filter(|e| e.tick == cur)
            .map(|e| e.op.clone())
            .collect();
        tree_step(&mut state, world, params, &ops);
    }
    let final_state_hash = tree_state_hash(&state, params);
    TreeRunResult { final_state: state, final_state_hash }
}
