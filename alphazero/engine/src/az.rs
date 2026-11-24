//! AlphaZero 式の先読み：PUCT-ISMCTS。方策ネットの事前確率で候補を絞り、葉はネットの価値で評価。
//! ネットがまだ無い世代0は、事前確率を一様・葉をプレイアウトで評価する。

use crate::ai::{determinize, key, playout};
use crate::engine::*;
use crate::features;
use crate::net::Net;
use crate::rng::Rng;
use std::sync::{Arc, Mutex, OnceLock};

struct Node {
    key: u64,
    player: usize, // この手を選んだプレイヤー
    prior: f32,
    n: u32,
    w: f32, // player から見た価値の合計（-1..1）
    avail: u32,
    children: Vec<usize>,
}

pub struct Az<'a> {
    pub net: Option<&'a Net>,
    pub sims: u32,
    pub cpuct: f32,
    pub noise: bool, // 根にディリクレ雑音（自己対戦用）
}

/// 根の結果：各選択肢の訪問割合と、選んだ番号
pub struct Res {
    pub pi: Vec<f32>,
    pub value: f32,
}

fn gamma(rng: &mut Rng, a: f64) -> f64 {
    // Marsaglia–Tsang（a<1 は a+1 で引いて U^(1/a) を掛ける）
    if a < 1.0 {
        let u = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        return gamma(rng, a + 1.0) * u.max(1e-300).powf(1.0 / a);
    }
    let d = a - 1.0 / 3.0;
    let c = 1.0 / (9.0 * d).sqrt();
    loop {
        let u1 = ((rng.next_u64() >> 11) as f64 + 0.5) / (1u64 << 53) as f64;
        let u2 = ((rng.next_u64() >> 11) as f64 + 0.5) / (1u64 << 53) as f64;
        let x = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        let v = (1.0 + c * x).powi(3);
        if v <= 0.0 { continue; }
        let u = ((rng.next_u64() >> 11) as f64 + 0.5) / (1u64 << 53) as f64;
        if u.ln() < 0.5 * x * x + d - d * v + d * v.ln() {
            return d * v;
        }
    }
}

/// 1つしか選べない判断は先読みせずに進める
fn skip_forced(s: &mut Game) {
    while s.winner.is_none() && s.decision.as_ref().unwrap().opts.len() == 1 {
        s.step(0);
    }
}

impl<'a> Az<'a> {
    /// 局面の評価：(判断するプレイヤーから見た価値, 選択肢ごとの事前確率)
    fn evaluate(&self, s: &Game, rng: &mut Rng) -> (f32, Vec<f32>) {
        let d = s.decision.as_ref().unwrap();
        let n = d.opts.len();
        match self.net {
            Some(net) => {
                let st = features::state(s, d.player);
                let of: Vec<Vec<f32>> = d.opts.iter().map(|&o| features::option(s, d.player, o)).collect();
                let (v, lg) = net.eval(&st, &of);
                let m = lg.iter().cloned().fold(f32::MIN, f32::max);
                let e: Vec<f32> = lg.iter().map(|x| (x - m).exp()).collect();
                let z: f32 = e.iter().sum();
                (v, e.iter().map(|x| x / z).collect())
            }
            None => {
                let mut c = s.clone();
                playout(&mut c, rng);
                let w = c.winner.unwrap();
                let v = if w == 2 { 0.0 } else if w == d.player { 1.0 } else { -1.0 };
                (v, vec![1.0 / n as f32; n])
            }
        }
    }

    pub fn search(&self, g: &Game, rng: &mut Rng) -> Res {
        let d = g.decision.as_ref().unwrap();
        let me = d.player;
        let nopt = d.opts.len();
        if nopt == 1 {
            return Res { pi: vec![1.0], value: 0.0 };
        }
        let mut nodes = vec![Node { key: 0, player: 1 - me, prior: 1.0, n: 0, w: 0.0, avail: 0, children: vec![] }];
        let mut root_noise: Vec<(u64, f32)> = vec![];
        for it in 0..self.sims {
            let mut s = determinize(g, me, rng);
            let mut path = vec![0usize];
            let mut cur = 0usize;
            let leaf_v: f32;
            let leaf_p: usize;
            loop {
                if s.winner.is_some() {
                    let w = s.winner.unwrap();
                    leaf_p = me;
                    leaf_v = if w == 2 { 0.0 } else if w == me { 1.0 } else { -1.0 };
                    break;
                }
                let d = s.decision.as_ref().unwrap();
                let player = d.player;
                let keys: Vec<u64> = d.opts.iter().map(|&o| key(&s, d, o)).collect();
                let missing = keys.iter().enumerate().any(|(i, k)| !keys[..i].contains(k) && !nodes[cur].children.iter().any(|&c| nodes[c].key == *k));
                if missing {
                    // 葉：評価して子を一度に作る
                    let (v, pr) = self.evaluate(&s, rng);
                    for (i, &k) in keys.iter().enumerate() {
                        if keys[..i].contains(&k) || nodes[cur].children.iter().any(|&c| nodes[c].key == k) {
                            continue;
                        }
                        nodes.push(Node { key: k, player, prior: pr[i], n: 0, w: 0.0, avail: 0, children: vec![] });
                        let id = nodes.len() - 1;
                        nodes[cur].children.push(id);
                    }
                    if cur == 0 && it == 0 && self.noise {
                        let ks: Vec<u64> = nodes[0].children.iter().map(|&c| nodes[c].key).collect();
                        let gs: Vec<f64> = ks.iter().map(|_| gamma(rng, 0.3)).collect();
                        let t: f64 = gs.iter().sum::<f64>().max(1e-12);
                        root_noise = ks.iter().zip(gs).map(|(&k, x)| (k, (x / t) as f32)).collect();
                    }
                    leaf_v = v;
                    leaf_p = player;
                    break;
                }
                // PUCT：この判断で合法な子だけで比べる
                let mut tot_avail = 0u32;
                let mut cand = vec![];
                for (i, &k) in keys.iter().enumerate() {
                    if keys[..i].contains(&k) { continue; }
                    let ch = nodes[cur].children.iter().copied().find(|&c| nodes[c].key == k).unwrap();
                    nodes[ch].avail += 1;
                    tot_avail += nodes[ch].n;
                    cand.push((ch, i));
                }
                let psum: f32 = cand.iter().map(|&(c, _)| nodes[c].prior).sum::<f32>().max(1e-6);
                let sq = (tot_avail as f32 + 1.0).sqrt();
                let mut best = (f32::MIN, 0usize, 0usize);
                for &(ch, i) in &cand {
                    let nd = &nodes[ch];
                    let mut p = nd.prior / psum;
                    if cur == 0 && !root_noise.is_empty() {
                        let e = root_noise.iter().find(|x| x.0 == nd.key).map(|x| x.1).unwrap_or(0.0);
                        p = 0.75 * p + 0.25 * e;
                    }
                    // まだ訪れていない手は親の平均より少し悪いとみなす（FPU）
                    let q = if nd.n > 0 { nd.w / nd.n as f32 } else { -0.2 };
                    let u = q + self.cpuct * p * sq / (1.0 + nd.n as f32);
                    if u > best.0 {
                        best = (u, ch, i);
                    }
                }
                s.step(best.2);
                path.push(best.1);
                cur = best.1;
                skip_forced(&mut s);
            }
            for &id in &path {
                let nd = &mut nodes[id];
                nd.n += 1;
                nd.w += if nd.player == leaf_p { leaf_v } else { -leaf_v };
            }
        }
        let keys: Vec<u64> = d.opts.iter().map(|&o| key(g, d, o)).collect();
        let mut pi = vec![0f32; nopt];
        let mut tot = 0f32;
        let mut wsum = 0f32;
        for (i, &k) in keys.iter().enumerate() {
            if keys[..i].contains(&k) { continue; }
            if let Some(ch) = nodes[0].children.iter().copied().find(|&c| nodes[c].key == k) {
                pi[i] = nodes[ch].n as f32;
                tot += pi[i];
                wsum += nodes[ch].w;
            }
        }
        if tot > 0.0 {
            for x in pi.iter_mut() { *x /= tot; }
        } else {
            pi = vec![1.0 / nopt as f32; nopt];
        }
        Res { pi, value: if tot > 0.0 { wsum / tot } else { 0.0 } }
    }

    pub fn choose(&self, g: &Game, rng: &mut Rng) -> usize {
        let r = self.search(g, rng);
        argmax(&r.pi, rng)
    }
}

pub fn argmax(pi: &[f32], rng: &mut Rng) -> usize {
    let mut best = (f32::MIN, 0usize);
    for (i, &x) in pi.iter().enumerate() {
        let x = x + rng.below(1000) as f32 * 1e-7;
        if x > best.0 { best = (x, i); }
    }
    best.1
}

pub fn sample(pi: &[f32], rng: &mut Rng) -> usize {
    let r = (rng.next_u64() >> 11) as f32 / (1u64 << 53) as f32;
    let mut acc = 0.0;
    for (i, &x) in pi.iter().enumerate() {
        acc += x;
        if r < acc { return i; }
    }
    pi.len() - 1
}

// ---------- 読み込んだネットの置き場（Agent は Copy なので番号で指す） ----------

static NETS: OnceLock<Mutex<Vec<Arc<Option<Net>>>>> = OnceLock::new();

/// "none" ならネット無し（世代0）
pub fn register(path: &str) -> usize {
    let net = if path == "none" { None } else { Some(Net::load(path).expect("weights")) };
    let m = NETS.get_or_init(|| Mutex::new(vec![]));
    let mut v = m.lock().unwrap();
    v.push(Arc::new(net));
    v.len() - 1
}

pub fn get(slot: usize) -> Arc<Option<Net>> {
    NETS.get().unwrap().lock().unwrap()[slot].clone()
}
