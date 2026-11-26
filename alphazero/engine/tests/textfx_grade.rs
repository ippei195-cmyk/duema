//! ローカルAIなどが作った手順（tests/textfx_cand_progs.rs）を、手で書いた処理と同じ場面で動かして一致率を出す。
//! cargo test --release --test textfx_grade -- --nocapture
use dmengine::cards::*;
use dmengine::effects::*;
use dmengine::engine::*;
use dmengine::textfx;

mod cand {
    include!("textfx_cand_progs.rs");
}

const DECKS: [&str; 5] = ["leasebike", "hustle", "jokers", "willide", "gorgi"];

fn deck_with(c: C) -> &'static str {
    DECKS.iter().copied().find(|d| deck(d).contains(&c)).unwrap()
}

/// ふつうの場面：手札5・マナ6・シールド5・相手のバトルゾーンに2体
fn scene(c: C, seed: u64) -> (Game, Card) {
    let d0 = deck_with(c);
    let d1 = DECKS[(seed as usize) % 5];
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
        g.rng.shuffle(&mut all);
        g.ps[p].deck.extend(all);
    }
    let i = g.ps[0].deck.iter().position(|x| x.c == c).unwrap();
    let card = g.ps[0].deck.remove(i);
    for p in 0..2 {
        for _ in 0..5 {
            let k = g.ps[p].deck.pop().unwrap();
            g.ps[p].hand.push(k);
        }
        for _ in 0..6 {
            let k = g.ps[p].deck.pop().unwrap();
            g.ps[p].mana.push(Mana { card: k, tapped: false, facedown: false });
        }
        for _ in 0..5 {
            let k = g.ps[p].deck.pop().unwrap();
            g.ps[p].shields.push(Shield { card: k, faceup: false });
        }
        for _ in 0..3 {
            let k = g.ps[p].deck.pop().unwrap();
            g.ps[p].grave.push(k);
        }
    }
    // 相手のバトルゾーンにクリーチャー2体（山札から）
    for _ in 0..2 {
        if let Some(i) = g.ps[1].deck.iter().position(|x| def(x.c).main.kind == Kind::Creature && def(x.c).evo == Evo::None) {
            let k = g.ps[1].deck.remove(i);
            let mut cr = g.new_cre(k);
            cr.sick = false;
            g.ps[1].battle.push(cr);
        }
    }
    g.tp = 0;
    g.turn = 7;
    g.stack.clear();
    g.decision = None;
    (g, card)
}

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
        if d.kind == DK::Main || n > 200 {
            break;
        }
        let i = policy(d);
        g.step(i);
        n += 1;
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
            q.battle.iter().filter(|c| c.tapped).map(|c| c.card.uid).collect(),
            q.grave.iter().map(|c| c.uid).collect(),
            q.shields.iter().map(|s| s.card.uid).collect(),
            q.deck.iter().map(|c| c.uid).collect(),
        ];
        for x in z.iter_mut() {
            x.sort();
        }
        v.extend(z);
    }
    v.push(vec![g.winner.map(|w| w as u16).unwrap_or(9)]);
    v
}

fn run_case(c: C, ev: textfx::Ev, seed: u64) -> Vec<Vec<u16>> {
    let (mut g, card) = scene(c, seed);
    g.stack.push(Task::MainStep);
    match ev {
        textfx::Ev::Spell(side) => {
            g.ps[0].grave.push(card);
            g.stack.push(Task::Fx(Fx::Cast { p: 0, uid: card.uid, side, free: true }));
        }
        _ => {
            enter(&mut g, 0, card, How::Put, None, 0);
        }
    }
    settle(&mut g);
    snapshot(&g)
}

#[test]
fn grade_candidates() {
    let n = 30;
    let mut total_ok = 0;
    for &(c, ev, pr) in cand::PROGS.iter() {
        if !matches!(ev, textfx::Ev::Etb | textfx::Ev::Spell(_)) {
            continue;
        }
        let mut ok = 0;
        for seed in 1..=n {
            let a = run_case(c, ev, seed);
            textfx::TEST_PROGS.with(|t| t.borrow_mut().push((c, ev, pr)));
            let b = run_case(c, ev, seed);
            textfx::TEST_PROGS.with(|t| t.borrow_mut().clear());
            if a == b {
                ok += 1;
            } else if std::env::var("TEXTFX_DEBUG").is_ok() {
                println!("DIFF {c:?} seed {seed}");
                for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
                    if x != y {
                        println!("  zone {i}: hand-written {x:?} / steps {y:?}");
                    }
                }
            }
        }
        if ok == n {
            total_ok += 1;
        }
        println!("GRADE {:?} {:?} {}/{}", c, ev, ok, n);
    }
    println!("GRADE_TOTAL {} / {}", total_ok, cand::PROGS.len());
}
