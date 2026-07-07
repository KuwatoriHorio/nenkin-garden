//! シミュレーションパラメータ（設計メモ §6）。全調整値をここに集約（ハードコード禁止）。
//! 既定値は「まず正しく動く」保守的な値。数値最適化は後続タスク。

#[derive(Clone, Copy, Debug)]
pub struct Params {
    // 場のサイズ
    pub h: usize,
    pub w: usize,

    // 感知（§5-1）
    pub sensor_dist: f64,
    pub sensor_angle: f64,
    pub w_e: f64,        // 感知の標高重み（忌避①）
    pub sea_penalty: f64,

    // 旋回・前進（§5-2,3）
    pub turn_speed: f64,
    pub step_size: f64,
    pub k_slope: f64,    // 傾斜コスト（忌避②）

    // 定着（§5-4）
    pub deposit: f64,
    pub e_hi: f64,       // 居住性しきい
    pub habit_top: f64,  // H=0 に落ちる上端（忌避③）
    pub e_lo: f64,       // 低標高帯しきい（メトリクス用）

    // 拡散・減衰（§5-7）
    pub diffuse_rate: f64,
    pub decay_base: f64,
    pub decay_high: f64, // 高標高の減衰（忌避④）

    // 砂糖・成長（§5-5,6）
    pub sugar_radius: f64,
    pub collect_rate: f64,
    pub sugar_beacon: f64,
    pub sugar_beacon_radius: f64, // ビーコンを撒く半径（0=単一セル, >0=小ブロブで源を網に埋め込む）
    pub agent_cap_base: f64,
    pub agent_cap_slope: f64,
    pub agent_cap_max: usize,
    pub spawn_per_tick: usize,
    pub spawn_cost: f64,
    pub spawn_jitter: f64,

    // 初期状態
    pub n_init_agents: usize,
    pub initial_biomass: f64,

    // --- core-002: ホーム凝集スタート＋誘引物質勾配コホージョン（すべて既定は現行挙動）---
    pub home_x: f64,             // ホーム座標。負(=-1)で「auto=World から低標高陸セルを算出」。
    pub home_y: f64,
    pub init_cluster_sigma: f64, // 0=一様散布（従来）。>0=ホーム周りの標準偏差で凝集配置。
    pub w_trail_cohesion: f64,   // 0=従来。>0=trailが下がる向きへの前進をソフト抑制（p_move>0は保つ）。

    // メトリクスしきい（§8）
    pub theta_cov: f64,
    pub theta_cc: f64,
    pub cc_connectivity: u8, // 4 or 8

    // 決定性: 量子化幅（§9）
    pub q_pos: f64,
    pub q_trail: f64,
    pub q_bio: f64,

    // テスト側定数（§10）
    pub warmup_ticks: u64,
    pub eps_conserve: f64,

    // --- analysis-001（効率ネットワーク解析）---
    // 網の二値化しきいは core の連結成分と整合させるため theta_cc を共有する。
    pub net_alpha: f64,         // 実効長 L_eff = L*(1+alpha*meanE) の標高加重
    pub skeleton_max_iter: usize, // 細線化の安全上限
    pub tap_min_len: f64,       // 砂糖源→ノードの tap 実効長の下限
    pub tap_radius: f64,        // 砂糖源が tap する近傍ノードの半径（過剰連結防止のため小さめ）
}

impl Default for Params {
    fn default() -> Self {
        Params {
            h: 96,
            w: 96,

            sensor_dist: 4.0,
            sensor_angle: 0.4,
            w_e: 0.6,
            sea_penalty: 1.0e9,

            turn_speed: 0.35,
            step_size: 1.0,
            k_slope: 3.0,

            deposit: 1.0,
            e_hi: 0.6,
            habit_top: 0.85,
            e_lo: 0.3,

            diffuse_rate: 0.18,
            decay_base: 0.02,
            decay_high: 0.20,

            sugar_radius: 3.0,
            collect_rate: 0.5,
            sugar_beacon: 6.0,
            // core-001: 砂糖源を半径3の小ブロブで撒き、源を周囲網に埋め込む。
            // 近接2源のブロブが拡散で融合し、代表シナリオで網が連結する（S9 で 9/9, analysis-003 前提）。
            sugar_beacon_radius: 3.0,
            agent_cap_base: 60.0,
            agent_cap_slope: 4.0,
            agent_cap_max: 4000,
            spawn_per_tick: 8,
            spawn_cost: 0.15,
            spawn_jitter: 1.5,

            // 初期エージェント数はバイオマス由来の上限(agent_cap)より小さくし、
            // 砂糖回収→バイオマス増→上限増→スポーンの成長ループが実際に駆動するようにする。
            n_init_agents: 80,
            initial_biomass: 5.0,

            // core-002: 既定は現行挙動（sigma=0 で一様散布, cohesion=0 で移動抑制なし）。
            home_x: -1.0,
            home_y: -1.0,
            init_cluster_sigma: 0.0,
            w_trail_cohesion: 0.0,

            theta_cov: 0.05,
            theta_cc: 0.05,
            cc_connectivity: 8,

            q_pos: 1.0e-4,
            q_trail: 1.0e-4,
            q_bio: 1.0e-6,

            warmup_ticks: 40,
            eps_conserve: 1.0e-4,

            net_alpha: 1.0,
            skeleton_max_iter: 1000,
            tap_min_len: 0.5,
            tap_radius: 4.0,
        }
    }
}
