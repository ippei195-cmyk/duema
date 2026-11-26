//! 効果文から作った「手順」（steps）を実行する小さな仕組み（2026-11-26）。
//! 手順は対戦卓の「処理の変更」と同じ形（JSON）。textfx/compile.py が JSON を下の Op の並びに変換して
//! textfx_progs.rs を作る。対局中に AI に考えさせることはない。
//! 誘発のタイミングは effects.rs の手書きの処理と同じ所から呼ぶ（手書きの処理があるカードには使わない）。

use crate::cards::*;
use crate::effects::{self, Dest, Fx, EQ, K};
use crate::engine::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Who { Me, Opp }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Zone { Hand, Mana, Grave, Battle, Shield, Deck }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Src { Var(u8), Zone(Who, Zone), SelfCard }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum To { Hand, Mana, ManaTapped, Grave, Shield, DeckTop, DeckBottom, Battle }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmp { Ge, Le, Eq }

/// カードの条件（0 は「指定なし」）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Filt {
    pub kinds: u8,     // bit: 1=クリーチャー 2=呪文 4=G城（フィールド） 8=タマシード
    pub civ: u8,       // どれかの文明を持つ
    pub race: u16,     // どれかの種族を持つ
    pub kw: u32,       // すべて持つ
    pub min_cost: u8,
    pub max_cost: u8,  // 255=指定なし
    pub min_power: i32,
    pub max_power: i32, // 0=指定なし
    pub non_evo: bool,
    pub tapped: u8,    // 0=どちらでも 1=タップ 2=アンタップ
    pub mono: bool,    // 多色ではない
}

pub const ANY: Filt = Filt { kinds: 0, civ: 0, race: 0, kw: 0, min_cost: 0, max_cost: 255, min_power: 0, max_power: 0, non_evo: false, tapped: 0, mono: false };

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Op {
    Look { who: Who, n: u8, var: u8 },
    Choose { from: Src, f: Filt, min: u8, max: u8, var: u8, by: Who },
    Filter { from: Src, f: Filt, var: u8 },
    Random { from: Src, f: Filt, n: u8, var: u8 }, // 見ないで選ぶ
    Move { var: u8, except: u8, to: To }, // except=255 なし
    Destroy { var: u8 },
    Draw { who: Who, n: u8 },
    TopToMana { who: Who, n: u8, tapped: bool },
    TopToShield { who: Who, n: u8 },
    Tap { var: u8 },
    Untap { var: u8 },
    Freeze { var: u8 },
    NoAttack { var: u8 },
    Power { var: u8, delta: i32 },
    Optional { skip: u16 },                  // 「〜してもよい」：使わなければ skip へ
    Mode { targets: &'static [u16] },        // どれか1つ選んで targets[i] へ
    If { who: Who, zone: Zone, f: Filt, cmp: Cmp, n: u8, els: u16 },
    Jump(u16),
    Win,
    Lose { who: Who },
    End,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ev { Etb, Spell(u8), Attack, TurnStart, TurnEnd, Destroyed }

pub const NVARS: usize = 6;

#[derive(Clone, Debug, PartialEq)]
pub struct Vm {
    pub p: usize,     // 能力の持ち主
    pub src: u16,
    pub prog: &'static [Op],
    pub pc: u16,
    pub vars: [Vec<u16>; NVARS],
}

thread_local! {
    /// テスト用：手書きの処理があるカードでも、ここに入れた手順を使う
    pub static TEST_PROGS: std::cell::RefCell<Vec<(C, Ev, &'static [Op])>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// カード c のタイミング ev の手順（無ければ None）
pub fn prog(c: C, ev: Ev) -> Option<&'static [Op]> {
    let t = TEST_PROGS.with(|t| t.borrow().iter().find(|(x, e, _)| *x == c && *e == ev).map(|x| x.2));
    if t.is_some() {
        return t;
    }
    crate::textfx_progs::PROGS.iter().find(|(x, e, _)| *x == c && *e == ev).map(|x| x.2)
}

pub fn test_override(c: C, ev: Ev) -> bool {
    TEST_PROGS.with(|t| t.borrow().iter().any(|(x, e, _)| *x == c && *e == ev))
}

pub fn start(g: &mut Game, p: usize, src: u16, prog: &'static [Op]) {
    let vm = Vm { p, src, prog, pc: 0, vars: Default::default() };
    run(g, vm);
}

fn pl(vm: &Vm, w: Who) -> usize {
    if w == Who::Me { vm.p } else { 1 - vm.p }
}

pub fn matches(g: &Game, uid: u16, f: &Filt) -> bool {
    let Some((q, z)) = locate(g, uid) else { return false };
    let c = card_of(g, q, z, uid).unwrap();
    let d = def(c);
    let s = d.main;
    if f.kinds != 0 {
        let k = match s.kind { Kind::Creature => 1, Kind::Spell => 2, Kind::GCastle => 4, Kind::Tama => 8 }
            | if d.spell.is_some() { 2 } else { 0 };
        if k & f.kinds == 0 {
            return false;
        }
    }
    let civ = s.civ | d.spell.map(|x| x.civ).unwrap_or(0);
    if f.civ != 0 && civ & f.civ == 0 {
        return false;
    }
    if f.mono && civ.count_ones() > 1 {
        return false;
    }
    if f.race != 0 && s.race & f.race == 0 {
        return false;
    }
    if f.kw != 0 && s.kw & f.kw != f.kw {
        return false;
    }
    let cost = if z == Zone::Battle { s.cost } else { effects::cost_of(c) };
    if cost < f.min_cost || (f.max_cost != 255 && cost > f.max_cost) {
        return false;
    }
    if f.non_evo && d.evo != Evo::None {
        return false;
    }
    if f.min_power != 0 || f.max_power != 0 || f.tapped != 0 {
        let pw = if z == Zone::Battle { g.find(q, uid).map(|i| g.power(q, i)).unwrap_or(s.power) } else { s.power };
        if f.min_power != 0 && pw < f.min_power {
            return false;
        }
        if f.max_power != 0 && pw > f.max_power {
            return false;
        }
        if f.tapped != 0 {
            let t = match z {
                Zone::Battle => g.find(q, uid).map(|i| g.ps[q].battle[i].tapped).unwrap_or(false),
                Zone::Mana => g.ps[q].mana.iter().any(|m| m.card.uid == uid && m.tapped),
                _ => false,
            };
            if (f.tapped == 1) != t {
                return false;
            }
        }
    }
    true
}

/// uid のカードがある場所
pub fn locate(g: &Game, uid: u16) -> Option<(usize, Zone)> {
    for q in 0..2 {
        let p = &g.ps[q];
        if p.hand.iter().any(|c| c.uid == uid) { return Some((q, Zone::Hand)); }
        if p.mana.iter().any(|m| m.card.uid == uid) { return Some((q, Zone::Mana)); }
        if p.grave.iter().any(|c| c.uid == uid) { return Some((q, Zone::Grave)); }
        if p.battle.iter().any(|c| c.card.uid == uid) { return Some((q, Zone::Battle)); }
        if p.shields.iter().any(|s| s.card.uid == uid) { return Some((q, Zone::Shield)); }
        if p.deck.iter().any(|c| c.uid == uid) { return Some((q, Zone::Deck)); }
    }
    None
}

fn card_of(g: &Game, q: usize, z: Zone, uid: u16) -> Option<C> {
    let p = &g.ps[q];
    match z {
        Zone::Hand => p.hand.iter().find(|c| c.uid == uid).map(|c| c.c),
        Zone::Mana => p.mana.iter().find(|m| m.card.uid == uid).map(|m| m.card.c),
        Zone::Grave => p.grave.iter().find(|c| c.uid == uid).map(|c| c.c),
        Zone::Battle => p.battle.iter().find(|c| c.card.uid == uid).map(|c| c.card.c),
        Zone::Shield => p.shields.iter().find(|s| s.card.uid == uid).map(|s| s.card.c),
        Zone::Deck => p.deck.iter().find(|c| c.uid == uid).map(|c| c.c),
    }
}

fn zone_uids(g: &Game, q: usize, z: Zone) -> Vec<u16> {
    let p = &g.ps[q];
    match z {
        Zone::Hand => p.hand.iter().map(|c| c.uid).collect(),
        Zone::Mana => p.mana.iter().map(|m| m.card.uid).collect(),
        Zone::Grave => p.grave.iter().map(|c| c.uid).collect(),
        // 相手のクリーチャーは「選ばれない」もの（ジャストダイバー）を除く
        Zone::Battle => p.battle.iter().filter(|c| !c.jd || q == usize::MAX).map(|c| c.card.uid).collect(),
        Zone::Shield => p.shields.iter().map(|s| s.card.uid).collect(),
        Zone::Deck => p.deck.iter().rev().map(|c| c.uid).collect(),
    }
}

fn src_uids(g: &Game, vm: &Vm, s: Src) -> Vec<u16> {
    match s {
        Src::Var(v) => vm.vars[v as usize].iter().copied().filter(|&u| locate(g, u).is_some()).collect(),
        Src::SelfCard => if vm.src != 0 && locate(g, vm.src).is_some() { vec![vm.src] } else { vec![] },
        Src::Zone(w, z) => {
            let q = pl(vm, w);
            let mut l = zone_uids(g, q, z);
            if z == Zone::Battle && q == vm.p {
                l = g.ps[q].battle.iter().map(|c| c.card.uid).collect(); // 自分のクリーチャーは選べる
            }
            l
        }
    }
}

/// カードを今の場所から取り出して to へ（バトルゾーンからは effects::leave を通す）
fn move_to(g: &mut Game, vm: &Vm, uid: u16, to: To) {
    let Some((q, z)) = locate(g, uid) else { return };
    let by_opp = q != vm.p;
    if z == Zone::Battle {
        let d = match to {
            To::Hand => Dest::Hand,
            To::Mana | To::ManaTapped => Dest::Mana,
            To::Grave => Dest::Grave,
            To::Shield => Dest::Shield,
            To::DeckBottom | To::DeckTop => Dest::DeckBottom, // 近似：山札の上へ戻す効果も下へ
            To::Battle => return,
        };
        effects::leave(g, q, uid, d, by_opp);
        return;
    }
    let card = match z {
        Zone::Hand => g.take_hand(q, uid),
        Zone::Mana => g.take_mana(q, uid),
        Zone::Grave => g.ps[q].grave.iter().position(|c| c.uid == uid).map(|i| g.ps[q].grave.remove(i)),
        Zone::Deck => g.ps[q].deck.iter().position(|c| c.uid == uid).map(|i| g.ps[q].deck.remove(i)),
        Zone::Shield => effects::take_shield_pub(g, q, uid).map(|(s, under)| {
            for u in under {
                g.ps[q].hand.push(u); // 近似：重なっていた下のカードは手札へ
            }
            g.stack.push(Task::Fx(Fx::Zrush { p: q })); // シールドが離れた時（Zラッシュ）
            s.card
        }),
        Zone::Battle => None,
    };
    let Some(card) = card else { return };
    match to {
        To::Hand => g.ps[q].hand.push(card),
        To::Mana => g.put_mana(q, card, false),
        To::ManaTapped => g.put_mana(q, card, true),
        To::Grave => g.ps[q].grave.push(card),
        To::Shield => effects::shield_stack_pub(g, q, vec![card]),
        To::DeckTop => g.ps[q].deck.push(card),
        To::DeckBottom => g.ps[q].deck.insert(0, card),
        To::Battle => {
            if effects::is_creature(card.c) {
                effects::enter(g, q, card, effects::How::Put, None, 0);
            } else {
                g.ps[q].grave.push(card); // 近似：クリーチャー以外は出さない
            }
        }
    }
    g.check_state();
}

fn count(g: &Game, vm: &Vm, w: Who, z: Zone, f: &Filt) -> usize {
    let q = pl(vm, w);
    let l: Vec<u16> = match z {
        Zone::Battle => g.ps[q].battle.iter().map(|c| c.card.uid).collect(),
        _ => zone_uids(g, q, z),
    };
    if *f == ANY { l.len() } else { l.into_iter().filter(|&u| matches(g, u, f)).count() }
}

fn cre_mut<'a>(g: &'a mut Game, uid: u16) -> Option<&'a mut Cre> {
    for q in 0..2 {
        if let Some(i) = g.find(q, uid) {
            return Some(&mut g.ps[q].battle[i]);
        }
    }
    None
}

/// 次の選択まで手順を進める
pub fn run(g: &mut Game, mut vm: Vm) {
    loop {
        if g.winner.is_some() {
            return;
        }
        let Some(&op) = vm.prog.get(vm.pc as usize) else { return };
        vm.pc += 1;
        match op {
            Op::End => return,
            Op::Jump(t) => vm.pc = t,
            Op::Look { who, n, var } => {
                let q = pl(&vm, who);
                let k = g.ps[q].deck.len();
                vm.vars[var as usize] = g.ps[q].deck[k.saturating_sub(n as usize)..].iter().rev().map(|c| c.uid).collect();
            }
            Op::Filter { from, f, var } => {
                let l: Vec<u16> = src_uids(g, &vm, from).into_iter().filter(|&u| matches(g, u, &f)).collect();
                vm.vars[var as usize] = l;
            }
            Op::Random { from, f, n, var } => {
                let mut l: Vec<u16> = src_uids(g, &vm, from).into_iter().filter(|&u| matches(g, u, &f)).collect();
                let mut out = vec![];
                for _ in 0..n {
                    if l.is_empty() {
                        break;
                    }
                    let i = g.rng.below(l.len());
                    out.push(l.remove(i));
                }
                vm.vars[var as usize] = out;
            }
            Op::Choose { from, f, min, max, var, by } => {
                let l: Vec<u16> = src_uids(g, &vm, from).into_iter().filter(|&u| matches(g, u, &f)).collect();
                vm.vars[var as usize].clear();
                if l.is_empty() || max == 0 {
                    continue;
                }
                let chooser = pl(&vm, by);
                let pc = vm.pc;
                g.stack.push(Task::Fx(Fx::Vm(Box::new(vm))));
                effects::ask_pub(g, K::VmPick, chooser, (min.min(l.len() as u8)) as i32, max as i32, l);
                let _ = pc;
                return;
            }
            Op::Move { var, except, to } => {
                let ex: Vec<u16> = if except == 255 { vec![] } else { vm.vars[except as usize].clone() };
                let l: Vec<u16> = vm.vars[var as usize].iter().copied().filter(|u| !ex.contains(u)).collect();
                let mut l2 = l.clone();
                if to == To::DeckBottom {
                    g.rng.shuffle(&mut l2); // 「好きな順番で」：近似として順番はランダム
                } else if to == To::Battle {
                    l2.reverse(); // 出た時の能力は後に出たものから積まれる（手書きの処理と同じ順番）
                }
                for u in l2 {
                    move_to(g, &vm, u, to);
                }
            }
            Op::Destroy { var } => {
                for u in vm.vars[var as usize].clone() {
                    if let Some((q, Zone::Battle)) = locate(g, u) {
                        effects::destroy(g, q, u, q != vm.p);
                    }
                }
            }
            Op::Draw { who, n } => {
                let q = pl(&vm, who);
                for _ in 0..n {
                    g.draw1(q);
                }
            }
            Op::TopToMana { who, n, tapped } => {
                g.mill_to_mana(pl(&vm, who), n as usize, tapped);
            }
            Op::TopToShield { who, n } => {
                for _ in 0..n {
                    effects::shieldify_top_pub(g, pl(&vm, who));
                }
            }
            Op::Tap { var } | Op::Untap { var } | Op::Freeze { var } | Op::NoAttack { var } | Op::Power { var, .. } => {
                for u in vm.vars[var as usize].clone() {
                    if let Some(c) = cre_mut(g, u) {
                        match op {
                            Op::Tap { .. } => c.tapped = true,
                            Op::Untap { .. } => c.tapped = false,
                            Op::Freeze { .. } => { c.tapped = true; c.no_untap = true; }
                            Op::NoAttack { .. } => c.cant_attack = true,
                            Op::Power { delta, .. } => c.power_mod += delta,
                            _ => {}
                        }
                    }
                }
                g.check_state();
            }
            Op::Optional { skip: _ } => {
                let p = vm.p;
                g.stack.push(Task::Fx(Fx::Vm(Box::new(vm))));
                effects::ask_pub(g, K::VmYes, p, 0, 0, vec![]);
                return;
            }
            Op::Mode { targets } => {
                let p = vm.p;
                g.stack.push(Task::Fx(Fx::Vm(Box::new(vm))));
                effects::ask_pub(g, K::VmMode, p, targets.len() as i32, 0, vec![]);
                return;
            }
            Op::If { who, zone, f, cmp, n, els } => {
                let k = count(g, &vm, who, zone, &f);
                let ok = match cmp { Cmp::Ge => k >= n as usize, Cmp::Le => k <= n as usize, Cmp::Eq => k == n as usize };
                if !ok {
                    vm.pc = els;
                }
            }
            Op::Win => g.winner = Some(vm.p),
            Op::Lose { who } => {
                let q = pl(&vm, who);
                if !g.ps[q].no_lose {
                    g.winner = Some(1 - q);
                }
            }
        }
    }
}

/// 一番上で待っている手順
fn top_vm(g: &mut Game) -> Option<&mut Vm> {
    for t in g.stack.iter_mut().rev() {
        if let Task::Fx(Fx::Vm(vm)) = t {
            return Some(vm);
        }
    }
    None
}

pub fn options(q: &EQ) -> Vec<Opt> {
    match q.k {
        K::VmYes => vec![Opt::No, Opt::Yes],
        K::VmMode => (0..q.a as u8).map(Opt::Mode).collect(),
        _ => {
            // VmPick：a=まだ選ばないといけない数、b=あと選べる数
            let mut v: Vec<Opt> = q.list.iter().map(|&u| Opt::Pick(u)).collect();
            if q.a <= 0 {
                v.insert(0, Opt::Pass);
            }
            v
        }
    }
}

pub fn answer(g: &mut Game, q: EQ, o: Opt) {
    match q.k {
        K::VmYes => {
            if o != Opt::Yes {
                if let Some(vm) = top_vm(g) {
                    if let Some(Op::Optional { skip }) = vm.prog.get(vm.pc as usize - 1).copied() {
                        vm.pc = skip;
                    }
                }
            }
        }
        K::VmMode => {
            if let Opt::Mode(i) = o {
                if let Some(vm) = top_vm(g) {
                    if let Some(Op::Mode { targets }) = vm.prog.get(vm.pc as usize - 1).copied() {
                        vm.pc = targets[i as usize];
                    }
                }
            }
        }
        _ => {
            let Opt::Pick(u) = o else { return };
            let var = match top_vm(g) {
                Some(vm) => match vm.prog.get(vm.pc as usize - 1).copied() {
                    Some(Op::Choose { var, .. }) => { vm.vars[var as usize].push(u); var }
                    _ => return,
                },
                None => return,
            };
            let _ = var;
            let left = q.b - 1;
            let mut l = q.list.clone();
            l.retain(|&x| x != u);
            if left > 0 && !l.is_empty() {
                let chooser = q.p;
                effects::ask_pub(g, K::VmPick, chooser, (q.a - 1).max(0).min(l.len() as i32), left, l);
            }
        }
    }
}
