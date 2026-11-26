// 同じAIどうしで、デッキの組み合わせごとの勝率（先攻=席0）とターン数を出す
use dmengine::ai::*;
use dmengine::cards::*;
use dmengine::engine::Game;
use dmengine::rng::Rng;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n: u32 = a[1].parse().unwrap();
    let per: u64 = a[2].parse().unwrap();
    let th: u64 = a[3].parse().unwrap();
    let ag = Agent::Ismcts(n);
    let total = per * 25;
    let next = Arc::new(AtomicU64::new(0));
    // [d0][d1] = (席0勝ち, 局, ターン合計)
    let res = Arc::new(Mutex::new([[(0f64, 0f64, 0f64); 5]; 5]));
    let mut hs = vec![];
    for _ in 0..th {
        let (next, res) = (next.clone(), res.clone());
        hs.push(std::thread::spawn(move || loop {
            let i = next.fetch_add(1, Ordering::SeqCst);
            if i >= total { break; }
            let (d0, d1) = ((i % 5) as usize, ((i / 5) % 5) as usize);
            let seed = 777_000 + i;
            let mut g = Game::new(&deck(DECKS[d0]), &deck(DECKS[d1]), seed);
            let mut rng = Rng::new(seed ^ 0x5151);
            while g.winner.is_none() {
                let i = pick(ag, &g, &mut rng);
                g.step(i);
            }
            let w = g.winner.unwrap();
            let r = if w == 0 { 1.0 } else if w == 1 { 0.0 } else { 0.5 };
            let mut m = res.lock().unwrap();
            m[d0][d1].0 += r; m[d0][d1].1 += 1.0; m[d0][d1].2 += g.turn as f64;
        }));
    }
    for h in hs { h.join().unwrap(); }
    let m = res.lock().unwrap();
    println!("先攻\\後攻  席0(先攻)の勝率 / 平均ターン");
    for x in 0..5 { for y in 0..5 {
        let (w, c, t) = m[x][y];
        println!("{:<10} vs {:<10} {:5.1}%  {:4.1}  ({} 局)", DECKS[x], DECKS[y], 100.0 * w / c, t / c, c);
    }}
    // デッキの強さ（先後合わせた勝率）
    for d in 0..5 {
        let mut w = 0.0; let mut c = 0.0;
        for o in 0..5 { if o == d { continue; }
            w += m[d][o].0 + (m[o][d].1 - m[o][d].0); c += m[d][o].1 + m[o][d].1; }
        println!("{} の他デッキへの勝率 {:.1}%", DECKS[d], 100.0 * w / c);
    }
}
