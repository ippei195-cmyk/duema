//! リースバイクの定石チェック：指定のAIにリースバイクを使わせ、相手（ismcts:300）はハッスルかジョーカーズ。
//! playbook <me: az:N:path | ismcts:N> <games> <threads> <seed>
use dmengine::ai::*;
use dmengine::cards::*;
use dmengine::engine::{Game, Opt, Q};
use dmengine::effects::K;
use dmengine::rng::Rng;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

fn parse(s: &str) -> Agent {
    if let Some(r) = s.strip_prefix("az:") {
        let (n, path) = r.split_once(':').unwrap_or((r, "none"));
        Agent::Az(n.parse().unwrap(), dmengine::az::register(path))
    } else if let Some(n) = s.strip_prefix("ismcts:") {
        Agent::Ismcts(n.parse().unwrap())
    } else { panic!("agent {s}") }
}

#[derive(Default, Clone)]
struct G1 {
    opp: usize, win: f64, first: bool,
    caddy_t: Option<u32>, ppp_t: Option<u32>, both_t: Option<u32>,
    // ジョーカーズ戦：相手のターンのはじめに、相手マナ≤4 / ≥5 で [どちらも無し, 1種, 2種] の回数
    lock: [[u32; 3]; 2],
    first_attack_t: Option<u32>, hyper_t: Option<u32>,
    attacks: u32, attacks_hyper: u32,
    gyaku_opp: u32,              // 相手が逆転撃を使った回数
    dagura_ask_castle: u32, dagura_ask_open: u32, // ダグラジャパニカンの効果を使う場面（キャッスルあり／なし）
    turns: u32,
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let me_ag = parse(&a[1]);
    let games: u64 = a[2].parse().unwrap();
    let th: u64 = a[3].parse().unwrap();
    let seed0: u64 = a[4].parse().unwrap();
    let opp_ag = Agent::Ismcts(300);
    let next = Arc::new(AtomicU64::new(0));
    let out = Arc::new(Mutex::new(Vec::<G1>::new()));
    let mut hs = vec![];
    for _ in 0..th {
        let (next, out) = (next.clone(), out.clone());
        hs.push(std::thread::spawn(move || loop {
            let i = next.fetch_add(1, Ordering::SeqCst);
            if i >= games { break; }
            let opp_deck = 1 + (i % 2) as usize;          // 1=hustle 2=jokers
            let me = ((i / 2) % 2) as usize;              // 0=先攻
            let seed = seed0 * 1_000_003 + i;
            let decks = if me == 0 { [0, opp_deck] } else { [opp_deck, 0] };
            let mut g = Game::new(&deck(DECKS[decks[0]]), &deck(DECKS[decks[1]]), seed);
            let mut rng = Rng::new(seed ^ 0x5151);
            let op = 1 - me;
            let mut r = G1 { opp: opp_deck, first: me == 0, ..Default::default() };
            let mut my_t = 0u32; let mut last_turn = u32::MAX;
            while g.winner.is_none() {
                if g.turn != last_turn {
                    last_turn = g.turn;
                    if g.tp == me { my_t += 1; } else if opp_deck == 2 {
                        let m = (g.ps[op].mana.len() >= 5) as usize;
                        let has = |c: C| g.ps[me].battle.iter().any(|x| x.card.c == c);
                        let k = has(C::Caddy) as usize + has(C::PPP) as usize;
                        r.lock[m][k] += 1;
                    }
                }
                let d = g.decision.as_ref().unwrap();
                let p = d.player;
                let ag = if p == me { me_ag } else { opp_ag };
                let ix = pick(ag, &g, &mut rng);
                let o = d.opts[ix].clone();
                if p == me {
                    if let Opt::Attack { att, .. } = o {
                        r.attacks += 1;
                        if r.first_attack_t.is_none() { r.first_attack_t = Some(my_t); }
                        if g.ps[me].battle.iter().any(|x| x.card.uid == att && x.hyper) { r.attacks_hyper += 1; }
                    }
                    if let Q::Custom(eq) = &d.q {
                        if eq.k == K::DaguraMana {
                            if g.ps[op].shields.iter().any(|s| s.faceup && s.card.c == C::Hustle) { r.dagura_ask_castle += 1 } else { r.dagura_ask_open += 1 }
                        }
                    }
                } else if let Q::UseTrigger { what: 2, .. } = d.q {
                    if o == Opt::Yes { r.gyaku_opp += 1; }
                }
                g.step(ix);
                let b = &g.ps[me].battle;
                let hc = b.iter().any(|x| x.card.c == C::Caddy);
                let hp = b.iter().any(|x| x.card.c == C::PPP);
                if hc && r.caddy_t.is_none() { r.caddy_t = Some(my_t); }
                if hp && r.ppp_t.is_none() { r.ppp_t = Some(my_t); }
                if hc && hp && r.both_t.is_none() { r.both_t = Some(my_t); }
                if b.iter().any(|x| x.hyper) && r.hyper_t.is_none() { r.hyper_t = Some(my_t); }
            }
            let w = g.winner.unwrap();
            r.win = if w == me { 1.0 } else if w == 2 { 0.5 } else { 0.0 };
            r.turns = my_t;
            out.lock().unwrap().push(r);
        }));
    }
    for h in hs { h.join().unwrap(); }
    let v = out.lock().unwrap();
    let avg = |f: &dyn Fn(&G1) -> Option<u32>, s: &[&G1]| -> String {
        let x: Vec<u32> = s.iter().filter_map(|r| f(r)).collect();
        if x.is_empty() { "-".into() } else { format!("{:.1}ターン目（{:.0}%の試合）", x.iter().sum::<u32>() as f64 / x.len() as f64, 100.0 * x.len() as f64 / s.len() as f64) }
    };
    let wr = |s: &[&G1]| if s.is_empty() { "-".to_string() } else { format!("{:.1}%（{}局）", 100.0 * s.iter().map(|r| r.win).sum::<f64>() / s.len() as f64, s.len()) };
    println!("AI={}  リースバイクで {} 局", a[1], v.len());
    for od in [1usize, 2] {
        let s: Vec<&G1> = v.iter().filter(|r| r.opp == od).collect();
        println!("\n■ 相手 {}：勝率 {}（先攻 {} ／ 後攻 {}）", DECKS[od], wr(&s),
            wr(&s.iter().copied().filter(|r| r.first).collect::<Vec<_>>()), wr(&s.iter().copied().filter(|r| !r.first).collect::<Vec<_>>()));
        println!("  キャディ初登場 {}", avg(&|r| r.caddy_t, &s));
        println!("  PP-「P」初登場 {}", avg(&|r| r.ppp_t, &s));
        println!("  2種そろう    {}", avg(&|r| r.both_t, &s));
        println!("  初めての攻撃 {}", avg(&|r| r.first_attack_t, &s));
        println!("  ハイパーモード初 {}", avg(&|r| r.hyper_t, &s));
        let at: u32 = s.iter().map(|r| r.attacks).sum(); let ah: u32 = s.iter().map(|r| r.attacks_hyper).sum();
        println!("  攻撃のうちハイパー状態のクリーチャーで {:.0}%（{}/{}）", 100.0 * ah as f64 / at.max(1) as f64, ah, at);
        let gy: u32 = s.iter().map(|r| r.gyaku_opp).sum();
        println!("  相手の逆転撃 1試合あたり {:.2} 回", gy as f64 / s.len().max(1) as f64);
        let dc: u32 = s.iter().map(|r| r.dagura_ask_castle).sum(); let dn: u32 = s.iter().map(|r| r.dagura_ask_open).sum();
        println!("  ジャパニカン効果の場面：キャッスルあり {} ／ キャッスルなし {}", dc, dn);
        let hy: Vec<&G1> = s.iter().copied().filter(|r| r.hyper_t.is_some()).collect();
        let nh: Vec<&G1> = s.iter().copied().filter(|r| r.hyper_t.is_none()).collect();
        println!("  ハイパーモードを使った試合の勝率 {} ／ 使わなかった試合 {}", wr(&hy), wr(&nh));
        if od == 2 {
            let mut l = [[0u32; 3]; 2];
            for r in &s { for m in 0..2 { for k in 0..3 { l[m][k] += r.lock[m][k]; } } }
            for (m, name) in [(0, "相手マナ4以下"), (1, "相手マナ5以上")] {
                let t = (l[m][0] + l[m][1] + l[m][2]).max(1) as f64;
                println!("  {}の相手ターン開始時：2種 {:.0}% ／ 1種 {:.0}% ／ 0種 {:.0}%", name, 100.0 * l[m][2] as f64 / t, 100.0 * l[m][1] as f64 / t, 100.0 * l[m][0] as f64 / t);
            }
            let both5: Vec<&G1> = s.iter().copied().filter(|r| r.lock[1][2] > 0).collect();
            let not5: Vec<&G1> = s.iter().copied().filter(|r| r.lock[1][2] == 0).collect();
            println!("  マナ5以上の相手ターンに2種を維持できた試合の勝率 {} ／ できなかった試合 {}", wr(&both5), wr(&not5));
        }
    }
}
