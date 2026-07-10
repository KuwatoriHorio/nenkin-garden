//! tree-growth-001: α成長木モデル（space colonization, 全体予算 B）の State（設計メモ §2-α）。
//! 現行 Jones モデルの state.rs とは独立（並置・無関係）。
//!
//! ノードは木を成す（index=id・親子のみ・閉路なし）。根 = ホーム座標（parent=None）。
//! 体積は全体で1つの予算 `b_free`（保存量）。パス距離 `d_i = |pos_i - pos_parent|` は
//! 明示的に保持せず、都度ノード座標から算出する（座標こそが正準の状態）。

use crate::rng::Rng;
use crate::state::Op;
use crate::world::World;

/// 木のノード。index が id（正準順序）。根は parent=None。
#[derive(Clone, Debug)]
pub struct Node {
    pub parent: Option<usize>,
    pub x: f32,
    pub y: f32,
}

/// 新モデル専用パラメータ（現行 `Params` は汚さない）。既定値は探索実測で調整（実装者裁量）。
#[derive(Clone, Copy, Debug)]
pub struct TreeParams {
    // 場のサイズは world 由来（World を共有・読み取り利用のみ）。
    /// 単位長さあたりの基礎コスト（構造として保存される分＝Σ k*d_i の係数）。
    pub k: f64,
    /// 標高による cost 割増係数（標高が高いほど実効コスト増＝ソフト忌避）。
    pub c_elev: f64,
    /// 分岐（新ノード生成）1回あたりの固定コスト（consumed、構造には乗らない）。
    pub c_branch: f64,
    /// 1tick に b_free のうち成長へ配る割合。
    pub growth_rate: f64,
    /// 誘引方向の角度差がこれを超えると2クラスタに分けて分岐する（rad）。
    pub branch_angle_threshold: f64,
    /// tip 自身のパス距離がこれを超えると（分岐なしでも）新ノードを spawn して継続する。
    pub max_path_len: f64,
    /// 誘引を失った tip が毎tick 縮む距離（保存的にb_freeへ還元）。
    pub retreat_rate: f64,
    /// 1tick あたりの最大伸長距離（暴走オーバーシュート防止のソフト速度上限）。
    pub max_step_per_tick: f64,
    /// space colonization の誘引半径（この半径内の残量ありの砂糖のみ誘引点になる）。
    pub attract_radius: f64,
    /// 砂糖回収半径。
    pub sugar_radius: f64,
    /// 砂糖回収レート（1砂糖あたり1tickに回収できる上限）。
    pub collect_rate: f64,
    /// 初期予算 B0（= 初期 b_free = 初期 collected_total）。
    pub initial_budget: f64,
    /// ホーム決定に使う低標高帯しきい（`World::default_home` 用）。
    pub e_lo: f64,
    /// prune 判定用の微小しきい（d<=eps で除去）。
    pub prune_eps: f64,
    /// state_hash の位置量子化幅。
    pub q_pos: f64,
    /// state_hash の体積/距離量子化幅。
    pub q_vol: f64,
    /// ランダム探索方向の重み（tree-growth-002）。0.0=探索オフ（既定・現行挙動と完全同一）。
    /// >0 で `dir = normalize(w_rand*rand_unit + Σ attractor_weight_i*attractor_dir_i)` に
    /// ブレンドし、誘引の無い tip も探索的に伸長する。
    pub w_rand: f64,
    /// 探索方向の持続性（0..1）。tip の直前の進行方向（親→tipベクトル）と新規ランダム方向を
    /// この重みで混ぜ、滑らかな彷徨いにする（純ジッタでなく相関ランダムウォーク）。
    /// `w_rand==0.0` のときは参照されない。
    /// **0.5 を超えないこと**: 直前方向の重みが新規ランダム方向の重みを上回ると、tip が海方向で
    /// 動けなくなった場合に（位置が更新されないため直前方向が固定され続け）ブレンド方向の取り得る
    /// 角度範囲が狭い扇形に限定され、その扇形がまるごと海だと**恒久的にデッドロック**する
    /// （実測で確認済み）。0.5 以下なら2ベクトルの重み付き和が全方位を連続的に取り得るため、
    /// 十分な乱数試行で必ず陸方向を再発見できる（保存則の「尽きたら止まる」＝予算律速のみを許容）。
    pub explore_persistence: f64,
}

impl Default for TreeParams {
    fn default() -> Self {
        TreeParams {
            k: 1.0,
            c_elev: 2.2,
            c_branch: 0.15,
            growth_rate: 0.6,
            branch_angle_threshold: 0.9, // ~51.6度
            max_path_len: 10.0,
            retreat_rate: 0.25,
            max_step_per_tick: 1.5,
            attract_radius: 48.0,
            sugar_radius: 3.0,
            collect_rate: 0.5,
            initial_budget: 250.0,
            e_lo: 0.3,
            prune_eps: 1.0e-6,
            q_pos: 1.0e-4,
            q_vol: 1.0e-4,
            w_rand: 0.0,
            explore_persistence: 0.45,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TreeState {
    pub tick: u64,
    pub nodes: Vec<Node>,
    pub b_free: f64,

    // 砂糖源（id 昇順を正準順序として維持。現行 Op データを流用）。
    pub sugar_id: Vec<u64>,
    pub sugar_x: Vec<f64>,
    pub sugar_y: Vec<f64>,
    pub sugar_strength: Vec<f64>,
    pub sugar_remaining: Vec<f64>,
    pub next_sugar_id: u64,

    // 保存則の帳簿: total_volume == collected_total - consumed_total
    pub collected_total: f64,
    pub consumed_total: f64,

    pub rng: Rng,
}

impl TreeState {
    #[inline]
    pub fn n_nodes(&self) -> usize {
        self.nodes.len()
    }
}

/// ノード i の親までのパス距離（根は0）。
#[inline]
pub fn path_len(state: &TreeState, i: usize) -> f64 {
    match state.nodes[i].parent {
        None => 0.0,
        Some(par) => {
            let dx = state.nodes[i].x as f64 - state.nodes[par].x as f64;
            let dy = state.nodes[i].y as f64 - state.nodes[par].y as f64;
            (dx * dx + dy * dy).sqrt()
        }
    }
}

/// 保存則の左辺: b_free + Σ(k*d_i)。
pub fn total_volume(state: &TreeState, k: f64) -> f64 {
    let mut sum = state.b_free;
    for i in 0..state.nodes.len() {
        sum += k * path_len(state, i);
    }
    sum
}

/// 決定論的な初期 TreeState。根はホーム座標に1ノードのみ（parent=None）。
pub fn initial_tree_state(seed: u64, world: &World, p: &TreeParams) -> TreeState {
    let rng = Rng::seed_from_u64(seed);
    let (hx, hy) = world.default_home(p.e_lo);
    TreeState {
        tick: 0,
        nodes: vec![Node {
            parent: None,
            x: hx as f32,
            y: hy as f32,
        }],
        b_free: p.initial_budget,
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
pub fn apply_tree_op(state: &mut TreeState, op: &Op) {
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

fn remove_sugar_at(state: &mut TreeState, pos: usize) {
    state.sugar_id.remove(pos);
    state.sugar_x.remove(pos);
    state.sugar_y.remove(pos);
    state.sugar_strength.remove(pos);
    state.sugar_remaining.remove(pos);
}

/// 残量0以下の砂糖源を id 昇順で決定論的に自動削除する（現行 core-003 と同趣旨）。
pub fn remove_depleted_sugar(state: &mut TreeState) {
    let mut i = 0;
    while i < state.sugar_id.len() {
        if state.sugar_remaining[i] <= 0.0 {
            remove_sugar_at(state, i);
        } else {
            i += 1;
        }
    }
}
