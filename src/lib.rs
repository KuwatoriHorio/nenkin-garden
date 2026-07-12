//! 粘菌ガーデン 決定論シミュレーションコア (core-000)。
//!
//! モジュール構成（設計メモ §2, 描画と分離）:
//!   core: params / world / state / rng / step / metrics / hash（純粋・副作用なし）
//!   headless: run_headless エントリ（core を呼ぶだけ）
//!
//! analysis/ render/ は本タスク対象外だが、依存方向（core ← analysis/render、逆は禁止）
//! を守るため後付けする。core はそれらに依存しない。

pub mod analysis;
pub mod graph_svg;
pub mod hash;
pub mod headless;
pub mod metrics;
pub mod netphys;
pub mod params;
pub mod rng;
pub mod state;
pub mod step;
pub mod tree;
pub mod world;

pub use analysis::{analyze, mean_land_elevation, AnalysisMetrics, AnalysisResult};
pub use hash::state_hash;
pub use headless::{run_headless, RunResult};
pub use metrics::{compute_metrics, Metrics};
pub use params::Params;
pub use state::{apply_op, initial_state, remove_depleted_sugar, Op, ScriptEntry, State};
pub use step::step;
pub use world::{make_synthetic_archipelago, World};
