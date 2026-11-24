//! 対戦卓（index.html）の盤面から、このエンジンの局面を作り、相手AIの次の一手を返す。
//! 入力は i32 の並び（JS 側の dmaz パッチが作る）。AI はいつもプレイヤー0（手番）。
//!   [1, 手番数, チャージ済み, 先読み回数, 乱数の種,
//!    プレイヤー0・1 それぞれ ゾーン（手札・マナ・バトル・シールド・墓地・山札）ごとに
//!      枚数, 各カード [カード番号, 卓の番号, 印(1=タップ 2=召喚酔い 4=表向き 8=EXライフあり 16=このターン攻撃できない), (印8ならEXライフのシールドの卓の番号),
//!                      下の枚数, 下のカード [番号, 卓の番号]…]]
//!   シールドの「下のカード」は G城が要塞化しているシールド。
//! 出力：[種類(0=分からない 1=チャージ 2=使う 3=攻撃 4=終わる), 卓の番号, 面, 出し方, 攻撃先の卓の番号(-1=プレイヤー), 価値×1000, 支払うコスト]

use crate::az::Az;
use crate::cards::*;
use crate::engine::*;
use crate::net::Net;
use crate::rng::Rng;

pub struct Built {
    pub g: Game,
    pub tid: Vec<(u16, i32)>, // (エンジンの uid, 卓の番号)
}

pub fn build(inp: &[i32]) -> Option<(Built, u32, u64)> {
    let mut it = inp.iter().copied();
    let mut nx = || it.next();
    if nx()? != 1 { return None; }
    let turn = nx()?.max(1) as u32;
    let charged = nx()? != 0;
    let sims = nx()?.max(1) as u32;
    let seed = nx()? as u32 as u64;
    let all = all();
    let mut g = Game::new(&deck("leasebike"), &deck("leasebike"), seed);
    g.ps = [Player::default(), Player::default()];
    g.stack.clear();
    g.decision = None;
    g.winner = None;
    g.tp = 0;
    g.turn = turn;
    g.next_uid = 1;
    let mut tid: Vec<(u16, i32)> = vec![];
    let mut exl = vec![];
    for p in 0..2 {
        for z in 0..6 {
            let n = nx()?;
            for _ in 0..n {
                let ci = nx()?;
                let t = nx()?;
                let fl = nx()?;
                let ex = if fl & 8 != 0 { nx()? } else { -1 };
                let nu = nx()?;
                if ci < 0 || ci as usize >= NCARDS { return None; }
                let uid = g.next_uid; g.next_uid += 1;
                tid.push((uid, t));
                let card = Card { c: all[ci as usize], uid };
                let mut under = vec![];
                for _ in 0..nu {
                    let uc = nx()?; let ut = nx()?;
                    if uc < 0 || uc as usize >= NCARDS { return None; }
                    let u = g.next_uid; g.next_uid += 1;
                    tid.push((u, ut));
                    under.push(Card { c: all[uc as usize], uid: u });
                }
                let q = &mut g.ps[p];
                match z {
                    0 => q.hand.push(card),
                    1 => q.mana.push(Mana { card, tapped: fl & 1 != 0, facedown: false }),
                    2 => {
                        let mut c = g.new_cre(card);
                        c.tapped = fl & 1 != 0;
                        c.sick = fl & 2 != 0;
                        c.cant_attack = fl & 16 != 0;
                        c.under = under;
                        c.entered = if c.sick { turn } else { turn.saturating_sub(1) };
                        g.ps[p].battle.push(c);
                        if ex >= 0 { exl.push((p, g.ps[p].battle.len() - 1, ex)); }
                    }
                    3 => {
                        q.shields.push(Shield { card, faceup: fl & 4 != 0 });
                        for u in under { q.shields.push(Shield { card: u, faceup: false }); }
                    }
                    4 => q.grave.push(card),
                    _ => q.deck.push(card),
                }
            }
        }
    }
    for (p, i, t) in exl {
        if let Some(&(u, _)) = tid.iter().find(|x| x.1 == t) {
            if g.ps[p].shields.iter().any(|s| s.card.uid == u) { g.ps[p].battle[i].exlife = Some(u); }
        }
    }
    g.ps[0].charged = charged;
    g.stack.push(Task::TurnEnd);
    g.stack.push(Task::EndTriggers);
    g.stack.push(Task::AttackStep);
    g.stack.push(Task::MainStep);
    if !charged { g.stack.push(Task::ChargeStep); }
    g.run();
    Some((Built { g, tid }, sims, seed))
}

pub fn decide(inp: &[i32], net: Option<&Net>) -> Vec<i32> {
    let none = vec![0, -1, 0, 0, -1, 0, -1];
    let Some((b, sims, seed)) = build(inp) else { return none };
    let mut g = b.g;
    let t = |u: u16| b.tid.iter().find(|x| x.0 == u).map(|x| x.1).unwrap_or(-1);
    let mut rng = Rng::new(seed ^ 0xD00D);
    let az = Az { net, sims, cpuct: 1.5, noise: false };
    for _ in 0..8 {
        if g.winner.is_some() { return vec![4, -1, 0, 0, -1, 0, -1]; }
        let Some(d) = g.decision.clone() else { return none };
        if d.player != 0 { return none; }
        let r = az.search(&g, &mut rng);
        let i = crate::az::argmax(&r.pi, &mut rng);
        let v = (r.value * 1000.0) as i32;
        match d.opts[i] {
            Opt::Charge(u) => return vec![1, t(u), 0, 0, -1, v, -1],
            Opt::Play { uid, side, how } => {
                let c = g.ps[0].hand.iter().chain(g.ps[0].grave.iter()).find(|c| c.uid == uid).map(|c| c.c);
                let cost = c.map(|c| crate::effects::play_cost(&g, 0, c, side, how).0 as i32).unwrap_or(-1);
                return vec![2, t(uid), side as i32, how as i32, -1, v, if how == 2 { 0 } else { cost }];
            }
            Opt::Attack { att, tgt } => {
                let tg = match tgt { Tgt::Player => -1, Tgt::Cre(u) => t(u) };
                return vec![3, t(att), 0, 0, tg, v, -1];
            }
            Opt::Pass if d.kind == DK::Attack => return vec![4, -1, 0, 0, -1, v, -1],
            Opt::Pass if d.kind == DK::Charge || d.kind == DK::Main => g.step(i),
            _ => return none,
        }
    }
    none
}
