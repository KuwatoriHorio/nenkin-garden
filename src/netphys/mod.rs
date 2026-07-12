//! netphys-001: 網 Physarum（移動性フォアジング前進波）の独立モジュール。
//! Jones でも tree でもない第3の独立モデル。現行 Jones モデル（src/{state,step,metrics,hash}.rs 等）・
//! tree モデル（src/tree/）とは無関係・並置。world.rs（陸海マスク・標高場・default_home）のみ
//! 読み取り共有し、Kirchhoff の線形代数部分（密ガウス消去）のみ `analysis::flow::solve_dense`
//! を可視性調整（`pub(crate)`）のうえ再利用する（analysis の挙動・テストは無変更）。
//!
//! Stage 1（netphys-001, 本モジュール）の合否対象は「網化・餌を結ぶ・不変条件・有界」（設計メモの
//! 受け入れ①②⑤⑥）。前進波移動・consolidation の効率改善（③④）は netphys-002 に繰り延べる。

pub mod hash;
pub mod kirchhoff;
pub mod state;
pub mod step;

pub use hash::netphys_state_hash;
pub use kirchhoff::{solve as netphys_kirchhoff_solve, NetFlowResult};
pub use state::{
    apply_net_op, initial_net_state, remove_depleted_sugar, total_mass, NEdge, NNode, NetParams,
    NetState,
};
pub use step::netphys_step;

use crate::state::ScriptEntry;
use crate::world::World;

pub struct NetRunResult {
    pub final_state: NetState,
    pub final_state_hash: u64,
}

/// ヘッドレス実行（run_headless.rs / tree/mod.rs の run_tree_headless と同じ骨格を踏襲）。
pub fn run_netphys_headless(
    seed: u64,
    input_script: &[ScriptEntry],
    ticks: u64,
    params: &NetParams,
    world: &World,
) -> NetRunResult {
    let mut state = initial_net_state(seed, world, params);
    for _ in 0..ticks {
        let cur = state.tick;
        let ops: Vec<crate::state::Op> = input_script
            .iter()
            .filter(|e| e.tick == cur)
            .map(|e| e.op.clone())
            .collect();
        netphys_step(&mut state, world, params, &ops);
    }
    let final_state_hash = netphys_state_hash(&state, params);
    NetRunResult { final_state: state, final_state_hash }
}
