//! ネットへの入力（見えている情報だけ）。p は判断するプレイヤー。

use crate::cards::*;
use crate::effects::K;
use crate::engine::*;

const NC: usize = NCARDS;
/// 効果の質問の種類のための枠
pub const KN: usize = 100;
/// 判断の種類の特徴の始まり
pub const DBASE: usize = 9 * NC + 40;
pub const S: usize = 768; // 局面の特徴の数（DBASE + 115 以上）
pub const O: usize = 192; // 選択肢の特徴の数
const _: () = assert!(DBASE + 8 + KN + 7 <= S);
// 選択肢の特徴の位置
pub const OC: usize = 10; // 札の種類 OC..OC+NC+1
pub const OZ: usize = OC + NC + 1; // ゾーン 12
pub const OP: usize = OZ + 12; // タップ・パワー
pub const OPL: usize = OP + 2; // プレイヤーを攻撃
pub const OT: usize = OPL + 1; // 攻撃先の札 NC+1
pub const OTP: usize = OT + NC + 1;
pub const OS: usize = OTP + 2; // 面
pub const OH: usize = OS + 1; // 出し方 8
pub const OGC: usize = OH + 8; // グランドクロスの軽減
pub const OM: usize = OGC + 1; // モード 16
const _: () = assert!(OM + 16 <= O);

pub fn state(g: &Game, p: usize) -> Vec<f32> {
    let mut f = vec![0f32; S];
    let op = 1 - p;
    let me = &g.ps[p];
    let en = &g.ps[op];
    let mut add = |blk: usize, c: C, v: f32| f[blk * NC + c as usize] += v;
    for c in &me.hand { add(0, c.c, 0.25); }
    for m in &me.mana { add(1, m.card.c, 0.25); }
    for c in &me.battle { add(2, c.card.c, 0.5); for u in &c.under { add(2, u.c, 0.25); } }
    for c in &me.tama { add(2, c.c, 0.5); }
    for c in &me.grave { add(3, c.c, 0.25); }
    for s in &me.shields { if s.faceup { add(4, s.card.c, 0.5); } }
    for m in &en.mana { add(5, m.card.c, 0.25); }
    for c in &en.battle { add(6, c.card.c, 0.5); for u in &c.under { add(6, u.c, 0.25); } }
    for c in &en.tama { add(6, c.c, 0.5); }
    for c in &en.grave { add(7, c.c, 0.25); }
    for s in &en.shields { if s.faceup { add(8, s.card.c, 0.5); } }
    let mut i = 9 * NC;
    let mut put = |v: f32| { f[i] = v; i += 1; };
    let untap = |q: &Player| q.mana.iter().filter(|m| !m.tapped).count() as f32;
    put(me.deck.len() as f32 / 40.0);
    put(me.shields.len() as f32 / 5.0);
    put(me.hand.len() as f32 / 10.0);
    put(untap(me) / 10.0);
    put(me.mana.len() as f32 / 10.0);
    put(en.deck.len() as f32 / 40.0);
    put(en.shields.len() as f32 / 5.0);
    put(en.hand.len() as f32 / 10.0);
    put(en.mana.len() as f32 / 10.0);
    put(untap(en) / 10.0);
    put((g.turn as f32 / 40.0).min(2.0));
    put((g.tp == p) as u8 as f32);
    put(me.charged as u8 as f32);
    for q in [me, en] {
        put(q.battle.iter().filter(|c| c.tapped).count() as f32 / 5.0);
        put(q.battle.iter().filter(|c| c.sick).count() as f32 / 5.0);
        put(q.battle.iter().filter(|c| c.hyper).count() as f32 / 3.0);
        put(q.battle.iter().filter(|c| c.no_untap || c.lock || c.cant_attack).count() as f32 / 3.0);
        put(q.battle.iter().filter(|c| c.jd).count() as f32 / 3.0);
        put(q.battle.iter().filter(|c| c.exlife.is_some()).count() as f32 / 3.0);
        put(q.debt as f32 / 3.0);
        put(q.no_lose as u8 as f32);
        put((q.spell_lock_until >= g.turn) as u8 as f32);
        put(q.creature_limit_next_turn as u8 as f32);
        put(q.shields.iter().filter(|s| s.faceup).count() as f32 / 2.0);
    }
    let pw = |pp: usize| (0..g.ps[pp].battle.len()).map(|k| g.power(pp, k).max(0) as f32).sum::<f32>() / 50000.0;
    put(pw(p));
    put(pw(op));
    put(g.attack_broke as u8 as f32);
    put((me.small_lock_turn == g.turn) as u8 as f32);
    put(crate::effects::gc_left_pub(g, p) as f32 / 2.0);
    // 判断の種類
    let base = DBASE;
    if let Some(d) = &g.decision {
        f[base + d.kind as usize] = 1.0;
        match &d.q {
            Q::Custom(e) => f[base + 8 + (e.k as usize).min(KN - 1)] = 1.0,
            Q::Shield { left, picked, .. } => {
                f[base + 8 + KN] = 1.0;
                f[base + 9 + KN] = (*left as f32 - picked.len() as f32) / 3.0;
            }
            Q::UseTrigger { what, .. } => f[base + 10 + KN + (*what as usize).min(4)] = 1.0,
            Q::None => {}
        }
    }
    f
}

/// uid の札の (種類, 持ち主から見たゾーン)
fn locate(g: &Game, p: usize, uid: u16) -> (usize, usize, Option<(usize, usize)>) {
    for (k, pp) in [p, 1 - p].into_iter().enumerate() {
        let q = &g.ps[pp];
        if let Some(c) = q.hand.iter().find(|c| c.uid == uid) { return (c.c as usize, k * 6, None); }
        if let Some(i) = q.battle.iter().position(|c| c.card.uid == uid) { return (q.battle[i].card.c as usize, k * 6 + 1, Some((pp, i))); }
        if let Some(c) = q.tama.iter().find(|c| c.uid == uid) { return (c.c as usize, k * 6 + 1, None); }
        if let Some(m) = q.mana.iter().find(|m| m.card.uid == uid) { return (m.card.c as usize, k * 6 + 2, None); }
        if let Some(s) = q.shields.iter().find(|s| s.card.uid == uid) {
            let c = if s.faceup || pp == p && false { s.card.c as usize } else { NC };
            return (c, k * 6 + 3, None);
        }
        if let Some(c) = q.grave.iter().find(|c| c.uid == uid) { return (c.c as usize, k * 6 + 4, None); }
        if let Some(c) = q.deck.iter().find(|c| c.uid == uid) { return (c.c as usize, k * 6 + 5, None); }
    }
    (NC, 0, None)
}

pub fn option(g: &Game, p: usize, o: Opt) -> Vec<f32> {
    let mut f = vec![0f32; O];
    let (t, main, tgt, side, how, mode): (usize, Option<u16>, Option<Tgt>, u8, u8, Option<u8>) = match o {
        Opt::Pass => (0, None, None, 0, 0, None),
        Opt::Yes => (1, None, None, 0, 0, None),
        Opt::No => (2, None, None, 0, 0, None),
        Opt::Charge(u) => (3, Some(u), None, 0, 0, None),
        Opt::Play { uid, side, how } => (4, Some(uid), None, side, how, None),
        Opt::Attack { att, tgt } => (5, Some(att), Some(tgt), 0, 0, None),
        Opt::Block(u) => (6, Some(u), None, 0, 0, None),
        Opt::Pick(u) => (7, Some(u), None, 0, 0, None),
        Opt::Mode(m) => (8, None, None, 0, 0, Some(m)),
    };
    f[t] = 1.0;
    let pow = |pi: Option<(usize, usize)>, at: usize, f: &mut Vec<f32>| {
        if let Some((pp, i)) = pi {
            f[at] = g.ps[pp].battle[i].tapped as u8 as f32;
            f[at + 1] = g.power(pp, i).max(0) as f32 / 20000.0;
        }
    };
    if let Some(u) = main {
        let (c, z, pi) = locate(g, p, u);
        f[OC + c] = 1.0;
        f[OZ + z] = 1.0;
        pow(pi, OP, &mut f);
    }
    match tgt {
        Some(Tgt::Player) => f[OPL] = 1.0,
        Some(Tgt::Cre(u)) => {
            let (c, _, pi) = locate(g, p, u);
            f[OT + c] = 1.0;
            pow(pi, OTP, &mut f);
        }
        None => {}
    }
    f[OS] = side as f32;
    f[OH + (how & 7) as usize] = 1.0;
    f[OGC] = (how & crate::effects::HOW_GC != 0) as u8 as f32;
    if let Some(m) = mode {
        f[OM + (m as usize).min(15)] = 1.0;
    }
    f
}

pub fn k_count() -> usize {
    K::ChooseRepl as usize + 1
}
