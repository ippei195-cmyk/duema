use dmengine::az::Az;
use dmengine::cards::*;
use dmengine::engine::*;
use dmengine::rng::Rng;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let net = dmengine::net::Net::load(&a[1]).unwrap();
    dmengine::playbook::set_strength(1.0);
    for seed in 0..40u64 {
        let mut g = Game::new(&deck("sousei"), &deck("zaze"), seed);
        let mut r = Rng::new(seed);
        dmengine::playbook::scenario(&mut g, 0, &mut r);
        let mut n = 0;
        while g.winner.is_none() && n < 400 {
            let d = g.decision.clone().unwrap();
            let az = Az { net: Some(&net), sims: 40, cpuct: 1.5, noise: false };
            let i = if d.opts.len() == 1 { 0 } else { let res = az.search(&g, &mut r); (0..res.pi.len()).max_by(|&x, &y| res.pi[x].partial_cmp(&res.pi[y]).unwrap()).unwrap() };
            if n >= 300 && n < 340 {
                let q = &g.ps[d.player];
                let desc = match d.opts[i] { Opt::Play { uid, side, how } => format!("play {:?} s{side} h{how}", q.hand.iter().chain(q.grave.iter()).find(|c| c.uid == uid).map(|c| c.c).or_else(|| q.mana.iter().find(|m| m.card.uid == uid).map(|m| m.card.c))), Opt::Pick(u) => format!("pick {u}"), o => format!("{o:?}") };
                println!("seed {seed} n{n} t{} p{} {:?} {:?} -> {desc}  mana {} untap {} hand {} deck {}", g.turn, d.player, d.kind, match &d.q { Q::Custom(e) => format!("{:?}", e.k), q => format!("{q:?}").chars().take(30).collect() }, q.mana.len(), q.mana.iter().filter(|m| !m.tapped).count(), q.hand.len(), q.deck.len());
            }
            g.step(i);
            n += 1;
        }
        if n >= 400 { println!("seed {seed} LONG"); break; } else { println!("seed {seed} done n{n} turn {}", g.turn); }
    }
}
