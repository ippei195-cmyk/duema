//! 裁定メモの場面テスト
use dmengine::cards::*;
use dmengine::effects::*;
use dmengine::engine::*;

/// 両者の場・手札・マナ・シールドを空にして、山札だけ残した局面を作る
fn blank(tp: usize) -> Game {
    let d0 = deck("jokers");
    let d1 = deck("leasebike");
    let mut g = Game::new(&d0, &d1, 1);
    for p in 0..2 {
        let q = &mut g.ps[p];
        let mut all: Vec<Card> = vec![];
        all.append(&mut q.hand);
        all.extend(q.mana.drain(..).map(|m| m.card));
        all.extend(q.shields.drain(..).map(|s| s.card));
        all.extend(q.battle.drain(..).map(|c| c.card));
        all.append(&mut q.grave);
        q.deck.extend(all);
    }
    g.tp = tp;
    g.stack.clear();
    g.decision = None;
    g
}

fn take(g: &mut Game, p: usize, c: C) -> Card {
    let i = g.ps[p].deck.iter().position(|x| x.c == c).unwrap_or_else(|| panic!("{c:?} not in deck"));
    g.ps[p].deck.remove(i)
}

/// 山札に無いカードを作る（相手デッキのカードを使う時）
fn make(g: &mut Game, c: C) -> Card {
    g.next_uid += 1;
    Card { c, uid: g.next_uid }
}

fn mana(g: &mut Game, p: usize, n: usize) {
    for _ in 0..n {
        let c = g.ps[p].deck.remove(0);
        g.ps[p].mana.push(Mana { card: c, tapped: false, facedown: false });
    }
}

fn put_battle(g: &mut Game, p: usize, card: Card) -> u16 {
    let mut c = g.new_cre(card);
    c.sick = false;
    c.entered = 0;
    g.ps[p].battle.push(c);
    card.uid
}

#[test]
fn ppp_sends_big_creature_to_mana() {
    // 相手の PP-「P」：マナ5枚でコスト7を出すとマナへ（PP-「P」は置換効果の持ち主が相手）
    let mut g = blank(0);
    let ppp = take(&mut g, 1, C::PPP);
    put_battle(&mut g, 1, ppp);
    mana(&mut g, 0, 5);
    let brand = take(&mut g, 0, C::Brand);
    let ok = enter(&mut g, 0, brand, How::Summon, None, 0);
    assert!(!ok);
    assert!(g.ps[0].mana.iter().any(|m| m.card.c == C::Brand));
}

#[test]
fn joe_cost_is_printed_cost() {
    // 201.2a：J・O・E で安く出してもコストは7。マナ6枚なら PP-「P」でマナへ
    let mut g = blank(0);
    let ppp = take(&mut g, 1, C::PPP);
    put_battle(&mut g, 1, ppp);
    mana(&mut g, 0, 6);
    let brand = take(&mut g, 0, C::Brand);
    g.ps[0].hand.push(brand);
    play_from_hand(&mut g, 0, brand.uid, 0, 1);
    assert!(g.find(0, brand.uid).is_none());
    assert!(g.ps[0].mana.iter().any(|m| m.card.c == C::Brand));
}

#[test]
fn hustle_stops_put_but_not_summon() {
    // ハッスル・キャッスル：召喚以外（効果で出す）はマナへ、召喚は止めない（Q&A 31590 / 48488）
    let mut g = blank(0);
    let h = make(&mut g, C::Hustle);
    g.ps[1].shields.push(Shield { card: h, faceup: true });
    mana(&mut g, 0, 9);
    let a = take(&mut g, 0, C::Magny);
    assert!(!enter(&mut g, 0, a, How::Put, None, 0));
    let b = take(&mut g, 0, C::Magny);
    assert!(enter(&mut g, 0, b, How::Summon, None, 0));
}

#[test]
fn revolution_change_fails_against_ppp() {
    // 701.26b：入れ替え先がマナへ行くなら入れ替えは行われず、元のクリーチャーが攻撃を続ける
    let mut g = blank(0);
    let ppp = take(&mut g, 1, C::PPP);
    put_battle(&mut g, 1, ppp);
    mana(&mut g, 0, 5);
    let att = take(&mut g, 0, C::Brand);
    let au = put_battle(&mut g, 0, att);
    let jg = take(&mut g, 0, C::Jogiragon);
    g.ps[0].hand.push(jg);
    g.stack.push(Task::Attack { att: au, tgt: Tgt::Player });
    g.run();
    // 革命チェンジの質問が出る
    let d = g.decision.clone().unwrap();
    let i = d.opts.iter().position(|o| *o == Opt::Pick(jg.uid)).expect("rev change offered");
    g.step(i);
    assert!(g.find(0, au).is_some(), "attacker stays");
    assert!(g.ps[0].hand.iter().any(|c| c.uid == jg.uid), "Jogiragon stays in hand");
}

#[test]
fn exlife_shield_goes_instead() {
    let mut g = blank(0);
    let m = take(&mut g, 0, C::Magny);
    assert!(enter(&mut g, 0, m, How::Summon, None, 0));
    assert_eq!(g.ps[0].shields.len(), 1);
    leave(&mut g, 0, m.uid, Dest::Grave, true);
    assert!(g.find(0, m.uid).is_some(), "Magny stays");
    assert_eq!(g.ps[0].shields.len(), 0);
}

#[test]
fn exlife_under_fujin_does_nothing() {
    // 相手のフウジン天＆ライジン天：相手ターン中はシールド化できない（Q&A 41417）
    let mut g = blank(0);
    let f = take(&mut g, 1, C::Fujin);
    put_battle(&mut g, 1, f);
    let m = take(&mut g, 0, C::Magny);
    assert!(enter(&mut g, 0, m, How::Summon, None, 0));
    assert_eq!(g.ps[0].shields.len(), 0);
    leave(&mut g, 0, m.uid, Dest::Grave, true);
    assert!(g.find(0, m.uid).is_none(), "leaves normally");
}

#[test]
fn faceup_gcastle_broken_goes_to_grave_and_is_not_a_break() {
    // 822.3・Q&A 49036：表向きのG城は墓地へ、ブレイクしたことにならない
    let mut g = blank(0);
    let h = make(&mut g, C::Hustle);
    g.ps[1].shields.push(Shield { card: h, faceup: true });
    g.attack_broke = false;
    shields_to_hand(&mut g, 1, vec![h.uid], true);
    assert!(g.ps[1].grave.iter().any(|c| c.uid == h.uid));
    assert!(!g.attack_broke);
}

#[test]
fn caddy_destroyed_goes_to_mana() {
    let mut g = blank(0);
    let c = take(&mut g, 1, C::Caddy);
    put_battle(&mut g, 1, c);
    destroy(&mut g, 1, c.uid, true);
    assert!(g.ps[1].mana.iter().any(|m| m.card.uid == c.uid));
}

#[test]
fn magnum_only_on_its_owners_turn() {
    let mut g = blank(1); // マグナムの持ち主（1）のターン
    let mg = take(&mut g, 1, C::Magnum);
    put_battle(&mut g, 1, mg);
    mana(&mut g, 0, 9);
    let a = take(&mut g, 0, C::Magny);
    assert!(!enter(&mut g, 0, a, How::FreeSummon, None, 0));
    assert!(g.ps[0].grave.iter().any(|x| x.uid == a.uid));
    g.tp = 0;
    let b = take(&mut g, 0, C::Magny);
    assert!(enter(&mut g, 0, b, How::Summon, None, 0));
}

#[test]
fn shacogail_wins_on_last_card() {
    let mut g = blank(0);
    let s = make(&mut g, C::Shacogail);
    put_battle(&mut g, 0, s);
    let keep = g.ps[0].deck.pop().unwrap();
    g.ps[0].grave.extend(g.ps[0].deck.drain(..));
    g.ps[0].deck.push(keep);
    g.draw1(0);
    assert_eq!(g.winner, Some(0));
}

#[test]
fn deck_out_loses() {
    let mut g = blank(0);
    let keep = g.ps[0].deck.pop().unwrap();
    g.ps[0].grave.extend(g.ps[0].deck.drain(..));
    g.ps[0].deck.push(keep);
    g.draw1(0);
    assert_eq!(g.winner, Some(1));
}
