//! netphys_state_hash（決定性検証用の正準ハッシュ）。既存 src/hash.rs・tree/hash.rs の方式
//! （FNV-1a・量子化）を踏襲した新規実装。既存 hash.rs / tree/hash.rs は変更しない。

use super::state::{NetParams, NetState};

const FNV64_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV64_PRIME: u64 = 0x0000_0100_0000_01b3;

#[inline]
fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV64_PRIME);
    }
    h
}

#[inline]
fn quant(x: f64, q: f64) -> i64 {
    (x / q).round() as i64
}

#[inline]
fn feed_i64(h: u64, v: i64) -> u64 {
    fnv1a(h, &v.to_le_bytes())
}

#[inline]
fn feed_u64(h: u64, v: u64) -> u64 {
    fnv1a(h, &v.to_le_bytes())
}

/// 64bit の決定論ハッシュ。同一 NetState（量子化後）→同一値。正準順序: ノードindex昇順・
/// 辺は (a,b,d,l) で安定ソート・前線id昇順・砂糖id昇順・rng内部状態。
pub fn netphys_state_hash(s: &NetState, p: &NetParams) -> u64 {
    let mut h = FNV64_OFFSET;

    h = feed_u64(h, s.tick);
    h = feed_i64(h, quant(s.free_budget, p.q_vol));

    // ノード（index順=正準）
    h = feed_u64(h, s.nodes.len() as u64);
    for nd in &s.nodes {
        h = feed_i64(h, quant(nd.x, p.q_pos));
        h = feed_i64(h, quant(nd.y, p.q_pos));
    }

    // 辺: (a,b,d,l) で安定ソートして正準化（consolidation の drain/rebuild で順序が変わりうるため）。
    let mut edges: Vec<(usize, usize, i64, i64)> = s
        .edges
        .iter()
        .map(|e| (e.a, e.b, quant(e.d, p.q_vol), quant(e.l, p.q_vol)))
        .collect();
    edges.sort();
    h = feed_u64(h, edges.len() as u64);
    for (a, b, d, l) in edges {
        h = feed_u64(h, a as u64);
        h = feed_u64(h, b as u64);
        h = feed_i64(h, d);
        h = feed_i64(h, l);
    }

    // 前線（id昇順に正準化）
    let mut frontier = s.frontier.clone();
    frontier.sort_unstable();
    h = feed_u64(h, frontier.len() as u64);
    for f in frontier {
        h = feed_u64(h, f as u64);
    }

    // 帳簿
    h = feed_i64(h, quant(s.collected_total, p.q_vol));
    h = feed_i64(h, quant(s.consumed_total, p.q_vol));

    // 砂糖源（id 昇順に正準化）
    let mut order: Vec<usize> = (0..s.sugar_id.len()).collect();
    order.sort_by_key(|&i| s.sugar_id[i]);
    h = feed_u64(h, order.len() as u64);
    for &i in &order {
        h = feed_u64(h, s.sugar_id[i]);
        h = feed_i64(h, quant(s.sugar_x[i], p.q_pos));
        h = feed_i64(h, quant(s.sugar_y[i], p.q_pos));
        h = feed_i64(h, quant(s.sugar_strength[i], p.q_vol));
        h = feed_i64(h, quant(s.sugar_remaining[i], p.q_vol));
    }

    // rng 内部状態
    for word in s.rng.state() {
        h = feed_u64(h, word);
    }

    h
}
