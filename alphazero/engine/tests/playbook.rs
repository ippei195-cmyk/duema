use dmengine::az::Az;
use dmengine::cards::*;
use dmengine::engine::*;
use dmengine::playbook;
use dmengine::rng::Rng;

fn start(name: &str, side: usize, seed: u64) -> Game {
    let me = deck(name);
    let op = deck("leasebike");
    let (a, b) = if side == 0 { (&me, &op) } else { (&op, &me) };
    let mut g = Game::new(a, b, seed);
    let mut r = Rng::new(seed);
    assert!(playbook::scenario(&mut g, side, &mut r));
    g
}

#[test]
fn giri_scenario_shape() {
    for seed in 0..30 {
        let side = (seed % 2) as usize;
        let g = start("giri4c", side, seed);
        let q = &g.ps[side];
        assert_eq!(g.tp, side);
        assert!(q.grave.iter().any(|c| c.c == C::Petens), "seed {seed}");
        assert!(q.hand.iter().any(|c| c.c == C::Shinkirou));
        assert!(q.shields.iter().any(|s| s.card.c == C::Giri));
        assert!(q.mana.len() >= 6);
        for p in 0..2 { assert_eq!(g.total_cards(p), 40); }
    }
}

#[test]
fn sousei_scenario_shape() {
    for seed in 0..30 {
        let side = (seed % 2) as usize;
        let g = start("sousei", side, seed);
        let q = &g.ps[side];
        assert_eq!(q.battle.iter().filter(|x| x.card.c == C::Drache).count(), 2, "seed {seed}");
        assert!(q.hand.iter().any(|c| c.c == C::Atlantis));
        for p in 0..2 { assert_eq!(g.total_cards(p), 40); }
    }
}

#[test]
fn scenario_games_finish() {
    for seed in 0..400 {
        let name = if seed % 2 == 0 { "giri4c" } else { "sousei" };
        let mut g = start(name, (seed / 2 % 2) as usize, seed);
        let mut r = Rng::new(seed ^ 77);
        while g.winner.is_none() {
            let n = g.decision.as_ref().unwrap().opts.len();
            g.step(r.below(n));
            for p in 0..2 { assert_eq!(g.total_cards(p), 40); }
        }
    }
}

#[test]
fn bias_changes_prior() {
    // 創世竜の場面で、何かの判断に重みがかかる（=None ではない）ことがある
    let mut hit = 0;
    for seed in 0..40 {
        let mut g = start(if seed % 2 == 0 { "giri4c" } else { "sousei" }, 0, seed);
        let mut r = Rng::new(seed);
        for _ in 0..60 {
            if g.winner.is_some() { break; }
            let d = g.decision.clone().unwrap();
            if let Some(w) = playbook::weights(&g, &d) {
                assert_eq!(w.len(), d.opts.len());
                assert!(w.iter().all(|x| *x > 0.0));
                hit += 1;
            }
            g.step(r.below(d.opts.len()));
        }
    }
    assert!(hit > 20, "hit {hit}");
    // 強さ0なら何も変わらない
    let g = start("sousei", 0, 3);
    let n = g.decision.as_ref().unwrap().opts.len();
    let mut pr = vec![1.0 / n as f32; n];
    playbook::apply(&g, &mut pr);
    assert!(pr.iter().all(|x| (x - 1.0 / n as f32).abs() < 1e-6));
    let _ = Az { net: None, sims: 1, cpuct: 1.5, noise: false };
}
