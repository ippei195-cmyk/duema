//! 自己対戦で学習データを作る。
//! selfplay <重み|none> <局数> <スレッド> <seed> <出力.bin> [深い読み回数=200] [浅い読み回数=40] [深い読みの割合=0.25] [focus=0] [重点デッキ=0（例 0,3,4）] [決めパターン直前から始める割合=0] [noteの方針バイアスの強さ=0]
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
    // focus: この確率で片方（どちらかはランダム）のデッキを重点デッキ（focus_decks、既定はリースバイク）のどれかにして、苦手や新しいデッキを重点的に学ぶ
    let focus: f64 = a.get(9).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let focus_decks: Vec<usize> = a.get(10).map(|s| s.split(',').filter_map(|x| x.parse().ok()).filter(|&x: &usize| x < DECKS.len()).collect()).unwrap_or_else(|| vec![0]);
    let focus_decks = Arc::new(if focus_decks.is_empty() { vec![0] } else { focus_decks });
    // note の考え方（playbook.rs）：ギリガザミ・創世竜の「決めの直前」から始める割合と、方策バイアスの強さ
    let scen: f64 = a.get(11).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    dmengine::playbook::set_strength(a.get(12).and_then(|s| s.parse().ok()).unwrap_or(0.0));
    let combo: Vec<usize> = ["giri4c", "sousei"].iter().filter_map(|n| DECKS.iter().position(|d| d == n)).collect();
    let combo = Arc::new(combo);
    let net: Arc<Option<Net>> = Arc::new(if wpath == "none" { None } else { Some(Net::load(&wpath).expect("weights")) });
    let file = Arc::new(Mutex::new(std::io::BufWriter::new(std::fs::File::create(&out).unwrap())));
    file.lock().unwrap().write_all(b"DMS1").unwrap();
    let next = Arc::new(AtomicU64::new(0));
    let stats = Arc::new(Mutex::new((0u64, 0u64, 0u64, [[0f64; 2]; DECKS.len()]))); // 局, 記録数, 判断数, デッキ別[勝ち,局]
    let t = Instant::now();
    let mut hs = vec![];
    for _ in 0..threads {
        let (net, file, next, stats, fd, combo) = (net.clone(), file.clone(), next.clone(), stats.clone(), focus_decks.clone(), combo.clone());
        hs.push(std::thread::spawn(move || loop {
            let i = next.fetch_add(1, Ordering::SeqCst);
            if i >= games { break; }
            let seed = seed0 * 1_000_003 + i;
            let mut rng = Rng::new(seed ^ 0xA2A2);
            let mut da = rng.below(DECKS.len());
            let mut db = rng.below(DECKS.len());
            if (rng.below(1_000_000) as f64) < focus * 1e6 {
                let f = fd[rng.below(fd.len())];
                if rng.below(2) == 0 { da = f } else { db = f }
            }
            let mut side = 2;
            if !combo.is_empty() && (rng.below(1_000_000) as f64) < scen * 1e6 {
                let c = combo[rng.below(combo.len())];
                side = rng.below(2);
                if side == 0 { da = c } else { db = c }
            }
            let mut g = Game::new(&deck(DECKS[da]), &deck(DECKS[db]), seed);
            if side < 2 {
                dmengine::playbook::scenario(&mut g, side, &mut rng);
            }
            let mut recs: Vec<(usize, Vec<u8>)> = vec![]; // (判断した人, 記録)
            let mut moves = [0u32; 2];
            let mut decisions = 0u64;
            let tg = Instant::now();
            while g.winner.is_none() {
                // 念のため：判断が多すぎる対局は引き分けで打ち切る（終わらない手順の保険）
                if decisions >= 3000 {
                    g.winner = Some(2);
                    break;
                }
                if decisions % 500 == 499 && std::env::var("SP_DEBUG").is_ok() {
                    eprintln!("long game {} {} vs {} side{} turn {} decisions {} {:.0}s", i, DECKS[da], DECKS[db], side, g.turn, decisions + 1, tg.elapsed().as_secs_f64());
                }
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
    let wr: Vec<String> = (0..DECKS.len()).map(|k| format!("{} {:.1}%", DECKS[k], 100.0 * s.3[k][0] / s.3[k][1].max(1.0))).collect();
    println!("games={} samples={} decisions={} secs={:.1} | {}", s.0, s.1, s.2, t.elapsed().as_secs_f64(), wr.join(" "));
}
