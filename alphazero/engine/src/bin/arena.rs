use dmengine::ai::*;
use dmengine::cards::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

fn parse(s: &str) -> Agent {
    let (a, n) = s.split_once(':').unwrap_or((s, "0"));
    let n: u32 = n.split(':').next().unwrap().parse().unwrap_or(0);
    match a {
        "random" => Agent::Random,
        "rollout" => Agent::Rollout,
        "greedy" => Agent::Greedy,
        "dmo" => Agent::Dmo(n.max(1)),
        "ismcts" => Agent::Ismcts(n.max(10)),
        // az:回数:重みファイル（none で世代0）
        "az" => {
            let (n, path) = s[3..].split_once(':').unwrap_or((&s[3..], "none"));
            Agent::Az(n.parse().unwrap(), dmengine::az::register(path))
        }
        _ => panic!("agent {s}"),
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let ag = [parse(&a[1]), parse(&a[2])];
    let games: u64 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(100);
    let threads: u64 = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(4);
    let seed0: u64 = a.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
    // 使うデッキ：数（既定5＝学習用5デッキ、15で環境デッキも）か、番号の並び（例 0,1,2,3）。5デッキ以外は組み合わせをランダムに選ぶ
    let pool: Vec<usize> = match a.get(6) {
        Some(x) if x.contains(',') => x.split(',').filter_map(|v| v.parse().ok()).filter(|&v: &usize| v < DECKS.len()).collect(),
        Some(x) => (0..x.parse::<usize>().unwrap_or(5).clamp(1, DECKS.len())).collect(),
        None => (0..5).collect(),
    };
    let pool = Arc::new(pool);
    let next = Arc::new(AtomicU64::new(0));
    let t = Instant::now();
    let mut hs = vec![];
    for _ in 0..threads {
        let next = next.clone();
        let pool = pool.clone();
        hs.push(std::thread::spawn(move || {
            let mut res = vec![];
            loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= games {
                    break;
                }
                // デッキの組み合わせ25通り × 席の入れ替え
                let nd = pool.len();
                let (da, db, swap) = if *pool == [0, 1, 2, 3, 4] {
                    ((i % 5) as usize, ((i / 5) % 5) as usize, (i / 25) % 2 == 1)
                } else {
                    let mut r = dmengine::rng::Rng::new(seed0 ^ (i / 2).wrapping_mul(0x9E37_79B9_7F4A_7C15));
                    (pool[r.below(nd)], pool[r.below(nd)], i % 2 == 1)
                };
                let (d0, d1) = (deck(DECKS[da]), deck(DECKS[db]));
                let agents = if swap { [ag[1], ag[0]] } else { [ag[0], ag[1]] };
                let w = play_game(&d0, &d1, agents, seed0 * 1_000_003 + i);
                // A から見た結果
                let ra = if w == 2 { 0.5 } else if (w == 0) != swap { 1.0 } else { 0.0 };
                res.push((da, db, swap, ra));
            }
            res
        }));
    }
    let mut all = vec![];
    for h in hs {
        all.extend(h.join().unwrap());
    }
    let n = all.len() as f64;
    let wa: f64 = all.iter().map(|r| r.3).sum();
    println!("{} vs {}: games {} A win rate {:.1}%  ({:.1}s)", a[1], a[2], all.len(), 100.0 * wa / n, t.elapsed().as_secs_f64());
    for d in 0..DECKS.len() {
        let v: Vec<f64> = all.iter().filter(|r| (if r.2 { r.1 } else { r.0 }) == d).map(|r| r.3).collect();
        if !v.is_empty() {
            println!("  A using {:<10} {:.1}% ({} games)", DECKS[d], 100.0 * v.iter().sum::<f64>() / v.len() as f64, v.len());
        }
    }
}
