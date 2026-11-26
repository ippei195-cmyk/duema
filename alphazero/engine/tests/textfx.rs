//! 効果文から作った手順（textfx）を、手で書いた処理と同じ場面で動かして、結果（各ゾーンのカード）が同じか確かめる。
use dmengine::cards::*;
use dmengine::effects::*;
use dmengine::engine::*;
use dmengine::textfx;

mod gold {
    include!("textfx_gold_progs.rs");
}

fn blank(d0: &str, d1: &str, seed: u64) -> Game {
    let mut g = Game::new(&deck(d0), &deck(d1), seed);
    for p in 0..2 {
        let q = &mut g.ps[p];
        let mut all: Vec<Card> = vec![];
        all.append(&mut q.hand);
        all.extend(q.mana.drain(..).map(|m| m.card));
        all.extend(q.shields.drain(..).map(|s| s.card));
        all.extend(q.battle.drain(..).map(|c| c.card));
        all.append(&mut q.grave);
        all.append(&mut q.tama);
        q.deck.extend(all);
    }
    g.tp = 0;
    g.turn = 5;
    g.stack.clear();
    g.decision = None;
    g
}

fn take(g: &mut Game, p: usize, c: C) -> Card {
    let i = g.ps[p].deck.iter().position(|x| x.c == c).unwrap();
    g.ps[p].deck.remove(i)
}

/// 選び方：一番大きい uid（並び順に左右されない）。はい／モード0（手札）
fn policy(d: &Decision) -> usize {
    let mut best = 0;
    let mut bu = -1i32;
    for (i, o) in d.opts.iter().enumerate() {
        let s = match o {
            Opt::Pick(u) => *u as i32 + 10,
            Opt::Yes => 5,
            Opt::Mode(0) => 4,
            _ => 0,
        };
        if s > bu {
            bu = s;
            best = i;
        }
    }
    best
}

fn settle(g: &mut Game) {
    g.run();
    let mut n = 0;
    while let Some(d) = g.decision.as_ref() {
        if d.kind == DK::Main {
            break;
        }
        let i = policy(d);
        g.step(i);
        n += 1;
        assert!(n < 100);
    }
}

fn snapshot(g: &Game) -> Vec<Vec<u16>> {
    let mut v = vec![];
    for p in 0..2 {
        let q = &g.ps[p];
        let mut z: Vec<Vec<u16>> = vec![
            q.hand.iter().map(|c| c.uid).collect(),
            q.mana.iter().map(|m| m.card.uid).collect(),
            q.battle.iter().map(|c| c.card.uid).collect(),
            q.grave.iter().map(|c| c.uid).collect(),
            q.shields.iter().map(|s| s.card.uid).collect(),
            q.deck.iter().map(|c| c.uid).collect(),
        ];
        for x in z.iter_mut() {
            x.sort();
        }
        v.extend(z);
    }
    v
}

fn with_gold<T>(c: C, ev: textfx::Ev, f: impl FnOnce() -> T) -> T {
    let pr = gold::PROGS.iter().find(|(x, e, _)| *x == c && *e == ev).expect("gold").2;
    textfx::TEST_PROGS.with(|t| t.borrow_mut().push((c, ev, pr)));
    let r = f();
    textfx::TEST_PROGS.with(|t| t.borrow_mut().clear());
    r
}

/// 呪文 c を唱えた場面（手書き／手順）の結果
fn cast_case(d: &str, c: C, seed: u64) -> Vec<Vec<u16>> {
    let mut g = blank(d, "hustle", seed);
    let card = take(&mut g, 0, c);
    g.ps[0].grave.push(card);
    g.stack.push(Task::MainStep);
    g.stack.push(Task::Fx(Fx::Cast { p: 0, uid: card.uid, side: 0, free: true }));
    settle(&mut g);
    snapshot(&g)
}

fn enter_case(d: &str, c: C, seed: u64, prep: fn(&mut Game)) -> Vec<Vec<u16>> {
    let mut g = blank(d, "hustle", seed);
    prep(&mut g);
    let card = take(&mut g, 0, c);
    g.stack.push(Task::MainStep);
    enter(&mut g, 0, card, How::Put, None, 0);
    settle(&mut g);
    snapshot(&g)
}

#[test]
fn kiyuri_same_as_hand_written() {
    for seed in 1..40 {
        let a = cast_case("leasebike", C::Kiyuri, seed);
        let b = with_gold(C::Kiyuri, textfx::Ev::Spell(0), || cast_case("leasebike", C::Kiyuri, seed));
        assert_eq!(a, b, "seed {seed}");
    }
}

#[test]
fn antiham_same_as_hand_written() {
    fn prep(g: &mut Game) {
        for c in [C::Caddy, C::PPP, C::RedZone] {
            let k = take(g, 0, c);
            g.ps[0].hand.push(k);
        }
    }
    for seed in 1..40 {
        let a = enter_case("leasebike", C::Antiham, seed, prep);
        let b = with_gold(C::Antiham, textfx::Ev::Etb, || enter_case("leasebike", C::Antiham, seed, prep));
        assert_eq!(a, b, "seed {seed}");
    }
}

#[test]
fn akane_same_as_hand_written() {
    for seed in 1..40 {
        let a = enter_case("gorgi", C::Akane, seed, |_| {});
        let b = with_gold(C::Akane, textfx::Ev::Etb, || enter_case("gorgi", C::Akane, seed, |_| {}));
        assert_eq!(a, b, "seed {seed}");
    }
}
