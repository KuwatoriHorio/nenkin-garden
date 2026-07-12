//! netphys-001: 網 Physarum（担体A・Tero純）の State（設計メモ §1, タスク仕様）。
//! 現行 Jones モデル(src/{state,step}.rs)・tree モデル(src/tree/)とは独立（並置・無関係）。
//! world.rs（陸海マスク・標高場・default_home）のみ読み取り共有する。
//!
//! 一般グラフ（ループ可）。ノード `{x,y}`、辺 `{a,b,d,l}`（無向, d=コンダクタンス, l=長さ）。
//! 質量 = `free_budget + Σ(d_e*l_e)`（担体A: 構造質量は辺に載る）。

use crate::rng::Rng;
use crate::state::Op;
use crate::world::World;

/// 一般グラフのノード。index が id（正準順序、ただし consolidation の prune で再採番されうる）。
#[derive(Clone, Debug)]
pub struct NNode {
    pub x: f64,
    pub y: f64,
}

/// 一般グラフの辺（無向・ループ可・多重辺可）。`a<b` を正準形とする。
#[derive(Clone, Debug)]
pub struct NEdge {
    pub a: usize,
    pub b: usize,
    pub d: f64, // コンダクタンス（構造の太さ）
    pub l: f64, // 実長
}

/// netphys 専用パラメータ（現行 `Params`/`TreeParams` は汚さない）。既定値は実測で調整（実装者裁量）。
#[derive(Clone, Copy, Debug)]
pub struct NetParams {
    /// consolidation の周期（tick）。
    pub period_n: u64,
    /// anastomosis（融合）判定距離。
    pub fusion_dist: f64,
    /// consolidation で選ぶ最外周（新前線）の点数。
    pub k_frontier: usize,
    /// Phase1 探索の1tickあたりの伸長距離。
    pub search_step: f64,
    /// 誘引（砂糖）を感知する半径。
    pub attract_radius: f64,
    /// 探索方向のうちランダム成分の重み（0で誘引のみ, 誘引が無い前線は探索できず停止するため
    /// 網化には >0 が必要）。
    pub w_rand: f64,
    /// 新規辺の初期コンダクタンス。
    pub d0: f64,
    /// 標高コスト割増係数（探索コスト・consolidation の実効長の両方に使う＝ソフト忌避）。
    pub c_elev: f64,
    /// netphys-003: 探索方向バイアス係数（第3の忌避経路）。probe方向の合成ベクトルに
    /// `-w_elev*∇E`（局所標高勾配の逆方向＝低標高側）を加算し、低標高方向を確率的に優先する。
    /// 0 なら現状（方向バイアス無し）と完全一致（後方互換のフォールバック）。壁は作らない
    /// （高標高方向の重みが下がるだけで、確率的には登れる＝ソフト）。
    pub w_elev: f64,
    /// consolidation の Kirchhoff で使う実効長係数 L_eff = L*(1+alpha*meanE)。
    pub net_alpha: f64,
    /// Tero 強化係数（dD = tero_gain*|Q| 分を毎 consolidation 加算）。
    pub tero_gain: f64,
    /// Tero 減衰係数（毎 consolidation で D *= (1-tero_decay)）。
    pub tero_decay: f64,
    /// D がこれ未満の辺は consolidation で刈り込む。
    pub prune_eps: f64,
    /// 砂糖回収半径。
    pub sugar_radius: f64,
    /// 砂糖回収レート。
    pub collect_rate: f64,
    /// consolidation で砂糖を端子として tap する半径。
    pub sugar_tap_radius: f64,
    /// 初期予算 B0（= 初期 free_budget = 初期 collected_total）。
    pub initial_budget: f64,
    /// ホーム決定に使う低標高帯しきい（`World::default_home` 用）。
    pub e_lo: f64,
    /// ノード数上限（有界性・爆発防止）。
    pub node_cap: usize,
    /// 辺数上限（有界性・爆発防止）。
    pub edge_cap: usize,
    /// state_hash の位置量子化幅。
    pub q_pos: f64,
    /// state_hash の体積/D/L量子化幅。
    pub q_vol: f64,
    /// netphys-004: Phase1 探索の扇状拡散。1前線ノードあたりに張る probe 本数。
    /// 1 なら中心方向のみ＝現状の単一方向伸長に完全縮退する（後方互換のフォールバック）。
    pub fan_count: usize,
    /// netphys-004: 扇の半角（ラジアン）。中心（誘引・乱数・標高バイアスの合成方向）を軸に
    /// `[-fan_spread, +fan_spread]` へ `fan_count` 本を等間隔に張る（`fan_count<=1` なら未使用）。
    pub fan_spread: f64,
}

impl Default for NetParams {
    fn default() -> Self {
        NetParams {
            period_n: 12,
            fusion_dist: 3.0,
            k_frontier: 4,
            search_step: 2.0,
            attract_radius: 40.0,
            w_rand: 1.0,
            d0: 0.35,
            c_elev: 1.5,
            // netphys-003: 通常予算(initial_budget=1200)でもコロニーが低標高帯に偏るために
            // 実測で調整（探索用一時テストで確認・削除済み）。0だと方向バイアス無し＝従来通り
            // 潤沢予算では標高を無視して登る（赤）。
            w_elev: 2.0,
            net_alpha: 1.0,
            tero_gain: 0.8,
            tero_decay: 0.5,
            prune_eps: 0.05,
            sugar_radius: 3.0,
            collect_rate: 0.5,
            sugar_tap_radius: 10.0,
            // netphys-002: consolidation の後方 prune で戻した ΣD*L を前線の再拡散(translocation)へ
            // 回す前進波移動(③)には、初期建設(anastomosis含む)を賄ってなお継続探索に回せる余剰が要る。
            // 400 だと初期網形成でほぼ使い切り「その場脈動」に留まる(実測: 探索用一時バイナリで確認・
            // 削除済み)。1200 で複数シード中央値で有意な持続的外向き成長を確認（netphys_002）。
            initial_budget: 1200.0,
            e_lo: 0.3,
            node_cap: 400,
            edge_cap: 1000,
            q_pos: 1.0e-4,
            q_vol: 1.0e-4,
            // netphys-004: 探索用一時テストで実測（面的指標: 角度カバレッジ/占有セル数/凸包面積が
            // fan_count=1（線的）比で有意に増え、かつ netphys-001/002/003 の既存受け入れ・保存則・
            // 有界性を壊さない値として採用・削除済み）。
            fan_count: 2,
            fan_spread: 0.35,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NetState {
    pub tick: u64,
    pub nodes: Vec<NNode>,
    pub edges: Vec<NEdge>,
    pub free_budget: f64,
    /// 現在アクティブな前線ノード id 集合（昇順を正準順序とする）。
    pub frontier: Vec<usize>,

    // 砂糖源（id 昇順を正準順序として維持。現行 Op データを流用）。
    pub sugar_id: Vec<u64>,
    pub sugar_x: Vec<f64>,
    pub sugar_y: Vec<f64>,
    pub sugar_strength: Vec<f64>,
    pub sugar_remaining: Vec<f64>,
    pub next_sugar_id: u64,

    // 保存則の帳簿: total_mass == collected_total - consumed_total
    pub collected_total: f64,
    pub consumed_total: f64,

    pub rng: Rng,
}

impl NetState {
    #[inline]
    pub fn n_nodes(&self) -> usize {
        self.nodes.len()
    }
    #[inline]
    pub fn n_edges(&self) -> usize {
        self.edges.len()
    }
}

/// 保存則の左辺: free_budget + Σ(d_e*l_e)（担体A）。
pub fn total_mass(s: &NetState) -> f64 {
    let mut sum = s.free_budget;
    for e in &s.edges {
        sum += e.d * e.l;
    }
    sum
}

/// 決定論的な初期 NetState。根（ノード0）はホーム座標に1つ、前線はそのノードのみ。
pub fn initial_net_state(seed: u64, world: &World, p: &NetParams) -> NetState {
    let rng = Rng::seed_from_u64(seed);
    let (hx, hy) = world.default_home(p.e_lo);
    NetState {
        tick: 0,
        nodes: vec![NNode { x: hx, y: hy }],
        edges: Vec::new(),
        free_budget: p.initial_budget,
        frontier: vec![0],
        sugar_id: Vec::new(),
        sugar_x: Vec::new(),
        sugar_y: Vec::new(),
        sugar_strength: Vec::new(),
        sugar_remaining: Vec::new(),
        next_sugar_id: 0,
        collected_total: p.initial_budget,
        consumed_total: 0.0,
        rng,
    }
}

/// 単一 op を適用（現行 `Op` データを流用。RemoveSugar は id 一致を削除）。
pub fn apply_net_op(state: &mut NetState, op: &Op) {
    match *op {
        Op::PlaceSugar { x, y, strength } => {
            let sid = state.next_sugar_id;
            state.next_sugar_id += 1;
            state.sugar_id.push(sid);
            state.sugar_x.push(x);
            state.sugar_y.push(y);
            state.sugar_strength.push(strength);
            state.sugar_remaining.push(strength);
        }
        Op::RemoveSugar { id } => {
            if let Some(pos) = state.sugar_id.iter().position(|&s| s == id) {
                remove_sugar_at(state, pos);
            }
        }
    }
}

fn remove_sugar_at(state: &mut NetState, pos: usize) {
    state.sugar_id.remove(pos);
    state.sugar_x.remove(pos);
    state.sugar_y.remove(pos);
    state.sugar_strength.remove(pos);
    state.sugar_remaining.remove(pos);
}

/// 残量0以下の砂糖源を id 昇順で決定論的に自動削除する（現行 core-003 と同趣旨）。
pub fn remove_depleted_sugar(state: &mut NetState) {
    let mut i = 0;
    while i < state.sugar_id.len() {
        if state.sugar_remaining[i] <= 0.0 {
            remove_sugar_at(state, i);
        } else {
            i += 1;
        }
    }
}
