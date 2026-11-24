//! デュエマの学習用ルールエンジン（中核）。
//! すべてのプレイヤーの判断は Decision（選択肢の一覧）として外に出し、step(index) で進める。
//! カード固有の効果は effects.rs。

use crate::cards::*;
use crate::rng::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Card {
    pub c: C,
    pub uid: u16,
}

#[derive(Clone, Debug)]
pub struct Cre {
    pub card: Card,
    pub under: Vec<Card>,
    pub tapped: bool,
    pub sick: bool,
    pub hyper: bool,
    pub no_untap: bool,      // 相手のチャクラ等：次の（相手の）ターンのはじめまでアンタップしない
    pub cant_attack: bool,   // このターン攻撃できない
    pub lock: bool,          // カーネル：次の（相手の）ターンのはじめまで攻撃もブロックもできない
    pub power_mod: i32,      // このターンのパワー修整
    pub minus_inf: bool,     // このターン -∞
    pub sa: bool,            // このターン SA
    pub player_ok: bool,     // このターン相手プレイヤーを攻撃できる（スロットン・決断）
    pub exlife: Option<u16>, // EXライフのシールドの uid
    pub ignored: bool,       // このターン能力を無視（決断）
    pub no_leave: bool,      // ドギラゴン逆の極限ファイナル革命：相手の効果で離れない
    pub jd: bool,            // ジャストダイバー：次の自分のターンのはじめまで選ばれず攻撃されない
    pub bottom: u8,          // ターンの終わりに山札の下へ（1=引かない 2=1枚引く：J・O・E 3=メラベンチャー）
    pub keep: Vec<u16>,      // アイアン・マンハッタン：ブレイクしないシールド
    pub entered: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Shield {
    pub card: Card,
    pub faceup: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Mana {
    pub card: Card,
    pub tapped: bool,
    pub facedown: bool, // 水晶チャージャー：裏向き（文明なし）
}

#[derive(Clone, Debug, Default)]
pub struct Player {
    pub deck: Vec<Card>, // 末尾が山札の上
    pub hand: Vec<Card>,
    pub mana: Vec<Mana>,
    pub shields: Vec<Shield>,
    pub battle: Vec<Cre>,
    pub grave: Vec<Card>,
    pub charged: bool,
    pub spells_this_turn: u8,
    pub debt: u8,              // 逆転撃：次の自分のターンのはじめに払うマナ
    pub no_free_this_turn: bool, // ガイアッシュ条件：このターン、マナをタップせずにクリーチャーを出すか呪文を唱えた
    pub once: u32,             // 各ターンに1度の能力の使用済みビット
    pub final_rev_used: bool,
    pub no_lose: bool,         // 極限ファイナル革命：次の自分のターンのはじめまで負けない
    pub spell_lock_until: u32, // ラフルル：このターン番号の終わりまで呪文を唱えられない
    pub creature_limit_next_turn: bool, // マンハッタン：次のターン、2体以上出せない
    pub entered_this_turn: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tgt {
    Player,
    Cre(u16),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Opt {
    Pass,
    Yes,
    No,
    Charge(u16),
    /// side: 0=クリーチャー/G城/呪文（表）、1=ツインパクトの呪文側。how: 0=通常 1=J・O・E 2=G・ゼロ
    Play { uid: u16, side: u8, how: u8 },
    Attack { att: u16, tgt: Tgt },
    Block(u16),
    Pick(u16),
    Mode(u8),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DK {
    Charge,
    Main,
    Attack,
    Block,
    Trigger,  // S・トリガー/G・ストライク等を使うか
    Effect,   // 効果の中の選択
}

#[derive(Clone, Debug, PartialEq)]
pub struct Decision {
    pub player: usize,
    pub kind: DK,
    pub opts: Vec<Opt>,
    pub q: Q,
}

/// 効果の中の質問。答えは answer() で処理する。
#[derive(Clone, Debug, PartialEq)]
pub enum Q {
    None,
    Shield { att_owner: usize, left: u8, by: Option<u16>, picked: Vec<u16> }, // ブレイクするシールドを選ぶ
    UseTrigger { owner: usize, uid: u16, what: u8 },          // 0=S・トリガー 1=G・ストライク 2=逆転撃 3=ニンジャ 4=革命チェンジ
    Custom(crate::effects::EQ),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Task {
    TurnStart,
    Untap,
    StartTriggers,
    DrawStep,
    ChargeStep,
    MainStep,
    AttackStep,
    EndTriggers,
    TurnEnd,
    Ask(Decision),
    Attack { att: u16, tgt: Tgt },
    BlockStep { att: u16, tgt: Tgt },
    Hit { att: u16, tgt: Tgt },
    AttackEnd { att: u16 },
    Battle { a: (usize, u16), b: (usize, u16) },
    Break { owner: usize, n: u8, by: Option<u16> }, // owner = シールドの持ち主
    ShieldsToHand { owner: usize, uids: Vec<u16>, broken: bool },
    DirectHit { winner: usize },
    Destroy { p: usize, uid: u16, by_opp: bool },
    Draw { p: usize, n: u8 },
    Fx(crate::effects::Fx),
    CheckState,
}

#[derive(Clone, Debug)]
pub struct Game {
    pub ps: [Player; 2],
    pub tp: usize,
    pub turn: u32,
    pub rng: Rng,
    pub winner: Option<usize>, // 2 = 引き分け
    pub stack: Vec<Task>,
    pub decision: Option<Decision>,
    pub next_uid: u16,
    pub log: bool,
    pub attack_broke: bool,         // この攻撃でシールドをブレイクしたか（キリフダッシュ）
    pub attack_gyakuten_used: bool, // この攻撃中に逆転撃を使ったか
    pub attack_ninja_used: bool,
    pub attack_cancel: bool,
    pub summoned: Vec<u16>,
    pub limbo: Vec<Card>, // 効果の途中で一時的に置いているカード（キユリ）
    pub steps: u32,
}

pub const MAX_TURNS: u32 = 80;

impl Game {
    pub fn new(deck0: &[C], deck1: &[C], seed: u64) -> Game {
        let mut g = Game {
            ps: [Player::default(), Player::default()],
            tp: 0,
            turn: 0,
            rng: Rng::new(seed),
            winner: None,
            stack: vec![],
            decision: None,
            next_uid: 1,
            log: false,
            attack_broke: false,
            attack_gyakuten_used: false,
            attack_ninja_used: false,
            attack_cancel: false,
            summoned: vec![],
            limbo: vec![],
            steps: 0,
        };
        for (p, d) in [deck0, deck1].iter().enumerate() {
            for &c in d.iter() {
                let uid = g.next_uid;
                g.next_uid += 1;
                g.ps[p].deck.push(Card { c, uid });
            }
            let mut deck = std::mem::take(&mut g.ps[p].deck);
            g.rng.shuffle(&mut deck);
            g.ps[p].deck = deck;
            for _ in 0..5 {
                let c = g.ps[p].deck.pop().unwrap();
                g.ps[p].shields.push(Shield { card: c, faceup: false });
            }
            for _ in 0..5 {
                let c = g.ps[p].deck.pop().unwrap();
                g.ps[p].hand.push(c);
            }
        }
        g.tp = (g.rng.next_u64() & 1) as usize;
        g.stack.push(Task::TurnStart);
        g.run();
        g
    }

    // ---------- 進行 ----------

    pub fn run(&mut self) {
        while self.decision.is_none() && self.winner.is_none() {
            self.steps += 1;
            if self.steps > 200_000 {
                self.winner = Some(2);
                return;
            }
            let Some(t) = self.stack.pop() else {
                self.winner = Some(2);
                return;
            };
            self.exec(t);
            self.sba();
        }
    }

    /// 状況起因：パワー0以下のクリーチャーは破壊
    fn sba(&mut self) {
        if self.winner.is_some() {
            return;
        }
        for p in 0..2 {
            for i in 0..self.ps[p].battle.len() {
                if self.power(p, i) <= 0 {
                    let uid = self.ps[p].battle[i].card.uid;
                    if !self.stack.iter().any(|t| matches!(t, Task::Destroy { uid: u, .. } if *u == uid)) {
                        self.stack.push(Task::Destroy { p, uid, by_opp: false });
                    }
                }
            }
        }
    }

    pub fn step(&mut self, i: usize) {
        let d = self.decision.take().expect("no decision");
        let o = d.opts[i];
        self.answer(d, o);
        self.run();
    }

    pub fn ask(&mut self, player: usize, kind: DK, opts: Vec<Opt>, q: Q) {
        if opts.is_empty() {
            return;
        }
        if opts.len() == 1 && kind != DK::Main && kind != DK::Attack && kind != DK::Charge {
            // 選択肢が1つしか無い時は自動で進める
            let d = Decision { player, kind, opts: opts.clone(), q };
            self.answer(d, opts[0]);
            return;
        }
        self.decision = Some(Decision { player, kind, opts, q });
    }

    fn exec(&mut self, t: Task) {
        let tp = self.tp;
        match t {
            Task::TurnStart => {
                self.turn += 1;
                if self.turn > MAX_TURNS {
                    self.winner = Some(2);
                    return;
                }
                let p = &mut self.ps[tp];
                p.charged = false;
                p.spells_this_turn = 0;
                p.once = 0;
                p.no_lose = false;
                p.entered_this_turn = 0;
                p.no_free_this_turn = false;
                self.ps[1 - tp].no_free_this_turn = false;
                self.ps[1 - tp].entered_this_turn = 0;
                self.ps[1 - tp].once = 0;
                // 「次の自分のターンのはじめまで」の効果が切れる
                for c in self.ps[tp].battle.iter_mut() {
                    c.hyper = false;
                }
                for c in self.ps[1 - tp].battle.iter_mut() {
                    c.no_untap = false;
                    c.lock = false;
                }
                for c in self.ps[tp].battle.iter_mut() {
                    c.no_leave = false;
                    c.jd = false;
                }
                self.stack.push(Task::TurnEnd);
                self.stack.push(Task::EndTriggers);
                self.stack.push(Task::AttackStep);
                self.stack.push(Task::MainStep);
                self.stack.push(Task::ChargeStep);
                self.stack.push(Task::DrawStep);
                self.stack.push(Task::StartTriggers);
                self.stack.push(Task::Untap);
            }
            Task::Untap => {
                let mut untapped = vec![];
                for c in self.ps[tp].battle.iter_mut() {
                    if !c.no_untap {
                        if c.tapped {
                            untapped.push(c.card.uid);
                        }
                        c.tapped = false;
                    }
                    c.sick = false;
                }
                crate::effects::untapped(self, tp, untapped);
                for m in self.ps[tp].mana.iter_mut() {
                    m.tapped = false;
                }
            }
            Task::StartTriggers => {
                crate::effects::turn_start(self);
            }
            Task::DrawStep => {
                // 先攻1ターン目はドローしない
                if self.turn > 1 {
                    self.stack.push(Task::Draw { p: tp, n: 1 });
                }
            }
            Task::ChargeStep => {
                let mut opts = vec![Opt::Pass];
                for c in self.ps[tp].hand.iter() {
                    opts.push(Opt::Charge(c.uid));
                }
                self.ask(tp, DK::Charge, opts, Q::None);
            }
            Task::MainStep => {
                let mut opts = vec![Opt::Pass];
                crate::effects::main_options(self, &mut opts);
                self.ask(tp, DK::Main, opts, Q::None);
            }
            Task::AttackStep => {
                let mut opts = vec![Opt::Pass];
                self.attack_options(&mut opts);
                self.ask(tp, DK::Attack, opts, Q::None);
            }
            Task::EndTriggers => {
                crate::effects::turn_end(self);
            }
            Task::TurnEnd => {
                for p in 0..2 {
                    for c in self.ps[p].battle.iter_mut() {
                        c.cant_attack = false;
                        c.power_mod = 0;
                        c.minus_inf = false;
                        c.sa = false;
                        c.player_ok = false;
                        c.ignored = false;
                    }
                }
                // ラフルル：相手のターンの終わりまで
                        self.ps[tp].creature_limit_next_turn = false;
                self.tp = 1 - tp;
                self.stack.push(Task::TurnStart);
            }
            Task::Ask(d) => {
                self.ask(d.player, d.kind, d.opts, d.q);
            }
            Task::Attack { att, tgt } => {
                // 攻撃する時の能力 → ブロック → 結果 → 攻撃の終わり
                self.attack_broke = false;
                self.attack_gyakuten_used = false;
                self.attack_ninja_used = false;
                self.attack_cancel = false;
                if let Some(i) = self.find(tp, att) {
                    self.ps[tp].battle[i].tapped = true;
                }
                self.stack.push(Task::AttackEnd { att });
                self.stack.push(Task::Hit { att, tgt });
                self.stack.push(Task::BlockStep { att, tgt });
                crate::effects::on_attack(self, att, tgt);
            }
            Task::BlockStep { att, tgt } => {
                if self.find(tp, att).is_none() || self.attack_cancel || crate::effects::unblockable(self, att) {
                    return;
                }
                let op = 1 - tp;
                let mut opts = vec![Opt::Pass];
                for c in self.ps[op].battle.iter() {
                    if !c.tapped && !c.lock && self.has_kw(op, c, K_BLOCKER) && Tgt::Cre(c.card.uid) != tgt {
                        opts.push(Opt::Block(c.card.uid));
                    }
                }
                if opts.len() > 1 {
                    self.ask(op, DK::Block, opts, Q::None);
                }
            }
            Task::Hit { att, tgt } => {
                if self.attack_cancel || self.find(tp, att).is_none() {
                    return;
                }
                match tgt {
                    Tgt::Cre(u) => {
                        if self.find(1 - tp, u).is_some() {
                            self.stack.push(Task::Battle { a: (tp, att), b: (1 - tp, u) });
                        }
                    }
                    Tgt::Player => {
                        let op = 1 - tp;
                        if self.ps[op].shields.is_empty() {
                            self.stack.push(Task::DirectHit { winner: tp });
                        } else {
                            let keep = self.ps[tp].battle[self.find(tp, att).unwrap()].keep.clone();
                            let rest: Vec<u16> = self.ps[op].shields.iter().map(|s| s.card.uid).filter(|u| !keep.contains(u)).collect();
                            if !keep.is_empty() && !rest.is_empty() {
                                // アイアン・マンハッタン：選んだ2つ以外をすべてブレイク
                                self.stack.push(Task::ShieldsToHand { owner: op, uids: rest, broken: true });
                            } else {
                                let n = self.breaker(tp, att);
                                self.stack.push(Task::Break { owner: op, n, by: Some(att) });
                            }
                        }
                    }
                }
            }
            Task::AttackEnd { att } => {
                crate::effects::attack_end(self, att);
            }
            Task::Battle { a, b } => {
                let (Some(ia), Some(ib)) = (self.find(a.0, a.1), self.find(b.0, b.1)) else { return };
                let pa = self.power(a.0, ia);
                let pb = self.power(b.0, ib);
                // 後で積んだものが先に処理される
                if pa <= pb {
                    self.stack.push(Task::Destroy { p: a.0, uid: a.1, by_opp: false });
                }
                if pb <= pa {
                    self.stack.push(Task::Destroy { p: b.0, uid: b.1, by_opp: false });
                }
                crate::effects::after_battle(self, a, b, pa, pb);
            }
            Task::Break { owner, n, by } => {
                if self.ps[owner].shields.is_empty() {
                    return;
                }
                let n = n.min(self.ps[owner].shields.len() as u8);
                if n as usize == self.ps[owner].shields.len() {
                    let uids: Vec<u16> = self.ps[owner].shields.iter().map(|s| s.card.uid).collect();
                    self.stack.push(Task::ShieldsToHand { owner, uids, broken: true });
                } else {
                    // 攻撃側がブレイクするシールドを選ぶ（1枚ずつ）
                    self.ask_shield(owner, n, by, vec![]);
                }
            }
            Task::ShieldsToHand { owner, uids, broken } => {
                crate::effects::shields_to_hand(self, owner, uids, broken);
            }
            Task::DirectHit { winner } => {
                if !self.ps[1 - winner].no_lose {
                    self.winner = Some(winner);
                }
            }
            Task::Destroy { p, uid, by_opp } => {
                crate::effects::destroy(self, p, uid, by_opp);
            }
            Task::Draw { p, n } => {
                for _ in 0..n {
                    if !self.draw1(p) {
                        return;
                    }
                }
            }
            Task::Fx(fx) => crate::effects::run_fx(self, fx),
            Task::CheckState => self.check_state(),
        }
    }

    fn ask_shield(&mut self, owner: usize, left: u8, by: Option<u16>, picked: Vec<u16>) {
        let opts: Vec<Opt> = self.ps[owner].shields.iter().filter(|s| !picked.contains(&s.card.uid)).map(|s| Opt::Pick(s.card.uid)).collect();
        let attacker = 1 - owner;
        self.ask(attacker, DK::Effect, opts, Q::Shield { att_owner: attacker, left, by, picked });
    }

    fn answer(&mut self, d: Decision, o: Opt) {
        let tp = self.tp;
        match (d.kind, o) {
            (DK::Charge, Opt::Charge(uid)) => {
                let p = &mut self.ps[tp];
                let i = p.hand.iter().position(|c| c.uid == uid).unwrap();
                let c = p.hand.remove(i);
                let multi = mana_tapped_on_entry(c.c);
                p.mana.push(Mana { card: c, tapped: multi, facedown: false });
                p.charged = true;
            }
            (DK::Charge, _) => {}
            (DK::Main, Opt::Pass) => {}
            (DK::Main, Opt::Play { uid, side, how }) => {
                self.stack.push(Task::MainStep);
                crate::effects::play_from_hand(self, tp, uid, side, how);
            }
            (DK::Attack, Opt::Pass) => {}
            (DK::Attack, Opt::Attack { att, tgt }) => {
                self.stack.push(Task::AttackStep);
                self.stack.push(Task::Attack { att, tgt });
            }
            (DK::Block, Opt::Pass) => {}
            (DK::Block, Opt::Block(b)) => {
                // 攻撃先をブロッカーに変更
                let op = 1 - tp;
                if let Some(i) = self.find(op, b) {
                    self.ps[op].battle[i].tapped = true;
                }
                // 積まれている Hit を書き換える
                for t in self.stack.iter_mut().rev() {
                    if let Task::Hit { tgt, .. } = t {
                        *tgt = Tgt::Cre(b);
                        break;
                    }
                }
                crate::effects::on_block(self, op, b);
            }
            (_, o) => match d.q {
                Q::Shield { att_owner, left, by, mut picked } => {
                    let Opt::Pick(uid) = o else { return };
                    let owner = 1 - att_owner;
                    picked.push(uid);
                    if (picked.len() as u8) < left {
                        self.ask_shield(owner, left, by, picked);
                    } else {
                        self.stack.push(Task::ShieldsToHand { owner, uids: picked, broken: true });
                    }
                }
                Q::UseTrigger { owner, uid, what } => {
                    if o == Opt::Yes {
                        crate::effects::use_trigger(self, owner, uid, what);
                    }
                }
                Q::Custom(eq) => crate::effects::answer(self, d.player, eq, o),
                Q::None => {}
            },
        }
    }

    // ---------- 判定 ----------

    pub fn find(&self, p: usize, uid: u16) -> Option<usize> {
        self.ps[p].battle.iter().position(|c| c.card.uid == uid)
    }

    pub fn find_any(&self, uid: u16) -> Option<(usize, usize)> {
        for p in 0..2 {
            if let Some(i) = self.find(p, uid) {
                return Some((p, i));
            }
        }
        None
    }

    pub fn kw(&self, p: usize, c: &Cre) -> u32 {
        let d = def(c.card.c);
        let mut k = d.main.kw;
        if c.hyper {
            if let Some((_, hk)) = d.hyper {
                k |= hk;
            }
        }
        if c.sa {
            k |= K_SA;
        }
        if c.ignored {
            return 0;
        }
        k | crate::effects::extra_kw(self, p, c)
    }

    pub fn has_kw(&self, p: usize, c: &Cre, k: u32) -> bool {
        self.kw(p, c) & k != 0
    }

    pub fn power(&self, p: usize, i: usize) -> i32 {
        let c = &self.ps[p].battle[i];
        if c.minus_inf {
            return i32::MIN / 2;
        }
        let d = def(c.card.c);
        let mut pw = d.main.power;
        if c.hyper {
            if let Some((hp, _)) = d.hyper {
                pw = hp;
            }
        }
        pw + c.power_mod + crate::effects::power_bonus(self, p, i)
    }

    pub fn breaker(&self, p: usize, uid: u16) -> u8 {
        let Some(i) = self.find(p, uid) else { return 1 };
        let c = &self.ps[p].battle[i];
        let k = self.kw(p, c);
        let mut n = if k & K_TB != 0 {
            3
        } else if k & K_WB != 0 {
            2
        } else {
            1
        };
        if crate::effects::powered_breaker(self, p, c) {
            let pw = self.power(p, i);
            n = n.max(1 + (pw / 6000) as u8);
        }
        n
    }

    pub fn can_attack(&self, p: usize, c: &Cre) -> bool {
        if c.tapped {
            return false;
        }
        // 相手のカードによる「攻撃できない」（コットン・G・ストライク・カーネル）。レッドゾーンがいれば無視
        if (c.cant_attack || c.lock) && !crate::effects::has(self, p, C::RedZone) {
            return false;
        }
        crate::effects::can_attack_extra(self, p, c)
    }

    fn attack_options(&self, opts: &mut Vec<Opt>) {
        let tp = self.tp;
        let op = 1 - tp;
        for c in self.ps[tp].battle.iter() {
            if !self.can_attack(tp, c) {
                continue;
            }
            let k = self.kw(tp, c);
            let sick = c.sick && k & K_SA == 0;
            let mach = k & K_MACH != 0 && c.entered == self.turn;
            if (!sick || c.player_ok) && crate::effects::can_attack_player(self, tp, c) {
                opts.push(Opt::Attack { att: c.card.uid, tgt: Tgt::Player });
            }
            for e in self.ps[op].battle.iter() {
                if e.jd {
                    continue;
                }
                if (e.tapped && !sick) || mach {
                    opts.push(Opt::Attack { att: c.card.uid, tgt: Tgt::Cre(e.card.uid) });
                }
            }
        }
    }

    pub fn draw1(&mut self, p: usize) -> bool {
        if self.ps[p].deck.is_empty() {
            return true;
        }
        let c = self.ps[p].deck.pop().unwrap();
        self.ps[p].hand.push(c);
        if self.ps[p].deck.is_empty() {
            // 山札の最後の1枚を引いた：シャコガイルがいれば勝ち、いなければ山札0で負け
            if self.ps[p].battle.iter().any(|c| c.card.c == C::Shacogail && !c.ignored) {
                self.winner = Some(p);
                return false;
            }
        }
        self.check_state();
        self.winner.is_none()
    }

    pub fn check_state(&mut self) {
        for p in 0..2 {
            if self.ps[p].deck.is_empty() && self.winner.is_none() && !self.ps[p].no_lose {
                self.winner = Some(1 - p);
            }
        }
    }

    pub fn untapped_mana(&self, p: usize) -> usize {
        self.ps[p].mana.iter().filter(|m| !m.tapped).count()
    }

    /// コスト cost・文明 civ を払えるマナの選び方（タップするマナの添字）。払えなければ None。
    pub fn pay_plan(&self, p: usize, civ: u8, cost: usize) -> Option<Vec<usize>> {
        let mana = &self.ps[p].mana;
        let free: Vec<usize> = (0..mana.len()).filter(|&i| !mana[i].tapped).collect();
        let cost = cost.max(civ.count_ones() as usize).max(1.min(cost));
        if free.len() < cost {
            return None;
        }
        // 必要な文明それぞれに1枚ずつ割り当て（文明の少ないカードから）→ 残りは文明の多いカードを温存
        let need: Vec<u8> = (0..5).map(|b| 1u8 << b).filter(|b| civ & b != 0).collect();
        let mut used = vec![false; mana.len()];
        let mut plan = vec![];
        // 小さな全探索：need は最大3
        fn assign(need: &[u8], k: usize, free: &[usize], mana: &[Mana], used: &mut Vec<bool>, plan: &mut Vec<usize>) -> bool {
            if k == need.len() {
                return true;
            }
            let mut cand: Vec<usize> = free.iter().copied().filter(|&i| !used[i] && mcv(&mana[i]) & need[k] != 0).collect();
            cand.sort_by_key(|&i| mcv(&mana[i]).count_ones());
            for i in cand {
                used[i] = true;
                plan.push(i);
                if assign(need, k + 1, free, mana, used, plan) {
                    return true;
                }
                plan.pop();
                used[i] = false;
            }
            false
        }
        if !assign(&need, 0, &free, mana, &mut used, &mut plan) {
            return None;
        }
        let mut rest: Vec<usize> = free.iter().copied().filter(|&i| !used[i]).collect();
        rest.sort_by_key(|&i| mcv(&mana[i]).count_ones());
        for i in rest {
            if plan.len() >= cost {
                break;
            }
            plan.push(i);
        }
        if plan.len() < cost {
            return None;
        }
        Some(plan)
    }

    pub fn pay(&mut self, p: usize, plan: &[usize]) {
        for &i in plan {
            self.ps[p].mana[i].tapped = true;
        }
    }

    // ---------- ゾーン移動の小道具 ----------

    pub fn take_hand(&mut self, p: usize, uid: u16) -> Option<Card> {
        let i = self.ps[p].hand.iter().position(|c| c.uid == uid)?;
        Some(self.ps[p].hand.remove(i))
    }

    pub fn take_mana(&mut self, p: usize, uid: u16) -> Option<Card> {
        let i = self.ps[p].mana.iter().position(|m| m.card.uid == uid)?;
        Some(self.ps[p].mana.remove(i).card)
    }

    pub fn put_mana(&mut self, p: usize, c: Card, tapped: bool) {
        let t = tapped || mana_tapped_on_entry(c.c);
        self.ps[p].mana.push(Mana { card: c, tapped: t, facedown: false });
    }

    pub fn mill_to_mana(&mut self, p: usize, n: usize, tapped: bool) -> usize {
        let mut k = 0;
        for _ in 0..n {
            if let Some(c) = self.ps[p].deck.pop() {
                self.put_mana(p, c, tapped);
                k += 1;
            }
        }
        self.check_state();
        k
    }

    pub fn new_cre(&self, c: Card) -> Cre {
        Cre {
            card: c,
            under: vec![],
            tapped: false,
            sick: true,
            hyper: false,
            no_untap: false,
            cant_attack: false,
            lock: false,
            power_mod: 0,
            minus_inf: false,
            sa: false,
            player_ok: false,
            exlife: None,
            ignored: false,
            no_leave: false,
            jd: false,
            bottom: 0,
            keep: vec![],
            entered: self.turn,
        }
    }

    pub fn total_cards(&self, p: usize) -> usize {
        let q = &self.ps[p];
        q.deck.len() + q.hand.len() + q.mana.len() + q.shields.len() + q.grave.len()
            + q.battle.iter().map(|c| 1 + c.under.len()).sum::<usize>()
            + if p == self.tp { self.limbo.len() } else { 0 }
    }
}

pub fn mcv(m: &Mana) -> u8 {
    if m.facedown { 0 } else { mana_civ(m.card.c) }
}

pub fn mana_civ(c: C) -> u8 {
    let d = def(c);
    d.main.civ | d.spell.map(|s| s.civ).unwrap_or(0)
}

/// 多色カードはタップしてマナに置かれる
pub fn mana_tapped_on_entry(c: C) -> bool {
    mana_civ(c).count_ones() >= 2
}
