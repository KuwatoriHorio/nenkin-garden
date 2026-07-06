//! analysis-001: 生き物網の効率ネットワーク解析（静的・裏方・非侵襲）。
//!
//! Jones コアが育てた trail 網を、適応ネットワーク（Tero–Nakagaki）の土俵に一度だけ
//! 乗せて効率を数値化する静的レイヤ。**適応則は回さない**（管の太化/細化なし）。
//! State は読むだけ・書き換え禁止（受け入れ #6）。同一 state_hash → 同一指標（#1）。
//!
//! パイプライン: しきい化 → 骨格抽出 → グラフ化 → 流れを1回解く → 指標算出。
//!
//! ## transport_efficiency の定義（候補提示と選定理由）
//! 候補:
//!   (A) 最大流エッジ流量 / 総流量（単一幹線への集中）
//!   (B) 上位k本の流量占有率（k 依存のため恣意性が残る）
//!   (C) 正規化エッジ流量の Herfindahl 指数 Σ(I_e/ΣI)^2  ← 採用
//! 採用理由: (C) は k などの外生パラメータ不要で全エッジを使い、値域 (0,1] が明快
//! （1 に近いほど少数の幹線へ集約＝効率的な輸送、分散するほど 0 へ）。(A) は外れ値
//! 1本に過敏、(B) は k の恣意性がある。決定的で State から一意に定まる（#1）。

pub mod flow;
pub mod graph;
pub mod skeleton;

use crate::params::Params;
use crate::state::State;
use crate::world::World;

#[derive(Clone, Debug)]
pub struct AnalysisMetrics {
    pub nodes: usize,
    pub edges: usize,
    pub total_length: f64,
    pub mst_length: f64,
    pub redundancy: f64,
    pub total_conductance: f64,
    pub effective_resistance: f64,
    pub transport_efficiency: f64,
    pub edge_mean_elevation: f64,
    pub num_cc: usize,
    pub largest_cc: usize,
    pub flow_connected: bool,
}

impl AnalysisMetrics {
    /// 外部依存なしの手書き JSON。
    pub fn to_json(&self) -> String {
        format!(
            "{{\"nodes\":{},\"edges\":{},\"total_length\":{},\"mst_length\":{},\
\"redundancy\":{},\"total_conductance\":{},\"effective_resistance\":{},\
\"transport_efficiency\":{},\"edge_mean_elevation\":{},\"num_cc\":{},\
\"largest_cc\":{},\"flow_connected\":{}}}",
            self.nodes,
            self.edges,
            self.total_length,
            self.mst_length,
            self.redundancy,
            self.total_conductance,
            self.effective_resistance,
            self.transport_efficiency,
            self.edge_mean_elevation,
            self.num_cc,
            self.largest_cc,
            self.flow_connected,
        )
    }
}

pub struct AnalysisResult {
    pub metrics: AnalysisMetrics,
    pub graph: graph::NetworkGraph,
}

/// 陸地全体の平均標高（受け入れ #4 の比較基準）。
pub fn mean_land_elevation(world: &World) -> f64 {
    let mut s = 0.0;
    let mut c = 0usize;
    for i in 0..world.h * world.w {
        if world.land_mask[i] {
            s += world.e[i] as f64;
            c += 1;
        }
    }
    if c > 0 {
        s / c as f64
    } else {
        0.0
    }
}

/// State（特に trail）を読むだけの静的解析（副作用なし）。
pub fn analyze(state: &State, world: &World, p: &Params) -> AnalysisResult {
    // 1. しきい化 → 2. 骨格抽出（トポロジ保存: 成分数を core と一致させる）
    let mask = skeleton::binarize(state, world, p);
    let mut skel = skeleton::thin(&mask, world.h, world.w, p.skeleton_max_iter);
    skeleton::preserve_components(&mask, &mut skel, world.h, world.w);

    // 3. グラフ化
    let g = graph::build(&skel, world);

    // エッジ幾何
    let total_length: f64 = g.edges.iter().map(|e| e.length).sum();
    let mst_length = graph::mst_length(&g);
    let redundancy = if mst_length > 1e-12 {
        total_length / mst_length
    } else {
        0.0
    };
    // 加重平均標高（length 加重）
    let (mut wsum, mut lsum) = (0.0, 0.0);
    for e in &g.edges {
        wsum += e.length * e.mean_e;
        lsum += e.length;
    }
    let edge_mean_elevation = if lsum > 1e-12 { wsum / lsum } else { 0.0 };

    // 4. 流れを1回解く
    let f = flow::solve(&g, state, world, p);

    let metrics = AnalysisMetrics {
        nodes: g.node_px.len(),
        edges: g.edges.len(),
        total_length,
        mst_length,
        redundancy,
        total_conductance: f.total_conductance,
        effective_resistance: f.effective_resistance,
        transport_efficiency: f.transport_efficiency,
        edge_mean_elevation,
        num_cc: g.num_cc,
        largest_cc: g.largest_cc,
        flow_connected: f.connected,
    };

    AnalysisResult { metrics, graph: g }
}
