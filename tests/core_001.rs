//! core-001 受け入れテスト（tasks/task-core-001.md）。
//! ビーコン小半径化（既定 sugar_beacon_radius=3）で代表シナリオの2砂糖源が連結する。
//! 過学習を避けるため正準9シードで頑健に評価（analysis-003 の flow ソルバ修正が前提）。

use nenkin_garden::analysis::analyze;
use nenkin_garden::params::Params;
use nenkin_garden::state::{Op, ScriptEntry};
use nenkin_garden::world::make_synthetic_archipelago;
use nenkin_garden::run_headless;

const S9: [u64; 9] = [1, 7, 13, 42, 99, 256, 1337, 2024, 31337];
const TICKS: u64 = 160;

#[test]
fn sugar_sources_connect_robustly() {
    let p = Params::default();
    let w = make_synthetic_archipelago(&p);
    let mut cells: Vec<usize> = (0..w.h * w.w)
        .filter(|&i| w.land_mask[i] && (w.e[i] as f64) < p.e_lo)
        .collect();
    if cells.is_empty() {
        cells = (0..w.h * w.w).filter(|&i| w.land_mask[i]).collect();
    }
    let c = cells[cells.len() / 2];
    let (sx, sy) = ((c % w.w) as f64 + 0.5, (c / w.w) as f64 + 0.5);
    let script = vec![
        ScriptEntry { tick: 0, op: Op::PlaceSugar { x: sx, y: sy, strength: 400.0 } },
        ScriptEntry { tick: 5, op: Op::PlaceSugar { x: sx + 6.0, y: sy + 4.0, strength: 300.0 } },
    ];

    let mut connected = 0;
    for &seed in &S9 {
        let r = run_headless(seed, &script, TICKS, &p, Some(&w));
        let a = analyze(&r.final_state, &w, &p).metrics;
        if a.flow_connected {
            connected += 1;
            assert!(a.total_conductance > 0.0, "seed {seed}: conductance 非正");
            assert!(a.effective_resistance > 0.0, "seed {seed}: 実効抵抗 非正");
            assert!(
                a.transport_efficiency > 0.0 && a.transport_efficiency <= 1.0 + 1e-9,
                "seed {seed}: transport_efficiency 値域外 {}",
                a.transport_efficiency
            );
        }
    }
    // 頑健性（過学習防止）: 正準9シード中 8 以上で連結（実測 9/9）。
    assert!(connected >= 8, "flow_connected が 9 シード中 {} のみ（>=8 が必要）", connected);
}
