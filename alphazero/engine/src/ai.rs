//! 先読みAI：ISMCTS（相手の見えない札を毎回並べ直して読む）と、比較用の DMO 式（候補ごとに1回プレイアウト）。

use crate::cards::*;
use crate::engine::*;
use crate::rng::Rng;

/// viewer から見えない札（相手の手札・裏向きシールド・山札、自分の裏向きシールド・山札）を並べ直す
/// 処理中の効果が指している札（並べ直すと壊れるので動かさない）
fn pinned(g: &Game) -> Vec<u16> {
    let mut v = vec![];
    let eq = |e: &crate::effects::EQ, v: &mut Vec<u16>| {
        v.extend(e.list.iter().copied());
        if e.a > 0 && e.a < 65536 { v.push(e.a as u16); }
        if e.b > 0 && e.b < 65536 { v.push(e.b as u16); }
    };
    if let Some(d) = &g.decision {
        for o in &d.opts {
            match *o {
                Opt::Charge(u) | Opt::Block(u) | Opt::Pick(u) => v.push(u),
                Opt::Play { uid, .. } => v.push(uid),
                Opt::Attack { att, .. } => v.push(att),
                _ => {}
            }
        }
        match &d.q {
            Q::Custom(e) => eq(e, &mut v),
            Q::UseTrigger { uid, .. } => v.push(*uid),
            Q::Shield { picked, .. } => v.extend(picked.iter().copied()),
            Q::None => {}
        }
    }
    // 下にカードが重なっているシールド（エバーラスト等でシールド化した進化エレメント）は動かさない
    for q in &g.ps {
        v.extend(q.sh_under.iter().map(|x| x.0));
    }
    for t in &g.stack {
        match t {
            Task::Fx(crate::effects::Fx::Ask(e)) => eq(e, &mut v),
            Task::Fx(crate::effects::Fx::TriggerDecl { uids, .. }) | Task::ShieldsToHand { uids, .. } => v.extend(uids.iter().copied()),
            Task::Ask(d) => {
                if let Q::UseTrigger { uid, .. } = d.q { v.push(uid); }
                if let Q::Custom(e) = &d.q { eq(e, &mut v); }
            }
            _ => {}
        }
    }
    v
}

pub fn determinize(g: &Game, viewer: usize, rng: &mut Rng) -> Game {
    let mut s = g.clone();
    let pin = pinned(g);
    for p in 0..2 {
        let q = &mut s.ps[p];
        let hidden_hand = p != viewer;
        // 動かす場所（山札・裏向きシールド・相手の手札）のうち、処理中の札は除く
        let mut slots: Vec<(u8, usize)> = vec![];
        for i in 0..q.deck.len() {
            if !pin.contains(&q.deck[i].uid) { slots.push((0, i)); }
        }
        for i in 0..q.shields.len() {
            if !q.shields[i].faceup && !pin.contains(&q.shields[i].card.uid) { slots.push((1, i)); }
        }
        if hidden_hand {
            for i in 0..q.hand.len() {
                if !pin.contains(&q.hand[i].uid) { slots.push((2, i)); }
            }
        }
        let mut pool: Vec<Card> = slots.iter().map(|&(z, i)| match z { 0 => q.deck[i], 1 => q.shields[i].card, _ => q.hand[i] }).collect();
        rng.shuffle(&mut pool);
        for (&(z, i), c) in slots.iter().zip(pool) {
            match z {
                0 => q.deck[i] = c,
                1 => q.shields[i].card = c,
                _ => q.hand[i] = c,
            }
        }
        // EXライフの紐づけは uid で持っているので、裏向きシールドの uid が変わると外れる。
        // 近似：紐づいたシールドは位置で追う
    }
    // EXライフの紐づけを位置で直す
    for p in 0..2 {
        let olds: Vec<u16> = g.ps[p].shields.iter().map(|x| x.card.uid).collect();
        let news: Vec<u16> = s.ps[p].shields.iter().map(|x| x.card.uid).collect();
        for c in s.ps[p].battle.iter_mut() {
            if let Some(u) = c.exlife {
                if let Some(i) = olds.iter().position(|&x| x == u) {
                    c.exlife = Some(news[i]);
                }
            }
        }
    }
    s.rng = Rng::new(rng.next_u64());
    s
}

fn card_of(g: &Game, uid: u16) -> (u8, u8) {
    // (カードの種類, ゾーン)
    for p in 0..2 {
        let q = &g.ps[p];
        let z = (p as u8) * 8;
        if let Some(c) = q.hand.iter().find(|c| c.uid == uid) {
            return (c.c as u8, z + 1);
        }
        if let Some(c) = q.battle.iter().find(|c| c.card.uid == uid) {
            return (c.card.c as u8, z + 2);
        }
        if let Some(c) = q.tama.iter().find(|c| c.uid == uid) {
            return (c.c as u8, z + 7);
        }
        if let Some(m) = q.mana.iter().find(|m| m.card.uid == uid) {
            return (m.card.c as u8, z + 3);
        }
        if let Some(s) = q.shields.iter().find(|s| s.card.uid == uid) {
            // 裏向きシールドは種類を区別しない（見えない）
            return (if s.faceup { s.card.c as u8 } else { 255 }, z + 4);
        }
        if let Some(c) = q.grave.iter().find(|c| c.uid == uid) {
            return (c.c as u8, z + 5);
        }
        if let Some(c) = q.deck.iter().find(|c| c.uid == uid) {
            return (c.c as u8, z + 6);
        }
    }
    (254, 15)
}

/// 選択肢を、見えない札の並べ方によらない鍵にする（同じ種類のカードは同じ鍵）
pub fn key(g: &Game, d: &Decision, o: Opt) -> u64 {
    let k = |a: u64, b: u64| a.wrapping_mul(1_000_003) ^ b;
    let base = (d.kind as u64) << 56;
    match o {
        Opt::Pass => k(base, 1),
        Opt::Yes => k(base, 2),
        Opt::No => k(base, 3),
        Opt::Mode(m) => k(base, 100 + m as u64),
        Opt::Charge(u) => k(k(base, 4), card_of(g, u).0 as u64),
        Opt::Play { uid, side, how } => k(k(k(k(base, 5), card_of(g, uid).0 as u64), side as u64), how as u64),
        Opt::Attack { att, tgt } => {
            let t = match tgt {
                Tgt::Player => 999,
                Tgt::Cre(u) => card_of(g, u).0 as u64,
            };
            k(k(k(base, 6), card_of(g, att).0 as u64), t)
        }
        Opt::Block(u) => k(k(base, 7), card_of(g, u).0 as u64),
        Opt::Pick(u) => {
            let (c, z) = card_of(g, u);
            k(k(k(base, 8), c as u64), z as u64)
        }
    }
}

/// プレイアウト用の簡単な方針：パスは選びにくくする
pub fn rollout_pick(g: &Game, rng: &mut Rng) -> usize {
    let d = g.decision.as_ref().unwrap();
    let n = d.opts.len();
    match d.kind {
        DK::Charge => {
            if n > 1 && (g.ps[d.player].mana.len() < 7 || rng.below(3) == 0) {
                1 + rng.below(n - 1)
            } else {
                0
            }
        }
        DK::Main | DK::Attack => {
            if n > 1 && rng.below(10) != 0 {
                1 + rng.below(n - 1)
            } else {
                0
            }
        }
        DK::Trigger => d.opts.iter().position(|&o| o == Opt::Yes).unwrap_or(0),
        _ => rng.below(n),
    }
}

/// ルールAI（簡易AI相当）：チャージする・一番重いカードを出す・プレイヤーを攻撃・トリガーは使う
pub fn greedy_pick(g: &Game, rng: &mut Rng) -> usize {
    let d = g.decision.as_ref().unwrap();
    let n = d.opts.len();
    let p = d.player;
    let cost = |o: &Opt| -> i32 {
        match *o {
            Opt::Play { uid, side, .. } => g.ps[p].hand.iter().chain(g.ps[p].grave.iter()).find(|c| c.uid == uid).map(|c| crate::effects::side_of(c.c, side).cost as i32).unwrap_or(0),
            _ => -1,
        }
    };
    match d.kind {
        DK::Charge => {
            if n > 1 && g.ps[p].mana.len() < 8 {
                // 一番安いカードを残し、手札で一番使いにくい（重い）カードをチャージ…ではなく、
                // 近似：ランダムにチャージ
                1 + rng.below(n - 1)
            } else {
                0
            }
        }
        DK::Main => {
            let mut best = (0i32, 0usize);
            for (i, o) in d.opts.iter().enumerate() {
                let c = cost(o);
                if c > best.0 || (c == best.0 && c > 0 && rng.below(2) == 0) {
                    best = (c, i);
                }
            }
            best.1
        }
        DK::Attack => {
            let pl: Vec<usize> = (0..n).filter(|&i| matches!(d.opts[i], Opt::Attack { tgt: Tgt::Player, .. })).collect();
            if !pl.is_empty() {
                return pl[rng.below(pl.len())];
            }
            let cr: Vec<usize> = (1..n).collect();
            if !cr.is_empty() && rng.below(2) == 0 { cr[rng.below(cr.len())] } else { 0 }
        }
        DK::Block => {
            if n > 1 && g.ps[p].shields.len() <= 2 { 1 + rng.below(n - 1) } else { 0 }
        }
        DK::Trigger => d.opts.iter().position(|&o| o == Opt::Yes).unwrap_or(0),
        _ => {
            // 効果の選択：パス以外を優先
            let np: Vec<usize> = (0..n).filter(|&i| d.opts[i] != Opt::Pass && d.opts[i] != Opt::No).collect();
            if np.is_empty() { rng.below(n) } else { np[rng.below(np.len())] }
        }
    }
}

pub fn playout(g: &mut Game, rng: &mut Rng) {
    let mut k = 0;
    while g.winner.is_none() {
        let i = if rng.below(5) == 0 { rollout_pick(g, rng) } else { greedy_pick(g, rng) };
        g.step(i);
        k += 1;
        if k > 3000 {
            g.winner = Some(2);
        }
    }
}

pub fn score(w: usize, p: usize) -> f64 {
    if w == 2 { 0.5 } else if w == p { 1.0 } else { 0.0 }
}

// ---------- ISMCTS ----------

struct Node {
    key: u64,
    player: usize, // この手を選んだプレイヤー
    visits: u32,
    avail: u32,
    wins: f64,
    children: Vec<usize>,
}

pub struct Ismcts {
    pub iters: u32,
    pub c: f64,
}

impl Ismcts {
    pub fn choose(&self, g: &Game, rng: &mut Rng) -> usize {
        let d = g.decision.as_ref().unwrap();
        if d.opts.len() == 1 {
            return 0;
        }
        let me = d.player;
        let mut nodes = vec![Node { key: 0, player: 1 - me, visits: 0, avail: 0, wins: 0.0, children: vec![] }];
        for _ in 0..self.iters {
            let mut s = determinize(g, me, rng);
            let mut path = vec![0usize];
            let mut cur = 0usize;
            let mut expanded = false;
            while s.winner.is_none() && !expanded {
                let d = s.decision.as_ref().unwrap();
                let keys: Vec<u64> = d.opts.iter().map(|&o| key(&s, d, o)).collect();
                let player = d.player;
                // 合法な子の avail を増やす
                let mut best: Option<(f64, usize, usize)> = None; // (値, 子, 選択肢)
                let mut untried = vec![];
                for (i, &k) in keys.iter().enumerate() {
                    if keys[..i].contains(&k) {
                        continue;
                    }
                    match nodes[cur].children.iter().copied().find(|&ch| nodes[ch].key == k) {
                        Some(ch) => {
                            nodes[ch].avail += 1;
                            let n = &nodes[ch];
                            let v = n.wins / n.visits.max(1) as f64 + self.c * ((n.avail as f64).ln() / n.visits.max(1) as f64).sqrt();
                            if best.map(|b| v > b.0).unwrap_or(true) {
                                best = Some((v, ch, i));
                            }
                        }
                        None => untried.push(i),
                    }
                }
                if !untried.is_empty() {
                    let i = untried[rng.below(untried.len())];
                    nodes.push(Node { key: keys[i], player, visits: 0, avail: 1, wins: 0.0, children: vec![] });
                    let ch = nodes.len() - 1;
                    nodes[cur].children.push(ch);
                    s.step(i);
                    path.push(ch);
                    expanded = true;
                } else {
                    let (_, ch, i) = best.unwrap();
                    s.step(i);
                    path.push(ch);
                    cur = ch;
                }
            }
            playout(&mut s, rng);
            let w = s.winner.unwrap();
            for &n in &path {
                nodes[n].visits += 1;
                nodes[n].wins += score(w, nodes[n].player);
            }
        }
        // 根の選択肢のうち、訪問回数が一番多いもの
        let keys: Vec<u64> = d.opts.iter().map(|&o| key(g, d, o)).collect();
        let mut best = (0u32, 0usize);
        for (i, &k) in keys.iter().enumerate() {
            if let Some(ch) = nodes[0].children.iter().copied().find(|&ch| nodes[ch].key == k) {
                if nodes[ch].visits > best.0 {
                    best = (nodes[ch].visits, i);
                }
            }
        }
        best.1
    }
}

// ---------- 比較用：DMO 式（候補ごとに1回だけ仮の対戦をして一番良いもの） ----------

pub fn dmo_choose(g: &Game, rng: &mut Rng, playouts: u32) -> usize {
    let d = g.decision.as_ref().unwrap();
    if d.opts.len() == 1 {
        return 0;
    }
    let me = d.player;
    let mut best = (-1.0, 0usize);
    for i in 0..d.opts.len() {
        let mut tot = 0.0;
        for _ in 0..playouts {
            let mut s = determinize(g, me, rng);
            s.step(i);
            playout(&mut s, rng);
            tot += score(s.winner.unwrap(), me);
        }
        let v = tot / playouts as f64 + rng.below(1000) as f64 * 1e-9;
        if v > best.0 {
            best = (v, i);
        }
    }
    best.1
}

#[derive(Clone, Copy, Debug)]
pub enum Agent {
    Random,
    Rollout,
    Greedy,
    Dmo(u32),
    Ismcts(u32),
    /// (先読み回数, 読み込んだネットの番号)
    Az(u32, usize),
}

pub fn pick(a: Agent, g: &Game, rng: &mut Rng) -> usize {
    match a {
        Agent::Random => rng.below(g.decision.as_ref().unwrap().opts.len()),
        Agent::Rollout => rollout_pick(g, rng),
        Agent::Greedy => greedy_pick(g, rng),
        Agent::Dmo(n) => dmo_choose(g, rng, n),
        Agent::Ismcts(n) => Ismcts { iters: n, c: 0.7 }.choose(g, rng),
        Agent::Az(n, slot) => {
            let net = crate::az::get(slot);
            crate::az::Az { net: net.as_ref().as_ref(), sims: n, cpuct: 1.5, noise: false }.choose(g, rng)
        }
    }
}

/// 1局：agents[0] がデッキ d0、agents[1] がデッキ d1。勝者を返す
pub fn play_game(d0: &[C], d1: &[C], agents: [Agent; 2], seed: u64) -> usize {
    let mut g = Game::new(d0, d1, seed);
    let mut rng = Rng::new(seed ^ 0x5151);
    while g.winner.is_none() {
        let p = g.decision.as_ref().unwrap().player;
        let i = pick(agents[p], &g, &mut rng);
        g.step(i);
    }
    g.winner.unwrap()
}
