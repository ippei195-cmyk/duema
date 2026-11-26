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
    let next = Arc::new(AtomicU64::new(0));
    let t = Instant::now();
    let mut hs = vec![];
    for _ in 0..threads {
        let next = next.clone();
        hs.push(std::thread::spawn(move || {
            let mut res = vec![];
            loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= games {
                    break;
                }
                // デッキの組み合わせ25通り × 席の入れ替え
                let da = (i % 5) as usize;
                let db = ((i / 5) % 5) as usize;
                let swap = (i / 25) % 2 == 1;
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
