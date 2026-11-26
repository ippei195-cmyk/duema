use dmengine::cards::*;
use dmengine::engine::*;
use std::time::Instant;

thread_local! { static STATS: std::cell::RefCell<std::collections::BTreeMap<String, u64>> = Default::default(); }

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1000);
    let mut wins = [[0u32; 3]; 25];
    let t = Instant::now();
    let mut decisions = 0u64;
    let mut turns = 0u64;
    let mut errors = 0u64;
    for seed in 0..n {
        let a = (seed % 5) as usize;
        let b = ((seed / 5) % 5) as usize;
        let da = deck(DECKS[a]);
        let db = deck(DECKS[b]);
        let res = std::panic::catch_unwind(|| {
            let mut g = Game::new(&da, &db, seed);
            let mut r = dmengine::rng::Rng::new(seed ^ 0xabcdef);
            let mut k = 0u64;
            while g.winner.is_none() {
                let d = g.decision.as_ref().unwrap();
                let i = r.below(d.opts.len());
                if let Q::Custom(eq) = &d.q { STATS.with(|s| *s.borrow_mut().entry(format!("{:?}", eq.k)).or_insert(0) += 1); }
                if let Q::UseTrigger { what, .. } = &d.q { STATS.with(|s| *s.borrow_mut().entry(format!("trigger{what}")).or_insert(0) += 1); }
                if let Opt::Play { uid, side, how } = d.opts[i] { let c = g.ps[g.tp].hand.iter().chain(g.ps[g.tp].grave.iter()).find(|c| c.uid == uid).unwrap().c; STATS.with(|s| *s.borrow_mut().entry(format!("play {:?}/{side}/{how}", c)).or_insert(0) += 1); }
                g.step(i);
                k += 1;
                for p in 0..2 {
                    assert_eq!(g.total_cards(p), 40, "card count p{p}");
                }
            }
            (g.winner.unwrap(), k, g.turn)
        });
        match res {
            Ok((w, k, tn)) => {
                wins[a * 5 + b][w] += 1;
                decisions += k;
                turns += tn as u64;
            }
            Err(_) => {
                errors += 1;
                eprintln!("seed {seed} panicked");
            }
        }
    }
    let el = t.elapsed().as_secs_f64();
    println!("games {n} errors {errors} time {:.2}s  {:.0} games/s  avg decisions {:.1} avg turns {:.1}", el, n as f64 / el, decisions as f64 / n as f64, turns as f64 / n as f64);
    if args.get(2).map(|s| s == "stats").unwrap_or(false) {
        STATS.with(|s| for (k, v) in s.borrow().iter() { println!("{k:>28} {v}"); });
    }
    for a in 0..5 {
        for b in 0..5 {
            let w = wins[a * 5 + b];
            println!("{:>10} vs {:<10} first-listed wins {} / {} draws {}", DECKS[a], DECKS[b], w[0], w[1], w[2]);
        }
    }
}
