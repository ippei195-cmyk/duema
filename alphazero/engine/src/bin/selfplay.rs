//! 自己対戦で学習データを作る。
//! selfplay <重み|none> <局数> <スレッド> <seed> <出力.bin> [深い読み回数=200] [浅い読み回数=40] [深い読みの割合=0.25]
//! 深く読んだ判断だけを記録する（KataGo の読み回数ランダム化）。
use dmengine::az::*;
use dmengine::cards::*;
use dmengine::engine::*;
use dmengine::features;
use dmengine::net::Net;
use dmengine::rng::Rng;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn sparse(out: &mut Vec<u8>, v: &[f32]) {
    let nz: Vec<usize> = (0..v.len()).filter(|&i| v[i] != 0.0).collect();
    out.extend((nz.len() as u16).to_le_bytes());
    for i in nz {
        out.extend((i as u16).to_le_bytes());
        out.extend(v[i].to_le_bytes());
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let wpath = a[1].clone();
    let games: u64 = a[2].parse().unwrap();
    let threads: u64 = a[3].parse().unwrap();
    let seed0: u64 = a[4].parse().unwrap();
    let out = a[5].clone();
    let full: u32 = a.get(6).and_then(|s| s.parse().ok()).unwrap_or(200);
    let fast: u32 = a.get(7).and_then(|s| s.parse().ok()).unwrap_or(40);
    let pfull: f64 = a.get(8).and_then(|s| s.parse().ok()).unwrap_or(0.25);
    let net: Arc<Option<Net>> = Arc::new(if wpath == "none" { None } else { Some(Net::load(&wpath).expect("weights")) });
    let file = Arc::new(Mutex::new(std::io::BufWriter::new(std::fs::File::create(&out).unwrap())));
    file.lock().unwrap().write_all(b"DMS1").unwrap();
    let next = Arc::new(AtomicU64::new(0));
    let stats = Arc::new(Mutex::new((0u64, 0u64, 0u64, [[0f64; 2]; 3]))); // 局, 記録数, 判断数, デッキ別[勝ち,局]
    let t = Instant::now();
    let mut hs = vec![];
    for _ in 0..threads {
        let (net, file, next, stats) = (net.clone(), file.clone(), next.clone(), stats.clone());
        hs.push(std::thread::spawn(move || loop {
            let i = next.fetch_add(1, Ordering::SeqCst);
            if i >= games { break; }
            let seed = seed0 * 1_000_003 + i;
            let mut rng = Rng::new(seed ^ 0xA2A2);
            let da = rng.below(3);
            let db = rng.below(3);
            let mut g = Game::new(&deck(DECKS[da]), &deck(DECKS[db]), seed);
            let mut recs: Vec<(usize, Vec<u8>)> = vec![]; // (判断した人, 記録)
            let mut moves = [0u32; 2];
            let mut decisions = 0u64;
            while g.winner.is_none() {
                let d = g.decision.as_ref().unwrap();
                if d.opts.len() == 1 { g.step(0); continue; }
                let p = d.player;
                decisions += 1;
                let deep = (rng.below(1_000_000) as f64) < pfull * 1e6;
                let az = Az { net: net.as_ref().as_ref(), sims: if deep { full } else { fast }, cpuct: 1.5, noise: deep };
                let r = az.search(&g, &mut rng);
                if deep {
                    let mut b = vec![];
                    b.extend((d.opts.len() as u16).to_le_bytes());
                    b.extend(0f32.to_le_bytes()); // z はあとで埋める
                    sparse(&mut b, &features::state(&g, p));
                    for (k, &o) in d.opts.iter().enumerate() {
                        b.extend(r.pi[k].to_le_bytes());
                        sparse(&mut b, &features::option(&g, p, o));
                    }
                    recs.push((p, b));
                }
                let i = if moves[p] < 15 { sample(&r.pi, &mut rng) } else { argmax(&r.pi, &mut rng) };
                moves[p] += 1;
                g.step(i);
            }
            let w = g.winner.unwrap();
            let mut buf = vec![];
            for (p, mut b) in recs.iter().cloned() {
                let z: f32 = if w == 2 { 0.0 } else if w == p { 1.0 } else { -1.0 };
                b[2..6].copy_from_slice(&z.to_le_bytes());
                buf.extend(b);
            }
            file.lock().unwrap().write_all(&buf).unwrap();
            let mut s = stats.lock().unwrap();
            s.0 += 1;
            s.1 += recs.len() as u64;
            s.2 += decisions;
            for (pl, dk) in [(0usize, da), (1, db)] {
                s.3[dk][1] += 1.0;
                s.3[dk][0] += if w == 2 { 0.5 } else if w == pl { 1.0 } else { 0.0 };
            }
            if s.0 % 50 == 0 {
                eprintln!("  {} 局 {:.0}s", s.0, t.elapsed().as_secs_f64());
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
    file.lock().unwrap().flush().unwrap();
    let s = stats.lock().unwrap();
    let wr: Vec<String> = (0..3).map(|k| format!("{} {:.1}%", DECKS[k], 100.0 * s.3[k][0] / s.3[k][1].max(1.0))).collect();
    println!("games={} samples={} decisions={} secs={:.1} | {}", s.0, s.1, s.2, t.elapsed().as_secs_f64(), wr.join(" "));
}
