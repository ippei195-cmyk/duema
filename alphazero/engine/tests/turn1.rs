//! 「各ターンに１度、自分のドラゴンが出た時」（ギャイ＜まかしとき！＞・アーシュ・熊田すず）の場面テスト。
//! 根拠：QA 44212（カード1枚につき1回）、QA 41443（「自分のドラゴンが出た時」は自身が出た時も誘発）、QA 49213（前半だけ・後半だけ可、片方でも使えばそのターンは使えない）
use dmengine::cards::*;
use dmengine::effects::*;
use dmengine::engine::*;

fn blank() -> Game {
    let mut g = Game::new(&deck("hustle"), &deck("hustle"), 3);
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
    g.tp = 0;
    g.turn = 5;
    g.stack.clear();
    g.decision = None;
    g
}

fn take(g: &mut Game, p: usize, c: C) -> Card {
    if let Some(i) = g.ps[p].deck.iter().position(|x| x.c == c) {
        return g.ps[p].deck.remove(i);
    }
    let i = g.ps[p].mana.iter().position(|m| m.card.c == c).unwrap();
    g.ps[p].mana.remove(i).card
}

fn put_battle(g: &mut Game, p: usize, c: C) -> u16 {
    let card = take(g, p, c);
    let mut cr = g.new_cre(card);
    cr.sick = false;
    g.ps[p].battle.push(cr);
    card.uid
}

fn k_of(g: &Game) -> Option<K> {
    match &g.decision.as_ref()?.q {
        Q::Custom(e) => Some(e.k),
        _ => None,
    }
}

fn choose(g: &mut Game, o: Opt) {
    let d = g.decision.as_ref().expect("no decision");
    let i = d.opts.iter().position(|&x| x == o).unwrap_or_else(|| panic!("{o:?} not in {:?}", d.opts));
    g.step(i);
}

/// ドラゴンを効果で出して、誘発を解決するまで進める（答え方は f が決める。None で止める）
fn enter_dragon(g: &mut Game, c: C, mut f: impl FnMut(&Game, K) -> Opt) -> Vec<K> {
    let card = take(g, 0, c);
    g.decision = None;
    g.stack.clear();
    g.stack.push(Task::MainStep);
    enter(g, 0, card, How::Put, None, 0);
    g.run();
    let mut seen = vec![];
    while let Some(k) = k_of(g) {
        seen.push(k);
        let o = f(g, k);
        choose(g, o);
    }
    seen
}

fn gyai_count(seen: &[K]) -> usize {
    seen.iter().filter(|&&k| k == K::GyaiMill).count()
}

#[test]
fn gyai_triggers_on_its_own_entry() {
    let mut g = blank();
    let seen = enter_dragon(&mut g, C::Gyai, |_, k| if k == K::GyaiMill { Opt::Yes } else { Opt::Pass });
    assert_eq!(gyai_count(&seen), 1);
}

#[test]
fn two_gyai_each_once_per_turn() {
    let mut g = blank();
    put_battle(&mut g, 0, C::Gyai);
    put_battle(&mut g, 0, C::Gyai);
    let m0 = g.ps[0].mana.len();
    // 1体目のドラゴン：2体のギャイがそれぞれ使える（QA 44212）
    let seen = enter_dragon(&mut g, C::Gaiash, |_, k| if k == K::GyaiMill { Opt::Yes } else { Opt::Pass });
    assert_eq!(gyai_count(&seen), 2);
    assert_eq!(g.ps[0].mana.len(), m0 + 4);
    // 同じターンの2体目のドラゴン：どちらも使用済み
    let seen = enter_dragon(&mut g, C::Gaiash, |_, k| if k == K::GyaiMill { Opt::Yes } else { Opt::Pass });
    assert_eq!(gyai_count(&seen), 0);
    // 次のターンはまた使える
    g.turn += 2;
    let seen = enter_dragon(&mut g, C::DogiGyaku, |_, k| if k == K::GyaiMill { Opt::Yes } else { Opt::Pass });
    assert_eq!(gyai_count(&seen), 2);
}

#[test]
fn declined_gyai_can_be_used_on_next_dragon() {
    let mut g = blank();
    put_battle(&mut g, 0, C::Gyai);
    let seen = enter_dragon(&mut g, C::Gaiash, |_, k| if k == K::GyaiMill { Opt::No } else { Opt::Pass });
    assert_eq!(gyai_count(&seen), 1);
    let m0 = g.ps[0].mana.len();
    let seen = enter_dragon(&mut g, C::Gaiash, |_, k| if k == K::GyaiMill { Opt::Yes } else { Opt::Pass });
    assert_eq!(gyai_count(&seen), 1);
    assert_eq!(g.ps[0].mana.len(), m0 + 2);
}

#[test]
fn gyai_second_half_only_uses_it() {
    // 前半を使わず後半（マナのドラゴンを手札へ）だけ使う → そのターンはもう使えない（QA 49213）
    let mut g = blank();
    put_battle(&mut g, 0, C::Gyai);
    let d = take(&mut g, 0, C::DogiGyaku);
    g.ps[0].mana.push(Mana { card: d, tapped: false, facedown: false });
    let h0 = g.ps[0].hand.len();
    let seen = enter_dragon(&mut g, C::Gaiash, |g, k| match k {
        K::GyaiMill => Opt::No,
        _ => {
            let d = g.decision.as_ref().unwrap();
            *d.opts.iter().find(|o| matches!(o, Opt::Pick(_))).unwrap_or(&Opt::Pass)
        }
    });
    assert!(seen.contains(&K::GyaiReturn));
    assert!(g.ps[0].hand.iter().any(|c| c.uid == d.uid));
    assert!(g.ps[0].hand.len() >= h0 + 1);
    let seen = enter_dragon(&mut g, C::Gaiash, |_, _| Opt::Yes);
    assert_eq!(gyai_count(&seen), 0);
}

#[test]
fn suzu_each_card_once() {
    let mut g = blank();
    put_battle(&mut g, 0, C::Suzu);
    put_battle(&mut g, 0, C::Suzu);
    for _ in 0..5 {
        let c = g.ps[1].deck.pop().unwrap();
        g.ps[1].hand.push(c);
    }
    enter_dragon(&mut g, C::Gaiash, |_, _| Opt::Pass);
    assert_eq!(g.ps[1].hand.len(), 3); // 2体がそれぞれ1枚
    enter_dragon(&mut g, C::Gaiash, |_, _| Opt::Pass);
    assert_eq!(g.ps[1].hand.len(), 3);
}
