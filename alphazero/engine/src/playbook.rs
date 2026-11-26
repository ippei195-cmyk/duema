//! note の考え方（/mnt/project-files/meta-decks/playbooks/）を学習に入れる。
//! 1. 方策の事前バイアス：weights() が選択肢ごとの重み（1=ふつう）を返し、Az の事前確率に掛ける（強さは set_strength、既定0＝使わない）
//! 2. 開始局面：scenario() が「決めパターンの直前」の局面を作る（selfplay の引数で混ぜる）
//! 出典：ギリガザミ＝かいこちゃん note（一平購入済み）、創世竜＝JO「創世竜ループ一覧」。

use crate::cards::*;
use crate::effects::{cost_of, K};
use crate::engine::*;
use crate::rng::Rng;
use std::sync::atomic::{AtomicU32, Ordering};

static STRENGTH: AtomicU32 = AtomicU32::new(0);

/// 事前バイアスの強さ（0=使わない、1=重みをそのまま掛ける）
pub fn set_strength(s: f32) {
    STRENGTH.store(s.to_bits(), Ordering::Relaxed);
}

pub fn strength() -> f32 {
    f32::from_bits(STRENGTH.load(Ordering::Relaxed))
}

/// 事前確率に重みを掛けて正規化し直す（強さ λ：w' = 1 + λ(w-1)）
pub fn apply(g: &Game, prior: &mut [f32]) {
    let lam = strength();
    if lam <= 0.0 {
        return;
    }
    let Some(d) = g.decision.as_ref() else { return };
    let Some(w) = weights(g, d) else { return };
    let mut z = 0.0;
    for (p, w) in prior.iter_mut().zip(w.iter()) {
        *p *= (1.0 + lam * (w - 1.0)).max(0.01);
        z += *p;
    }
    if z > 0.0 {
        for p in prior.iter_mut() {
            *p /= z;
        }
    }
}

fn card_anywhere(g: &Game, p: usize, uid: u16) -> Option<C> {
    let q = &g.ps[p];
    q.hand.iter().chain(q.grave.iter()).chain(q.tama.iter()).chain(q.deck.iter()).find(|c| c.uid == uid).map(|c| c.c)
        .or_else(|| q.mana.iter().find(|m| m.card.uid == uid).map(|m| m.card.c))
        .or_else(|| q.battle.iter().find(|x| x.card.uid == uid).map(|x| x.card.c))
        .or_else(|| q.shields.iter().find(|s| s.card.uid == uid).map(|s| s.card.c))
}

fn in_hand(g: &Game, p: usize, c: C) -> bool {
    g.ps[p].hand.iter().any(|h| h.c == c)
}

/// スーパー・S・トリガーの「ストック」（シールドに仕込んだギリガザミ・グリモア）の数（自分は中身を知っている扱い）
fn stocks(g: &Game, p: usize) -> usize {
    g.ps[p].shields.iter().filter(|s| matches!(s.card.c, C::Giri | C::Grimoire)).count()
}

/// 選択肢ごとの重み。どの決まりにも当てはまらなければ None
pub fn weights(g: &Game, d: &Decision) -> Option<Vec<f32>> {
    let p = d.player;
    let op = 1 - p;
    let me = &g.ps[p];
    let mut w = vec![1.0f32; d.opts.len()];
    let mut any = false;
    let mut set = |i: usize, v: f32, any: &mut bool| {
        w[i] *= v;
        *any = true;
    };
    match (&d.kind, &d.q) {
        // ---- チャージ ----
        (DK::Charge, _) => {
            for (i, o) in d.opts.iter().enumerate() {
                let Opt::Charge(u) = *o else { continue };
                let Some(c) = card_anywhere(g, p, u) else { continue };
                match c {
                    // ギリガザミ：決めに使う札はマナに置かない。序盤は多色を置いて色を作る（逆アポロ擬態）
                    C::Shinkirou | C::Petens | C::Giri | C::Grimoire | C::Shacogail => set(i, 0.3, &mut any),
                    C::Kakugo | C::Risou | C::Dolma if me.mana.len() < 3 => set(i, 2.0, &mut any),
                    // 創世竜：創世竜は場に出したい。タマシードはマナから実行できるので置いてよい
                    C::Drache if !me.battle.iter().any(|x| x.card.c == C::Drache) => set(i, 0.4, &mut any),
                    C::Pearly | C::Trap | C::Levya | C::Atlantis | C::Scorpion if me.battle.iter().any(|x| x.card.c == C::Drache) => set(i, 1.5, &mut any),
                    _ => {}
                }
            }
        }
        // ---- メイン ----
        (DK::Main, _) => {
            let drache = me.battle.iter().filter(|x| x.card.c == C::Drache).count();
            for (i, o) in d.opts.iter().enumerate() {
                let Opt::Play { uid, side, how } = *o else { continue };
                let Some(c) = card_anywhere(g, p, uid) else { continue };
                match c {
                    // エターナル・ブレイン（上面）は2ターン目には使わない
                    C::EBrain if side == 0 && me.mana.len() <= 2 => set(i, 0.3, &mut any),
                    // 墓地にペテンズがあれば真気楼（実行モードで暴発）
                    C::Shinkirou if me.grave.iter().any(|x| x.c == C::Petens) => set(i, 3.0, &mut any),
                    // ストックがあればペテンズで暴発
                    C::Petens if stocks(g, p) >= 1 => set(i, 3.0, &mut any),
                    C::Petens => set(i, 0.5, &mut any),
                    // 4ターン目の理想と平和
                    C::Risou if me.mana.len() >= 4 => set(i, 1.5, &mut any),
                    // 創世竜：まず創世竜を2体、そろったらタマシード（マナからの実行を含む）
                    C::Drache if drache < 2 => set(i, 2.5, &mut any),
                    C::Pearly | C::Levya | C::Trap | C::Atlantis | C::Scorpion | C::Emeral if drache >= 1 => {
                        set(i, if how == 7 { 2.0 } else { 1.7 }, &mut any)
                    }
                    C::Alspa if drache >= 2 => set(i, 1.5, &mut any),
                    _ => {}
                }
            }
            if any {
                // 何もしない（Pass）は少し下げる
                if let Some(i) = d.opts.iter().position(|&o| o == Opt::Pass) {
                    w[i] *= 0.8;
                }
            }
        }
        // ---- 効果の中の選択 ----
        (DK::Effect, Q::Custom(e)) => {
            let pick_c = |o: &Opt| -> Option<C> {
                if let Opt::Pick(u) = *o { card_anywhere(g, p, u).or_else(|| card_anywhere(g, op, u)) } else { None }
            };
            match e.k {
                K::RisouMode => {
                    // 真気楼・ペテンズがそろっていなければ回収（Mode 0）、そろっていれば盾追加（Mode 1）
                    let ready = in_hand(g, p, C::Shinkirou) && (in_hand(g, p, C::Petens) || me.grave.iter().any(|x| x.c == C::Petens));
                    let want = if ready { 1 } else { 0 };
                    for (i, o) in d.opts.iter().enumerate() {
                        if *o == Opt::Mode(want) {
                            set(i, 3.0, &mut any);
                        }
                    }
                }
                K::ShinMode => {
                    // 墓地に S・トリガー（特にペテンズ）があれば実行モード
                    let pet = me.grave.iter().any(|x| x.c == C::Petens);
                    for (i, o) in d.opts.iter().enumerate() {
                        if *o == Opt::Mode(1) && pet {
                            set(i, 4.0, &mut any);
                        }
                    }
                }
                K::ShinTrig | K::GrimCast | K::GiriCast | K::DokinCast => {
                    for (i, o) in d.opts.iter().enumerate() {
                        let v = match pick_c(o) {
                            Some(C::Petens) if stocks(g, p) >= 1 || g.ps[p].shields.len() >= 2 => 4.0,
                            Some(C::LoveAbyss) if g.ps[op].shields.is_empty() => 4.0,
                            Some(C::Dolma) => 1.5,
                            Some(C::Grimoire) | Some(C::Kakugo) => 1.3,
                            _ => 1.0,
                        };
                        if v != 1.0 {
                            set(i, v, &mut any);
                        }
                    }
                }
                K::DracheUnder => {
                    // 創世竜の下に敷く（ループの基本）
                    for (i, o) in d.opts.iter().enumerate() {
                        if *o != Opt::Pass {
                            set(i, 3.0, &mut any);
                        }
                    }
                }
                K::TrapPick => {
                    // トラップ：全部敷いた創世竜を選ぶ（下のカードがマナに戻って初期盤面へ）
                    for (i, o) in d.opts.iter().enumerate() {
                        if let Opt::Pick(u) = *o {
                            if let Some(x) = g.ps[p].battle.iter().find(|x| x.card.uid == u) {
                                if x.card.c == C::Drache && !x.under.is_empty() {
                                    set(i, 2.0 + x.under.len() as f32, &mut any);
                                }
                            }
                        }
                    }
                }
                K::PearlyReturn | K::LevyaPut => {
                    for (i, o) in d.opts.iter().enumerate() {
                        let v = match pick_c(o) {
                            Some(C::Levya) | Some(C::Pearly) | Some(C::Trap) => 2.5,
                            Some(C::Atlantis) | Some(C::Scorpion) => 1.5,
                            _ => 1.0,
                        };
                        if v != 1.0 {
                            set(i, v, &mut any);
                        }
                    }
                }
                K::AtlDestroy | K::AtlRevive => {
                    // 4以下のタマシードの出た時を使い回す（パーリ・トラップ・エメラル）
                    for (i, o) in d.opts.iter().enumerate() {
                        if matches!(pick_c(o), Some(C::Pearly) | Some(C::Trap) | Some(C::Emeral) | Some(C::GreatBlue)) {
                            set(i, 2.0, &mut any);
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    if any { Some(w) } else { None }
}

// ---------- 開始局面 ----------

fn pull(g: &mut Game, p: usize, c: C) -> Option<Card> {
    let q = &mut g.ps[p];
    if let Some(i) = q.deck.iter().position(|x| x.c == c) {
        return Some(q.deck.remove(i));
    }
    if let Some(i) = q.hand.iter().position(|x| x.c == c) {
        return Some(q.hand.remove(i));
    }
    if let Some(i) = q.shields.iter().position(|s| s.card.c == c) {
        return Some(q.shields.remove(i).card);
    }
    None
}

fn from_top(g: &mut Game, p: usize) -> Option<Card> {
    g.ps[p].deck.pop()
}

/// p のデッキが ギリガザミ か 創世竜 なら、決めパターンの直前の局面に並べ替える（相手はふつうに数ターン進んだ形）
pub fn scenario(g: &mut Game, p: usize, rng: &mut Rng) -> bool {
    let has = |g: &Game, c: C| g.ps[p].deck.iter().chain(g.ps[p].hand.iter()).chain(g.ps[p].shields.iter().map(|s| &s.card)).any(|x| x.c == c);
    let kind = if has(g, C::Giri) && has(g, C::Petens) {
        1
    } else if has(g, C::Drache) && has(g, C::Pearly) {
        2
    } else {
        return false;
    };
    let op = 1 - p;
    // 相手：マナ5枚・クリーチャー1〜2体
    for _ in 0..5 {
        if let Some(c) = from_top(g, op) {
            g.ps[op].mana.push(Mana { card: c, tapped: false, facedown: false });
        }
    }
    for _ in 0..(1 + rng.below(2)) {
        if let Some(i) = g.ps[op].deck.iter().rposition(|c| def(c.c).main.kind == Kind::Creature && def(c.c).evo == Evo::None && cost_of(c.c) <= 5) {
            let c = g.ps[op].deck.remove(i);
            let mut cr = g.new_cre(c);
            cr.sick = false;
            g.ps[op].battle.push(cr);
        }
    }
    if kind == 1 {
        // ギリガザミ：手札に真気楼、墓地にペテンズ、シールドにストック2つ（ギリガザミ・グリモア）、マナ6（光・水・闇）
        let mut put_sh = vec![];
        for c in [C::Giri, C::Grimoire] {
            if let Some(x) = pull(g, p, c) {
                put_sh.push(x);
            }
        }
        // シールドを入れ替え：ストックを入れて、余りは山札へ
        while g.ps[p].shields.len() + put_sh.len() > 3 {
            let s = g.ps[p].shields.pop().unwrap();
            g.ps[p].deck.insert(0, s.card);
        }
        for x in put_sh {
            g.ps[p].shields.push(Shield { card: x, faceup: false });
        }
        if let Some(x) = pull(g, p, C::Shinkirou) {
            g.ps[p].hand.push(x);
        }
        if let Some(x) = pull(g, p, C::Petens) {
            g.ps[p].grave.push(x);
        }
        for c in [C::Shinkirou, C::Risou, C::Kakugo, C::Petens, C::Dolma, C::Kurin] {
            if g.ps[p].mana.len() >= 6 {
                break;
            }
            if let Some(x) = pull(g, p, c) {
                g.ps[p].mana.push(Mana { card: x, tapped: false, facedown: false });
            }
        }
        while g.ps[p].mana.len() < 6 {
            let Some(c) = from_top(g, p) else { break };
            g.ps[p].mana.push(Mana { card: c, tapped: false, facedown: false });
        }
    } else {
        // 創世竜：場に創世竜2体、手札にアトランティス・スコーピオン・パーリ、マナにレヴィヤ・トラップ・パーリ＋3枚
        for _ in 0..2 {
            if let Some(x) = pull(g, p, C::Drache) {
                let mut cr = g.new_cre(x);
                cr.sick = false;
                g.ps[p].battle.push(cr);
            }
        }
        for c in [C::Atlantis, C::Scorpion, C::Pearly] {
            if let Some(x) = pull(g, p, c) {
                g.ps[p].hand.push(x);
            }
        }
        for c in [C::Levya, C::Trap, C::Pearly, C::Akane, C::Eringi, C::Chiryu] {
            if let Some(x) = pull(g, p, c) {
                g.ps[p].mana.push(Mana { card: x, tapped: false, facedown: false });
            }
        }
    }
    // 山札が少なくなりすぎないように、シールドは最低3つ
    while g.ps[p].shields.len() < 3 {
        let Some(c) = from_top(g, p) else { break };
        g.ps[p].shields.push(Shield { card: c, faceup: false });
    }
    // p のターンのはじめからやり直す
    g.decision = None;
    g.stack.clear();
    g.winner = None;
    g.tp = p;
    g.turn = 8 + rng.below(4) as u32;
    g.stack.push(Task::TurnStart);
    g.run();
    true
}
