//! 3デッキ40種類のカード効果。根拠は /mnt/project-files/alphazero/裁定メモ.md と card-texts.md。
//! 単純化したところは「近似：」と書く。

use crate::cards::*;
use crate::engine::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum How {
    Summon,     // マナを払って召喚
    FreeSummon, // コストを払わない召喚（S・トリガー、ニンジャ、ガイアッシュ）
    Put,        // 効果で出す（召喚ではない）
}

/// 効果の中の質問の種類
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum K {
    EvoBase,       // 進化元を選ぶ（a=side,b=how）
    Kiyuri,        // list の中から出すクリーチャー（b=残り回数）
    HumptyDiscard, // 相手の手札から捨てさせる
    HumptyKill,    // a=コスト
    GStrike,
    ChakraTap,
    DaguraShield,
    DaguraMana,
    AntihamPut,
    CottonSpell,
    Discard,       // 自分で捨てる（a=残り枚数, b=後処理 0なし 1=モウジャキンガ 2=ジョギラゴン 3=マンハッタン 4=ブランド 5=蒼斬丸）
    GyaiMill,      // Yes/No
    GyaiReturn,
    AshDraw,
    Gyakuten,      // -∞ の対象
    Yabe,          // -2000 の対象
    KernelLock,
    MiracleSpell,  // 手札からコスト5以下の呪文を唱えるか
    SozanDraw,     // 何枚引くか
    SozanCancel,   // 手札を捨てて攻撃中止するか
    LaffChange,    // 未使用
    Kusabira,      // 何枚マナへ（Mode）
    KusaOppMana,   // 相手が自分のクリーチャーを選んでマナへ（a=残り）
    KusaPut,       // マナから出す（a=残り）
    DarkMasters,   // a=残り
    JoJoJo,
    Jolly,
    FujiPick,      // 富士山の表向き3枚：全部手札へ（自動）
    SlottonPick,
    Slot7Pick,
    BrandDiscard,  // 何枚捨てるか→1枚ずつ
    BrandKill,     // a=残り
    JogimeraPick,
    MelaPut,       // a=残り
    JogiragonUse,  // 捨てたジョーカーズの出た時能力を使うか（a=card）
    MoujaShield,
    MoujaCre,
    NiyaniMode,
    JBig,          // 使う能力を選ぶ（list=候補のカード種類）
    Kesshin,       // a=残り回数
    KesshinBottom,
    KesshinAtk,
    KesshinIgnore,
    SakkiPick,
    SakkiPut,      // b=コスト上限
    ManhattanKeep, // a=残り
    ManhattanDiscard,
    DogiEnd,       // Yes/No マナへ2枚
    DogiPut,       // a=残りコスト b=残り体数
    FujisanReturn, // 富士山：戻すジョーカーズを選ぶ（a=合計）
    PPPEnd,        // Yes/No
    PPPReturn,
    GaiashSummon,  // Yes/No
    RevChange,     // 革命チェンジ（list=候補）
    DDD,           // D・D・D（Yes/No）
    DDDBase,
    Ninja,         // Yes/No
    Kirifuda,      // Yes/No
    SmashBurst,    // Yes/No
    SakkiAtk,
    ReturnShield,  // サイレント・ジェラシー：シールドを1つ手札へ
    // ---- 2026-11-26 追加（白緑ウィリデ・ゴルギーオージャー）----
    SaintRepl,     // セイント・キャッスルでかわりにブレイクさせるか（Yes/No）
    ZazeMode,      // ザゼ・ゼーン：アンタップの後のモード（list=[ザゼ]）
    EverShield,    // エバーラスト：シールド化するエレメント（Pick）か山札の上（Mode(0)）か Pass
    WillideDraw,   // ウィリデ：1枚引くか（b=コスト上限）
    WillidePut,    // ウィリデ：手札から出すクリーチャー（b=コスト上限）
    EmoName,       // ハードコア：名前を選ぶ（Mode(カード番号)）（list=[ハードコア]）
    GorgiReturn,   // ゴルギーオージャー：マナから手札へ（a=残り）
    CanzoneRaid,   // 超魂レイド：下に置くカード（list=表向きの3枚, b=カンツォーネ）
    CanzoneShield, // カンツォーネ：相手のエレメントをシールド化
    CanzoneDeath,  // カンツォーネが破壊された：下にあったカードを置くクリーチャー（Pick）か、シールド（Pass）
    AkanePick,     // 2枚から：Pick=手札 Mode(k)=list[k]をマナ
    TakaLook,      // TAKASUGI：見た4枚からクリーチャーを手札へ
    // ---- 2026-11-26 追加：効果文から作った手順（textfx.rs）----
    VmPick,        // 手順の choose（a=まだ選ばないといけない数 b=あと選べる数）
    VmYes,         // 手順の optional（Yes/No）
    VmMode,        // 手順の mode（a=選択肢の数）
    ChooseRepl,    // 未使用（必ず最後）
}

#[derive(Clone, Debug, PartialEq)]
pub struct EQ {
    pub k: K,
    pub p: usize,
    pub a: i32,
    pub b: i32,
    pub list: Vec<u16>,
}

/// 後で処理する効果
#[derive(Clone, Debug, PartialEq)]
pub enum Fx {
    Enter { p: usize, uid: u16 },            // 出た時の能力
    Ability { p: usize, src: u16, c: C },     // c の出た時能力を src が使う（Jビッグ・富士山・ジョギラゴン）
    Cast { p: usize, uid: u16, side: u8, free: bool }, // 呪文の効果
    Ask(EQ),
    Zrush { p: usize },
    ShieldPlaced { p: usize, n: u8 },
    LeftAbyss { p: usize },
    LeftJokers { p: usize, c: C },
    DragonEntered { p: usize },
    TriggerDecl { owner: usize, uids: Vec<u16> },
    MoujaEnd { p: usize, uid: u16 },
    TakaLook { p: usize },             // TAKASUGI：山札の上から4枚を見る
    FirstEvent { p: usize, cost: u8 }, // p（ターン・プレイヤー）のはじめてのエレメント／呪文（ウィリデ）
    Vm(Box<crate::textfx::Vm>),        // 効果文から作った手順の続き
}

fn push(g: &mut Game, fx: Fx) {
    g.stack.push(Task::Fx(fx));
}

fn ask(g: &mut Game, k: K, p: usize, a: i32, b: i32, list: Vec<u16>) {
    push(g, Fx::Ask(EQ { k, p, a, b, list }));
}

// textfx.rs から使う小道具
pub(crate) fn ask_pub(g: &mut Game, k: K, p: usize, a: i32, b: i32, list: Vec<u16>) {
    ask(g, k, p, a, b, list);
}
pub(crate) fn take_shield_pub(g: &mut Game, p: usize, uid: u16) -> Option<(Shield, Vec<Card>)> {
    take_shield(g, p, uid)
}
pub(crate) fn shield_stack_pub(g: &mut Game, p: usize, cards: Vec<Card>) {
    shield_stack(g, p, cards, false);
}
pub(crate) fn shieldify_top_pub(g: &mut Game, p: usize) -> Option<u16> {
    shieldify_top(g, p)
}
/// 「各ターンに１度」をこのカードで使った
fn t1_mark(g: &mut Game, p: usize, uid: u16) {
    let t = g.turn;
    if let Some(i) = g.find(p, uid) {
        g.ps[p].battle[i].t1 = t;
    }
}
pub(crate) fn untap_mana_pub(g: &mut Game, p: usize, n: usize) {
    untap_mana(g, p, n);
}
pub(crate) fn is_creature(c: C) -> bool {
    is_creature_card(c)
}

// ---------- 小道具 ----------

pub fn side_of(c: C, side: u8) -> Side {
    let d = def(c);
    if side == 1 { d.spell.unwrap() } else { d.main }
}

pub fn cost_of(c: C) -> u8 {
    def(c).main.cost
}

fn race(c: C) -> u16 {
    let d = def(c);
    d.main.race | d.spell.map(|s| s.race).unwrap_or(0)
}

pub fn is_dragon(c: C) -> bool {
    def(c).main.race & R_DRAGON != 0
}

pub fn is_jokers(c: C) -> bool {
    race(c) & R_JOKERS != 0
}

fn is_creature_card(c: C) -> bool {
    def(c).main.kind == Kind::Creature
}

fn is_multi(c: C) -> bool {
    crate::engine::mana_civ(c).count_ones() >= 2
}

/// 能力が生きているクリーチャーがいるか
pub fn has(g: &Game, p: usize, c: C) -> bool {
    g.ps[p].battle.iter().any(|x| x.card.c == c && !x.ign())
}

fn uids(g: &Game, p: usize) -> Vec<u16> {
    g.ps[p].battle.iter().map(|c| c.card.uid).collect()
}

/// 相手が選べるクリーチャー（ジャストダイバーを除く）
fn targetable(g: &Game, p: usize) -> Vec<u16> {
    g.ps[p].battle.iter().filter(|c| !c.jd).map(|c| c.card.uid).collect()
}

fn jokers_count(g: &Game, p: usize) -> usize {
    g.ps[p].battle.iter().filter(|c| is_jokers(c.card.c)).count()
        + g.ps[p].mana.iter().filter(|m| !m.facedown && is_jokers(m.card.c)).count()
}

fn picks(v: &[u16]) -> Vec<Opt> {
    v.iter().map(|&u| Opt::Pick(u)).collect()
}

fn with_pass(mut v: Vec<Opt>) -> Vec<Opt> {
    v.insert(0, Opt::Pass);
    v
}

fn bottom_random(g: &mut Game, p: usize, mut cards: Vec<Card>) {
    g.rng.shuffle(&mut cards);
    for c in cards {
        g.ps[p].deck.insert(0, c);
    }
}

fn top_n(g: &mut Game, p: usize, n: usize) -> Vec<Card> {
    let mut v = vec![];
    for _ in 0..n {
        if let Some(c) = g.ps[p].deck.pop() {
            v.push(c);
        }
    }
    v
}

/// 一時置き場（山札から見ているカード）
fn stash(g: &mut Game, p: usize, cards: &[Card]) -> Vec<u16> {
    // 見ているカードは山札の上に戻したまま uid だけ渡し、処理の最後に山札の下へ送る
    let u = cards.iter().map(|c| c.uid).collect();
    for &c in cards.iter().rev() {
        g.ps[p].deck.push(c);
    }
    u
}

fn take_deck(g: &mut Game, p: usize, uid: u16) -> Option<Card> {
    let i = g.ps[p].deck.iter().position(|c| c.uid == uid)?;
    Some(g.ps[p].deck.remove(i))
}

fn deck_card(g: &Game, p: usize, uid: u16) -> Option<C> {
    g.ps[p].deck.iter().find(|c| c.uid == uid).map(|c| c.c)
}

fn send_looked_to_bottom(g: &mut Game, p: usize, list: &[u16]) {
    let mut cs = vec![];
    for &u in list {
        if let Some(c) = take_deck(g, p, u) {
            cs.push(c);
        }
    }
    bottom_random(g, p, cs);
}

fn hand_card(g: &Game, p: usize, uid: u16) -> Option<C> {
    g.ps[p].hand.iter().find(|c| c.uid == uid).map(|c| c.c)
}

fn discard(g: &mut Game, p: usize, uid: u16) -> Option<C> {
    let c = g.take_hand(p, uid)?;
    g.ps[p].grave.push(c);
    Some(c.c)
}

fn random_discard(g: &mut Game, p: usize) {
    if g.ps[p].hand.is_empty() {
        return;
    }
    let i = g.rng.below(g.ps[p].hand.len());
    let c = g.ps[p].hand.remove(i);
    g.ps[p].grave.push(c);
}

fn fujin_blocks(g: &Game, p: usize) -> bool {
    // 相手のターン中、相手（p）はシールドゾーンにカードを置けない
    g.tp == p && has(g, 1 - p, C::Fujin)
}

/// 山札の上から1枚をシールド化。置けたら uid を返す
fn shieldify_top(g: &mut Game, p: usize) -> Option<u16> {
    if fujin_blocks(g, p) {
        return None;
    }
    let c = g.ps[p].deck.pop()?;
    g.check_state();
    g.ps[p].shields.push(Shield { card: c, faceup: false });
    push(g, Fx::ShieldPlaced { p, n: 1 });
    Some(c.uid)
}

// ---------- 2026-11-26 追加の小道具 ----------

/// how に足すビット：グリーン・グランドクロスの軽減を使う
pub const HOW_GC: u8 = 8;

pub fn is_neo(c: C) -> bool {
    matches!(def(c).evo, Evo::Neo | Evo::GNeo)
}

pub fn is_giant(c: C) -> bool {
    race(c) & R_GIANT != 0
}

/// マナゾーンの光のカードの枚数（マナ武装）
fn light_mana(g: &Game, p: usize) -> usize {
    g.ps[p].mana.iter().filter(|m| mcv(m) & L != 0).count()
}

/// 進化元にするエレメントに含まれるカードの枚数
fn base_cards(g: &Game, p: usize, b: u16) -> Option<usize> {
    if let Some(i) = g.find(p, b) {
        return Some(1 + g.ps[p].battle[i].under.len());
    }
    g.ps[p].tama.iter().any(|t| t.uid == b).then_some(1)
}

/// このクリーチャーに含まれるカードの枚数（109.2c）
fn contained(g: &Game, p: usize, uid: u16) -> usize {
    g.find(p, uid).map(|i| 1 + g.ps[p].battle[i].under.len()).unwrap_or(0)
}

/// コハクの超魂X の数（バトルゾーンのコハクと、クリーチャーの下のコハク）
fn kohaku_count(g: &Game, p: usize) -> usize {
    g.ps[p].battle.iter().filter(|x| !x.ign()).map(|x| (x.card.c == C::Kohaku) as usize + x.under.iter().filter(|u| u.c == C::Kohaku).count()).sum()
}

/// グリーン・グランドクロスの軽減がこのターンあと何回使えるか
fn gc_left(g: &Game, p: usize) -> usize {
    let n = g.ps[p].shields.iter().filter(|s| s.faceup && s.card.c == C::GrandCross).count();
    n.saturating_sub(g.ps[p].gc_used as usize)
}

pub fn gc_left_pub(g: &Game, p: usize) -> usize {
    gc_left(g, p)
}

/// これ以上クリーチャーを出せないか（マンハッタン・進化した一音の妖精）
pub fn creature_blocked(g: &Game, p: usize) -> bool {
    let ichion = g.ps[1 - p].battle.iter().any(|x| x.card.c == C::Ichion && !x.ign() && !x.under.is_empty());
    (g.ps[p].creature_limit_next_turn || ichion) && g.ps[p].entered_this_turn >= 1
}

/// 5000VT：このターン、パワー5000以下のクリーチャーを出せないか
pub fn small_locked(g: &Game, p: usize, c: C) -> bool {
    g.ps[p].small_lock_turn == g.turn && def(c).main.kind == Kind::Creature && def(c).main.power <= 5000
}

fn ddd_taka_cost(g: &Game, p: usize) -> usize {
    let n = g.ps[p].battle.iter().filter(|x| x.card.c == C::Cineraria && !x.ign()).count();
    2usize.saturating_sub(n).max(1)
}

/// マナを n 枚アンタップ（多色のカードから）
fn untap_mana(g: &mut Game, p: usize, n: usize) {
    let mut idx: Vec<usize> = (0..g.ps[p].mana.len()).filter(|&i| g.ps[p].mana[i].tapped).collect();
    idx.sort_by_key(|&i| std::cmp::Reverse(mcv(&g.ps[p].mana[i]).count_ones()));
    for &i in idx.iter().take(n) {
        g.ps[p].mana[i].tapped = false;
    }
}

/// シールドを取り出す（重なっている下のカードも）。EXライフの紐づけも外す
fn take_shield(g: &mut Game, p: usize, uid: u16) -> Option<(Shield, Vec<Card>)> {
    let si = g.ps[p].shields.iter().position(|s| s.card.uid == uid)?;
    let s = g.ps[p].shields.remove(si);
    let mut under = vec![];
    g.ps[p].sh_under.retain(|&(t, c)| {
        if t == uid {
            under.push(c);
            false
        } else {
            true
        }
    });
    for c in g.ps[p].battle.iter_mut() {
        if c.exlife == Some(uid) {
            c.exlife = None;
        }
    }
    Some((s, under))
}

/// 何枚かのカードを1つのシールドとしてシールド化（先頭が上）
fn shield_stack(g: &mut Game, p: usize, cards: Vec<Card>, faceup: bool) {
    let Some(&top) = cards.first() else { return };
    g.ps[p].shields.push(Shield { card: top, faceup });
    for &c in cards[1..].iter() {
        g.ps[p].sh_under.push((top.uid, c));
    }
    push(g, Fx::ShieldPlaced { p, n: cards.len() as u8 });
}

/// ターン・プレイヤー p のエレメントが出た／呪文を唱えた（ウィリデ：相手のターン中に、はじめて）
fn first_event(g: &mut Game, p: usize, cost: u8) {
    if g.tp != p || g.ps[p].first_done {
        return;
    }
    g.ps[p].first_done = true;
    if has(g, 1 - p, C::Willide) {
        push(g, Fx::FirstEvent { p, cost });
    }
}

/// エモーショナル・ハードコアの「選んだ名前の能力を無視」を計算し直す
pub fn recompute_emo(g: &mut Game) {
    let mut names: Vec<(C, u16)> = vec![];
    for p in 0..2 {
        for x in g.ps[p].battle.iter() {
            if x.card.c == C::Emotional && !x.ignored {
                if let Some(n) = x.emo {
                    names.push((n, x.card.uid));
                }
            }
        }
    }
    for p in 0..2 {
        for x in g.ps[p].battle.iter_mut() {
            x.emo_ign = names.iter().any(|&(n, u)| n == x.card.c && u != x.card.uid);
        }
    }
}

/// セイント・キャッスル：相手のクリーチャーがシールドをブレイクする時、かわりにこのシールドをブレイクさせてもよい
pub fn saint_check(g: &mut Game, owner: usize) {
    if let Some(s) = g.ps[owner].shields.iter().find(|s| s.faceup && s.card.c == C::SaintCastle) {
        let u = s.card.uid;
        g.ask(owner, DK::Effect, vec![Opt::No, Opt::Yes], Q::Custom(EQ { k: K::SaintRepl, p: owner, a: u as i32, b: 0, list: vec![] }));
    }
}

// ---------- 状態を参照する能力 ----------

pub fn extra_kw(g: &Game, p: usize, c: &Cre) -> u32 {
    let mut k = 0;
    if c.card.c == C::Fujin && g.ps[1 - p].shields.is_empty() {
        k |= K_SA;
    }
    // 熊田すず：自分の生徒会組にスレイヤー（after_battle で処理）
    let _ = g;
    k
}

fn slayer(g: &Game, p: usize, c: &Cre) -> bool {
    race(c.card.c) & R_SEITOKAI != 0 && has(g, p, C::Suzu)
}

pub fn power_bonus(g: &Game, p: usize, i: usize) -> i32 {
    let c = &g.ps[p].battle[i];
    let mut b = 0;
    // ブランド G・G・G：手札1枚以下なら他のクリーチャーすべて+4000
    if g.ps[p].hand.len() <= 1 {
        b += 4000 * g.ps[p].battle.iter().filter(|x| x.card.c == C::Brand && !x.ign() && x.card.uid != c.card.uid).count() as i32;
    }
    b
}

pub fn powered_breaker(g: &Game, p: usize, c: &Cre) -> bool {
    g.ps[p].hand.len() <= 1 && g.ps[p].battle.iter().any(|x| x.card.c == C::Brand && !x.ign() && x.card.uid != c.card.uid)
}

pub fn can_attack_extra(g: &Game, p: usize, c: &Cre) -> bool {
    // アオフェシー：相手のクリーチャーは、出たターン、攻撃できない
    if c.entered == g.turn && has(g, 1 - p, C::Aofesy) && !has(g, p, C::RedZone) {
        return false;
    }
    // 流星アーシュ：ドラゴンではないクリーチャーは出たターンに攻撃できない
    if c.entered == g.turn && !is_dragon(c.card.c) && (has(g, 0, C::Ash) || has(g, 1, C::Ash)) {
        // レッドゾーン：自分のクリーチャーは相手のカードによる攻撃できない効果を無視
        let own_ash = has(g, p, C::Ash);
        if own_ash || !has(g, p, C::RedZone) {
            return false;
        }
    }
    true
}

pub fn can_attack_player(g: &Game, p: usize, c: &Cre) -> bool {
    // ガイアッシュ：相手のクリーチャーは出たターン、自分を攻撃できない
    if c.entered == g.turn && has(g, 1 - p, C::Gaiash) && !has(g, p, C::RedZone) {
        return false;
    }
    true
}

pub fn unblockable(g: &Game, att: u16) -> bool {
    // ジョギメラゴン：攻撃先は変更されない
    g.find(g.tp, att).map(|i| {
        let c = &g.ps[g.tp].battle[i];
        c.card.c == C::Jogimera && !c.ign()
    }).unwrap_or(false)
}

// ---------- メインステップの選択肢 ----------

/// 実行コスト（軽減後）と文明の条件
pub fn play_cost(g: &Game, p: usize, c: C, side: u8, how: u8) -> (usize, u8) {
    play_cost_b(g, p, c, side, how, None)
}

/// how の下位3ビット：0=通常 1=J・O・E 2=G・ゼロ 3=アビスラッシュ 5=ウィリデの[光(2)] 6=アカネの[無色(2)]。
/// HOW_GC のビット：グリーン・グランドクロスの軽減を使う。base：進化元（ゴルギーオージャーの軽減）
pub fn play_cost_b(g: &Game, p: usize, c: C, side: u8, how: u8, base: Option<u16>) -> (usize, u8) {
    let s = side_of(c, side);
    let mut cost = s.cost as i32;
    let mut civ = s.civ;
    let alt = how & 7;
    if side == 0 && alt == 5 && c == C::Willide {
        cost = 2;
        civ = L;
    }
    if side == 0 && alt == 6 && c == C::Akane {
        cost = 2;
        civ = ZERO;
    }
    if side == 0 && s.kind == Kind::Creature {
        // マナ武装3（エバーラスト）
        if c == C::Everlast && light_mana(g, p) >= 3 {
            cost -= 2;
        }
        // ゴルギーオージャー：進化元に含まれるカード1枚につき2少なく（2以下にはならない）
        if c == C::Gorgi {
            if let Some(n) = base.and_then(|b| base_cards(g, p, b)) {
                cost = (cost - 2 * n as i32).max(3);
            }
        }
        // 5000VT：バトルゾーンのクリーチャー1体につき1少なく
        if c == C::Vt5000 {
            cost -= (g.ps[0].battle.len() + g.ps[1].battle.len()) as i32;
        }
        // コハクの超魂X：NEOクリーチャーの召喚コスト-1
        if is_neo(c) {
            cost -= kohaku_count(g, p) as i32;
        }
        if how & HOW_GC != 0 {
            cost -= 1;
        }
        if cost >= 10 && has(g, p, C::Gaiash) {
            cost -= 4 * g.ps[p].battle.iter().filter(|x| x.card.c == C::Gaiash && !x.ign()).count() as i32;
        }
        if c == C::Jogimera && jokers_count(g, p) >= 4 {
            cost -= 3;
        }
        if how == 1 {
            cost -= match c {
                C::Brand => 4,
                C::Manhattan => 2,
                _ => 0,
            };
        }
    }
    // シネラリア：自分のジャイアントを使うコスト-1
    if side == 0 && is_giant(c) {
        cost -= g.ps[p].battle.iter().filter(|x| x.card.c == C::Cineraria && !x.ign()).count() as i32;
    }
    if race(c) & R_SEITOKAI != 0 {
        // シンパシー：自分のドラゴン1つにつき1少なく（アーシュ1体ごとに付与）
        let ash = g.ps[p].battle.iter().filter(|x| x.card.c == C::Ash && !x.ign()).count() as i32;
        let dragons = g.ps[p].battle.iter().filter(|x| is_dragon(x.card.c)).count() as i32;
        cost -= ash * dragons;
        // ギャイ：生徒会組はマナをすべての文明として使える
        if has(g, p, C::Gyai) {
            civ = 0;
        }
    }
    (cost.max(1) as usize, civ)
}

fn can_cast_spell(g: &Game, p: usize, cost: u8) -> bool {
    let op = 1 - p;
    if g.ps[p].spell_lock_until >= g.turn {
        return false;
    }
    if g.tp == p && has(g, op, C::Jogiragon) && g.ps[p].spells_this_turn >= 1 {
        return false;
    }
    // 一音の妖精：相手は各ターン1回しか呪文を唱えられない
    if g.ps[p].spells_this_turn >= 1 && has(g, op, C::Ichion) {
        return false;
    }
    // チャクラ（ハイパー）：相手はコスト5以下の呪文を唱えられない
    if cost <= 5 && g.ps[op].battle.iter().any(|x| x.card.c == C::Chakra && x.hyper && !x.ign()) {
        return false;
    }
    true
}

pub fn main_options(g: &Game, opts: &mut Vec<Opt>) {
    let p = g.tp;
    let mut seen: Vec<(C, u8, u8)> = vec![];
    // アビスラッシュ：ジャヴェールを墓地から召喚（そのターンはプレイヤーを攻撃でき、ターンの終わりに山札の下）
    if let Some(h) = g.ps[p].grave.iter().find(|c| c.c == C::Javert) {
        let ok = !creature_blocked(g, p) && !small_locked(g, p, C::Javert);
        let (cost, civ) = play_cost(g, p, C::Javert, 0, 0);
        if ok && g.pay_plan(p, civ, cost).is_some() {
            opts.push(Opt::Play { uid: h.uid, side: 0, how: 3 });
        }
    }
    for h in g.ps[p].hand.iter() {
        let c = h.c;
        let d = def(c);
        for side in 0..(if d.spell.is_some() { 2 } else { 1 }) {
            let s = side_of(c, side);
            let mut hows: Vec<u8> = match (c, side) {
                (C::Brand, 0) | (C::Manhattan, 0) => vec![0, 1],
                (C::Kesshin, 0) => vec![0, 2],
                _ => vec![0],
            };
            if side == 0 && c == C::Willide && g.ps[p].shields.len() > g.ps[1 - p].shields.len() {
                hows.push(5);
            }
            if side == 0 && c == C::Akane && [L, W, N].iter().all(|&cv| g.ps[p].mana.iter().any(|m| mcv(m) & cv != 0)) {
                hows.push(6);
            }
            // グリーン・グランドクロス：各ターンに1度、召喚コストを1少なくしてもよい
            if side == 0 && s.kind == Kind::Creature && gc_left(g, p) > 0 {
                let base: Vec<u8> = hows.iter().copied().filter(|&h| h != 2 && play_cost(g, p, c, 0, h).0 > 1).collect();
                for h in base {
                    hows.push(h | HOW_GC);
                }
            }
            for &how in hows.iter() {
                if seen.contains(&(c, side, how)) {
                    continue; // 同じカードは1つだけ候補に
                }
                if s.kind == Kind::Spell && !can_cast_spell(g, p, s.cost) {
                    continue;
                }
                if c == C::Melaventure && !g.ps[p].battle.iter().any(|x| is_jokers(x.card.c)) {
                    continue;
                }
                if s.kind == Kind::Creature && (creature_blocked(g, p) || small_locked(g, p, c)) {
                    continue;
                }
                let ok = if how == 2 {
                    jokers_count(g, p) >= 7
                } else if s.kind == Kind::Creature && d.evo != Evo::None {
                    !evo_choices(g, p, c, side, how, true).is_empty()
                } else {
                    let (cost, civ) = play_cost(g, p, c, side, how);
                    g.pay_plan(p, civ, cost).is_some()
                };
                if ok {
                    seen.push((c, side, how));
                    opts.push(Opt::Play { uid: h.uid, side, how });
                }
            }
        }
    }
}

pub fn play_from_hand(g: &mut Game, p: usize, uid: u16, side: u8, how: u8) {
    if how == 3 {
        let (cost, civ) = play_cost(g, p, C::Javert, 0, 0);
        let Some(plan) = g.pay_plan(p, civ, cost) else { return };
        let Some(gi) = g.ps[p].grave.iter().position(|c| c.uid == uid) else { return };
        g.pay(p, &plan);
        let card = g.ps[p].grave.remove(gi);
        mark_summoned(g, uid);
        if enter(g, p, card, How::Summon, None, 0) {
            if let Some(i) = g.find(p, uid) {
                g.ps[p].battle[i].player_ok = true;
                g.ps[p].battle[i].bottom = 1;
            }
        }
        return;
    }
    let Some(c) = hand_card(g, p, uid) else { return };
    let s = side_of(c, side);
    if s.kind == Kind::Creature && def(c).evo != Evo::None {
        // 進化：進化元を選んでから支払う（ゴルギーオージャーは進化元でコストが変わる）
        let o = evo_choices(g, p, c, side, how, true);
        if o.is_empty() {
            return;
        }
        if how & HOW_GC != 0 {
            g.ps[p].gc_used += 1;
        }
        let card = g.take_hand(p, uid).unwrap();
        g.ps[p].grave.push(card); // 一時置き場（answer で取り出す）
        let code = if how & 7 == 1 { 1 } else { 0 };
        g.ask(p, DK::Effect, o, Q::Custom(EQ { k: K::EvoBase, p, a: uid as i32, b: code, list: vec![1, side as u16, how as u16, 0] }));
        return;
    }
    if how & HOW_GC != 0 {
        g.ps[p].gc_used += 1;
    }
    if how != 2 {
        let (cost, civ) = play_cost(g, p, c, side, how);
        let Some(plan) = g.pay_plan(p, civ, cost) else { return };
        g.pay(p, &plan);
    } else {
        g.ps[p].no_free_this_turn = true;
    }
    match s.kind {
        Kind::Spell => {
            let card = g.take_hand(p, uid).unwrap();
            cast(g, p, card, side, how == 2);
        }
        Kind::GCastle => {
            let card = g.take_hand(p, uid).unwrap();
            put_gcastle(g, p, card);
        }
        Kind::Tama => {
            let card = g.take_hand(p, uid).unwrap();
            put_tama(g, p, card);
        }
        Kind::Creature => {
            let d = def(c);
            match d.evo {
                Evo::Evo | Evo::Neo | Evo::GNeo => {}
                Evo::None => {
                    let card = g.take_hand(p, uid).unwrap();
                    mark_summoned(g, uid);
                    if how == 1 {
                        enter(g, p, card, How::Summon, None, 2);
                    } else {
                        enter(g, p, card, How::Summon, None, 0);
                    }
                }
            }
        }
    }
}

fn evo_ok(c: C, base: C) -> bool {
    let civ = def(base).main.civ;
    match c {
        C::Melaventure => is_jokers(base),
        C::RedZone => civ & (D | F | N) != 0,
        C::Madan => civ & (W | D | F) != 0,
        C::Takasugi => civ & (L | F | N) != 0,
        C::Ichion | C::Canzone | C::Akane | C::Heracles | C::Gindan | C::Gorgi => civ & (L | W | N) != 0,
        _ => false,
    }
}

/// 進化元の候補（バトルゾーンのクリーチャーと、シンカライズのタマシード）
fn evo_bases(g: &Game, p: usize, c: C) -> Vec<u16> {
    let mut v: Vec<u16> = g.ps[p].battle.iter().filter(|x| evo_ok(c, x.card.c)).map(|x| x.card.uid).collect();
    if is_neo(c) || def(c).evo == Evo::Evo {
        v.extend(g.ps[p].tama.iter().filter(|t| t.c == C::Jasmine && evo_ok(c, t.c)).map(|t| t.uid));
    }
    v
}

/// 進化クリーチャーを出す時の選択肢（Pass=進化しないで出す）。pay なら支払えるものだけ
fn evo_choices(g: &Game, p: usize, c: C, side: u8, how: u8, pay: bool) -> Vec<Opt> {
    let ok = |b: Option<u16>| {
        if !pay {
            return true;
        }
        let (cost, civ) = play_cost_b(g, p, c, side, how, b);
        g.pay_plan(p, civ, cost).is_some()
    };
    let mut o = vec![];
    if def(c).evo != Evo::Evo && ok(None) {
        o.push(Opt::Pass);
    }
    for b in evo_bases(g, p, c) {
        if ok(Some(b)) {
            o.push(Opt::Pick(b));
        }
    }
    o
}

/// タマシードを出す（ジャスミンの地版）
fn put_tama(g: &mut Game, p: usize, card: Card) {
    g.ps[p].tama.push(card);
    first_event(g, p, cost_of(card.c));
    if card.c == C::Jasmine {
        g.mill_to_mana(p, 1, true);
    }
}

fn put_gcastle(g: &mut Game, p: usize, card: Card) {
    if fujin_blocks(g, p) {
        g.ps[p].grave.push(card);
        return;
    }
    g.ps[p].shields.push(Shield { card, faceup: true });
    push(g, Fx::ShieldPlaced { p, n: 1 });
    // ハッスル・キャッスル：このG城が表向きで置かれた時、山札の上から1枚をタップしてマナへ
    if card.c == C::Hustle {
        g.mill_to_mana(p, 1, true);
    }
}

// ---------- クリーチャーが出る ----------

/// 置換効果：出るかわりにどこへ行くか。None なら普通に出る。Some(true)=マナ Some(false)=墓地
fn enter_replacement(g: &Game, p: usize, c: C, how: How) -> Option<bool> {
    let op = 1 - p;
    let cost = cost_of(c) as usize;
    let mana = g.ps[p].mana.len();
    let mut grave = false;
    let mut mana_r = false;
    // VAN：相手がコマンドまたはドラゴンを出す時、かわりに墓地
    if has(g, op, C::Van) && race(c) & (R_DRAGON | R_COMMAND) != 0 {
        grave = true;
    }
    // マグナム：自分のターン中、相手のクリーチャーが出る時、かわりに墓地
    if g.tp == op && has(g, op, C::Magnum) {
        grave = true;
    }
    // ハッスル・キャッスル：召喚以外で出る時マナへ
    if how == How::Put && g.ps[op].shields.iter().any(|s| s.faceup && s.card.c == C::Hustle) {
        mana_r = true;
    }
    // PP-「P」：マナの枚数よりコストが大きい
    if cost > mana && has(g, op, C::PPP) {
        mana_r = true;
    }
    // キャディ：相手（キャディの持ち主から見て）のターン中
    if cost > mana && g.tp == p && has(g, op, C::Caddy) {
        mana_r = true;
    }
    // 置換は1つだけ。選ぶのは置換効果の持ち主（相手）。墓地のほうが相手に損なので墓地を選ぶ（近似）
    if grave {
        Some(false)
    } else if mana_r {
        Some(true)
    } else {
        None
    }
}

/// p のクリーチャー card をバトルゾーンへ。base=進化元の uid。joe: 2=J・O・E
pub fn enter(g: &mut Game, p: usize, card: Card, how: How, base: Option<u16>, joe: u8) -> bool {
    if creature_blocked(g, p) || small_locked(g, p, card.c) {
        // 近似：出せないカードは墓地へ
        g.ps[p].grave.push(card);
        return false;
    }
    if let Some(to_mana) = enter_replacement(g, p, card.c, how) {
        if to_mana {
            g.put_mana(p, card, false);
        } else {
            g.ps[p].grave.push(card);
        }
        if how != How::Summon {
            g.ps[p].no_free_this_turn = true;
        }
        return false;
    }
    if how != How::Summon {
        g.ps[p].no_free_this_turn = true;
    }
    let mut cre = g.new_cre(card);
    let mut taka_base = false;
    if let Some(b) = base {
        if let Some(i) = g.find(p, b) {
            let old = g.ps[p].battle.remove(i);
            // TAKASUGI：進化元になる時
            taka_base = old.card.c == C::Takasugi && !old.ign();
            cre.under.push(old.card);
            cre.under.extend(old.under);
            // 進化：元の状態を引き継ぐ。進化クリーチャーは召喚酔いしない（801.3）
            cre.tapped = old.tapped;
            cre.sick = false;
            cre.entered = old.entered;
        } else if let Some(ti) = g.ps[p].tama.iter().position(|t| t.uid == b) {
            // シンカライズのタマシードの上（41077：召喚酔いしない）
            let t = g.ps[p].tama.remove(ti);
            cre.under.push(t);
            cre.sick = false;
        }
    }
    if joe == 2 {
        cre.bottom = 2;
    }
    if def(card.c).main.kw & K_JUSTDIVER != 0 {
        cre.jd = true;
    }
    // ザゼ・ゼーン：タップして出る
    if card.c == C::Zaze {
        cre.tapped = true;
    }
    // EXライフ：出す時、山札の上から1枚をシールド化
    if def(card.c).main.kw & K_EXLIFE != 0 {
        cre.exlife = shieldify_top(g, p);
    }
    g.ps[p].battle.push(cre);
    g.ps[p].entered_this_turn += 1;
    let uid = card.uid;
    // 出た時の誘発（後に積んだものが先）。非ターン・プレイヤーのウィリデを先に積む
    first_event(g, p, cost_of(card.c));
    if taka_base {
        push(g, Fx::TakaLook { p });
    }
    if is_dragon(card.c) {
        push(g, Fx::DragonEntered { p });
    }
    push(g, Fx::Enter { p, uid });
    // コットン：相手のクリーチャーが出た時、そのターン攻撃できない
    let op = 1 - p;
    let cotton = g.ps[op].battle.iter().filter(|x| x.card.c == C::Cotton && !x.ign()).count();
    if cotton > 0 {
        if let Some(i) = g.find(p, uid) {
            g.ps[p].battle[i].cant_attack = true;
        }
    }
    true
}

/// 出た時の能力（Jビッグなどで他のクリーチャーが使う場合は src が使う側）
fn enter_ability(g: &mut Game, p: usize, src: u16, c: C) {
    let op = 1 - p;
    if let Some(pr) = crate::textfx::prog(c, crate::textfx::Ev::Etb) {
        crate::textfx::start(g, p, src, pr);
        return;
    }
    match c {
        C::Humpty => {
            if !g.ps[op].hand.is_empty() {
                let l: Vec<u16> = g.ps[op].hand.iter().map(|c| c.uid).collect();
                ask(g, K::HumptyDiscard, p, 0, 0, l);
            }
        }
        C::Chakra => {
            shieldify_top(g, p);
        }
        C::Dagura => {
            let l: Vec<u16> = g.ps[p].shields.iter().map(|s| s.card.uid).collect();
            if !l.is_empty() {
                ask(g, K::DaguraShield, p, 0, 0, l);
            }
        }
        C::RedZone => {
            if let Some(i) = g.find(p, src) {
                g.ps[p].battle[i].tapped = false;
            }
            kill_weakest(g, op);
        }
        C::Antiham => {
            g.draw1(p);
            ask(g, K::AntihamPut, p, 0, 0, vec![]);
        }
        C::Gaiash => {
            push(g, Fx::Ability { p, src, c: C::Gaiash }); // 下の Draw 用ダミー
        }
        C::DogiGyaku => {
            if !g.ps[p].final_rev_used {
                g.ps[p].final_rev_used = true;
                g.ps[p].no_lose = true;
                for x in g.ps[p].battle.iter_mut() {
                    x.no_leave = true;
                }
            }
        }
        C::Van => {} // 召喚して出した時：enter_summon で処理
        C::DarkMasters => {
            if !g.ps[op].hand.is_empty() {
                ask(g, K::DarkMasters, p, 3, 0, vec![]);
            }
        }
        C::Kernel => {
            let l = targetable(g, op);
            if !l.is_empty() {
                ask(g, K::KernelLock, p, 0, 0, l);
            }
        }
        C::Sozanmaru => ask(g, K::SozanDraw, p, 0, 0, vec![]),
        C::Laffrull => {
            // 次の相手のターンの終わりまで、相手は呪文を唱えられない
            // 「次の相手のターン」＝今のターンより後の最初の相手のターン（相手のターン中に出たら、今のターンではなくその次の相手のターン）
            let until = if g.tp == p { g.turn + 1 } else { g.turn + 2 };
            g.ps[op].spell_lock_until = g.ps[op].spell_lock_until.max(until);
        }
        C::Kusabira => {} // 召喚によって出た時のみ（enter_summon）
        C::Shacogail => {
            let mut gr = std::mem::take(&mut g.ps[p].grave);
            g.ps[p].deck.append(&mut gr);
            let mut d = std::mem::take(&mut g.ps[p].deck);
            g.rng.shuffle(&mut d);
            g.ps[p].deck = d;
        }
        C::Madan => {
            let evolved = g.find(p, src).map(|i| !g.ps[p].battle[i].under.is_empty()).unwrap_or(false);
            if evolved {
                // 相手は進化ではないクリーチャーをすべて山札の下へ（近似：順序はランダム）
                let l: Vec<u16> = g.ps[op].battle.iter().filter(|x| x.under.is_empty()).map(|x| x.card.uid).collect();
                for u in l {
                    leave(g, op, u, Dest::DeckBottom, true);
                }
            }
            let l: Vec<u16> = g.ps[op].battle.iter().filter(|x| x.under.is_empty()).map(|x| x.card.uid).collect();
            for u in l {
                g.stack.push(Task::Destroy { p: op, uid: u, by_opp: true });
            }
        }
        C::Fujisan => {
            let top = top_n(g, p, 3);
            let mut rest = vec![];
            for c in top {
                if is_jokers(c.c) {
                    g.ps[p].hand.push(c);
                } else {
                    rest.push(c);
                }
            }
            bottom_random(g, p, rest);
            g.check_state();
        }
        C::Slotton => {
            let top = top_n(g, p, 3);
            g.check_state();
            if !top.is_empty() && top.len() == 3 && top.iter().all(|c| is_jokers(c.c)) {
                let l = stash(g, p, &top);
                let cands: Vec<u16> = top.iter().filter(|c| is_creature_card(c.c)).map(|c| c.uid).collect();
                if !cands.is_empty() {
                    ask(g, K::SlottonPick, p, 0, 0, l);
                    return;
                }
                send_looked_to_bottom(g, p, &l);
            } else {
                bottom_random(g, p, top);
            }
        }
        C::Brand => ask(g, K::BrandDiscard, p, 0, 0, vec![]),
        C::Jogimera => {
            let top = top_n(g, p, 4);
            g.check_state();
            let l = stash(g, p, &top);
            ask(g, K::JogimeraPick, p, 0, 0, l);
        }
        C::Melaventure => ask(g, K::MelaPut, p, 2, 0, vec![]),
        C::Jogiragon => {
            g.draw1(p);
            if !g.ps[p].hand.is_empty() {
                ask(g, K::Discard, p, 1, 2, vec![src]);
            }
        }
        C::Moujakinga => {
            if !g.ps[op].shields.is_empty() {
                ask(g, K::MoujaShield, p, 0, 0, vec![]);
            } else {
                ask(g, K::MoujaCre, p, 0, 0, vec![0]);
            }
        }
        C::Niyani => ask(g, K::NiyaniMode, p, 0, 0, vec![src]),
        C::Sakki => ask(g, K::SakkiPick, p, 0, 0, vec![src]),
        C::Manhattan => {
            if g.ps[op].shields.len() > 2 {
                ask(g, K::ManhattanKeep, p, 2, 0, vec![src]);
            } else {
                ask(g, K::ManhattanDiscard, p, 0, 0, vec![]);
            }
        }
        C::Magny | C::Javert => {} // スマッシュ・バーストは攻撃時
        // ---- 2026-11-26 追加 ----
        C::Everlast => ask(g, K::EverShield, p, 0, 0, vec![]),
        C::Emotional => ask(g, K::EmoName, p, 0, 0, vec![src]),
        C::Gorgi => {
            let n = contained(g, p, src);
            g.mill_to_mana(p, n, false);
            ask(g, K::GorgiReturn, p, 3, 0, vec![]);
        }
        C::Canzone => {
            // 近似：超魂レイド → シールド化 の順に解決
            let l = targetable(g, op);
            if !l.is_empty() {
                ask(g, K::CanzoneShield, p, 0, 0, l);
            }
            let top = top_n(g, p, 3);
            g.check_state();
            if !top.is_empty() {
                let l = stash(g, p, &top);
                ask(g, K::CanzoneRaid, p, 0, src as i32, l);
            }
        }
        C::Akane => {
            let top = top_n(g, p, 2);
            g.check_state();
            if !top.is_empty() {
                let l = stash(g, p, &top);
                ask(g, K::AkanePick, p, 0, 0, l);
            }
        }
        C::Heracles => {
            let n = contained(g, p, src).max(1);
            g.draw1(p);
            g.draw1(p);
            untap_mana(g, p, n);
        }
        C::Takasugi => {
            let evolved = g.find(p, src).map(|i| !g.ps[p].battle[i].under.is_empty()).unwrap_or(false);
            if evolved {
                push(g, Fx::TakaLook { p });
            }
        }
        C::Kohaku => {
            if let Some(i) = g.find(p, src) {
                g.ps[p].battle[i].stay = true;
            }
        }
        C::Gindan => {
            let n = contained(g, p, src).max(1);
            for _ in 0..n {
                let l = targetable(g, op);
                if !l.is_empty() {
                    ask(g, K::ChakraTap, p, 0, 0, l);
                }
            }
        }
        C::Aofesy => {
            if let Some(k) = g.revch.iter().position(|&u| u == src) {
                g.revch.remove(k);
                g.stack.push(Task::Draw { p, n: 2 });
            }
        }
        C::Cineraria => {
            if g.ps[p].battle.iter().any(|x| x.card.uid != src && is_giant(x.card.c)) {
                g.draw1(p);
            }
        }
        C::Vt5000 => {
            let l: Vec<u16> = (0..g.ps[op].battle.len()).filter(|&i| g.power(op, i) <= 5000).map(|i| g.ps[op].battle[i].card.uid).collect();
            for u in l {
                leave(g, op, u, Dest::Hand, true);
            }
            g.ps[op].small_lock_turn = if g.tp == p { g.turn + 1 } else { g.turn + 2 };
        }
        _ => {}
    }
}

/// 召喚によって出た時だけの能力
fn summon_ability(g: &mut Game, p: usize, uid: u16, c: C) {
    let op = 1 - p;
    match c {
        C::Van => {
            let l = uids(g, op);
            for u in l {
                leave(g, op, u, Dest::Hand, true);
            }
        }
        C::Kusabira => kusabira(g, p, uid),
        _ => {}
    }
}

fn kusabira(g: &mut Game, p: usize, _uid: u16) {
    ask(g, K::Kusabira, p, 0, 0, vec![]);
}

fn kill_weakest(g: &mut Game, op: usize) {
    if g.ps[op].battle.is_empty() {
        return;
    }
    let pw: Vec<i32> = (0..g.ps[op].battle.len()).map(|i| g.power(op, i)).collect();
    let m = *pw.iter().min().unwrap();
    let l: Vec<u16> = (0..pw.len()).filter(|&i| pw[i] == m).map(|i| g.ps[op].battle[i].card.uid).collect();
    for u in l {
        g.stack.push(Task::Destroy { p: op, uid: u, by_opp: true });
    }
}

// ---------- クリーチャーが離れる ----------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dest {
    Grave,
    Hand,
    Mana,
    DeckBottom,
    Shield, // シールド化（裏向き）
}

/// by_opp: 相手のカードの効果によるか
pub fn leave(g: &mut Game, p: usize, uid: u16, dest: Dest, by_opp: bool) -> bool {
    leave2(g, p, uid, dest, by_opp, true)
}

fn leave2(g: &mut Game, p: usize, uid: u16, dest: Dest, by_opp: bool, repl: bool) -> bool {
    let Some(i) = g.find(p, uid) else { return false };
    if by_opp && g.ps[p].battle[i].no_leave {
        return false;
    }
    // ザゼ・ゼーン／コハク：離れない
    if g.ps[p].battle[i].stay {
        return false;
    }
    if dest == Dest::Shield && fujin_blocks(g, p) {
        return false;
    }
    let c = g.ps[p].battle[i].clone();
    // 置換効果（持ち主が選ぶ。近似：EXライフ > G-NEO > エターナル・Ω > キャディ の順に使う）
    if let Some(su) = c.exlife {
        if let Some((s, under)) = take_shield(g, p, su) {
            g.ps[p].grave.push(s.card);
            g.ps[p].grave.extend(under);
            push(g, Fx::Zrush { p });
            return false;
        }
    }
    if def(c.card.c).evo == Evo::GNeo && !c.under.is_empty() && !c.ign() {
        let under = std::mem::take(&mut g.ps[p].battle[i].under);
        if dest == Dest::Shield {
            shield_stack(g, p, under, false);
        } else {
            for u in under {
                move_card(g, p, u, dest);
            }
        }
        return false;
    }
    // 自分の効果で動かす時以外（相手の効果・バトル・破壊）の置換
    if repl && (by_opp || dest == Dest::Grave) {
        // エバーラストのマナ武装5：かわりにシールドを2つ手札に加える（近似：シールドが3つ以上で、コスト5以上のクリーチャーの時に自動で使う）
        if has(g, p, C::Everlast) && light_mana(g, p) >= 5 && g.ps[p].shields.len() >= 3 && cost_of(c.card.c) >= 5 {
            for _ in 0..2 {
                let su = g.ps[p].shields.last().unwrap().card.uid;
                if let Some((s, under)) = take_shield(g, p, su) {
                    g.ps[p].hand.push(s.card);
                    g.ps[p].hand.extend(under);
                }
            }
            push(g, Fx::Zrush { p });
            return false;
        }
        // シネラリアのウルトラ・セイバー：自分のジャイアントが離れる時、かわりにシネラリアを破壊（近似：コスト4以上のジャイアントの時に自動で使う）
        if is_giant(c.card.c) && cost_of(c.card.c) >= 4 {
            if let Some(su) = g.ps[p].battle.iter().find(|x| x.card.c == C::Cineraria && !x.ign() && !x.stay && x.card.uid != uid).map(|x| x.card.uid) {
                leave2(g, p, su, Dest::Grave, false, false);
                return false;
            }
        }
    }
    let mut dest = dest;
    if c.card.c == C::Van && !c.ign() {
        dest = Dest::Hand;
    } else if dest == Dest::Grave && c.card.c == C::Caddy && !c.ign() {
        dest = Dest::Mana;
    }
    let c = g.ps[p].battle.remove(i);
    if dest == Dest::Shield {
        // 進化エレメントは1つのシールドにまとめる（近似：316.3・113.2）
        let mut all = vec![c.card];
        all.extend(c.under.iter().copied());
        shield_stack(g, p, all, false);
    } else {
        move_card(g, p, c.card, dest);
        for &u in c.under.iter() {
            move_card(g, p, u, dest);
        }
    }
    // カンツォーネが破壊された時：下にあったカードを、自分のNEO・進化クリーチャーの下か、表向きの1つのシールドへ
    if dest == Dest::Grave && c.card.c == C::Canzone && !c.ign() && !c.under.is_empty() {
        let l: Vec<u16> = c.under.iter().map(|u| u.uid).collect();
        ask(g, K::CanzoneDeath, p, 0, 0, l);
    }
    // 離れた時の誘発
    if race(c.card.c) & R_ABYSS != 0 {
        push(g, Fx::LeftAbyss { p });
    }
    if is_jokers(c.card.c) {
        push(g, Fx::LeftJokers { p, c: c.card.c });
    }
    true
}

fn move_card(g: &mut Game, p: usize, c: Card, dest: Dest) {
    match dest {
        Dest::Grave => g.ps[p].grave.push(c),
        Dest::Hand => g.ps[p].hand.push(c),
        Dest::Mana => g.put_mana(p, c, false),
        Dest::DeckBottom => g.ps[p].deck.insert(0, c),
        Dest::Shield => shield_stack(g, p, vec![c], false),
    }
}

pub fn destroy(g: &mut Game, p: usize, uid: u16, by_opp: bool) {
    leave(g, p, uid, Dest::Grave, by_opp);
}

pub fn after_battle(g: &mut Game, a: (usize, u16), b: (usize, u16), pa: i32, pb: i32) {
    // スレイヤー：バトルの後、相手を破壊
    let ia = g.find(a.0, a.1).unwrap();
    let ib = g.find(b.0, b.1).unwrap();
    let sa = slayer(g, a.0, &g.ps[a.0].battle[ia]);
    let sb = slayer(g, b.0, &g.ps[b.0].battle[ib]);
    if sa && pa < pb {
        g.stack.push(Task::Destroy { p: b.0, uid: b.1, by_opp: true });
    }
    if sb && pb < pa {
        g.stack.push(Task::Destroy { p: a.0, uid: a.1, by_opp: true });
    }
}

// ---------- シールド ----------

pub fn shields_to_hand(g: &mut Game, owner: usize, list: Vec<u16>, broken: bool) {
    let mut to_hand = vec![];
    let mut left = false;
    for u in list {
        let Some((s, under)) = take_shield(g, owner, u) else { continue };
        left = true;
        let saint = g.saint == Some(u);
        if saint {
            g.saint = None;
        }
        if s.faceup && def(s.card.c).main.kind == Kind::GCastle && !saint {
            g.ps[owner].grave.push(s.card); // G城：かわりに墓地。ブレイクしたことにならない
            g.ps[owner].grave.extend(under);
            continue;
        }
        if broken {
            g.attack_broke = true;
        }
        g.ps[owner].hand.push(s.card);
        to_hand.push(s.card.uid);
        for c in under {
            g.ps[owner].hand.push(c);
            to_hand.push(c.uid);
        }
    }
    if left {
        push(g, Fx::Zrush { p: owner });
    }
    if broken && !to_hand.is_empty() {
        push(g, Fx::TriggerDecl { owner, uids: to_hand });
    }
}

fn trigger_kind(g: &Game, owner: usize, c: C) -> Option<u8> {
    // ウィリデ：相手のクリーチャーが3体以上あれば、シールドゾーンにあるこれに S・トリガー（113.6b）
    if c == C::Willide && g.ps[1 - owner].battle.len() >= 3 {
        return Some(0);
    }
    let d = def(c);
    if d.main.kw & K_STRIG != 0 || d.spell.map(|s| s.kw & K_STRIG != 0).unwrap_or(false) {
        return Some(0);
    }
    if d.main.kw & K_GSTRIKE != 0 {
        return Some(1);
    }
    None
}

pub fn use_trigger(g: &mut Game, owner: usize, uid: u16, what: u8) {
    let Some(c) = hand_card(g, owner, uid) else { return };
    match what {
        0 => {
            // S・トリガー：コストを支払わずに召喚／唱える
            let d = def(c);
            let spell_side = if d.main.kw & K_STRIG == 0 && d.spell.map(|s| s.kw & K_STRIG != 0).unwrap_or(false) { 1 } else { 0 };
            let s = side_of(c, spell_side);
            if s.kind == Kind::Spell {
                let card = g.take_hand(owner, uid).unwrap();
                if can_cast_spell(g, owner, s.cost) {
                    cast(g, owner, card, spell_side, true);
                } else {
                    g.ps[owner].hand.push(card);
                }
            } else if creature_blocked(g, owner) || small_locked(g, owner, c) {
                // 出せない：手札に残る（46851）
            } else if d.evo != Evo::None {
                let o = evo_choices(g, owner, c, 0, 0, false);
                if o.is_empty() {
                    return;
                }
                let card = g.take_hand(owner, uid).unwrap();
                g.ps[owner].grave.push(card);
                g.ask(owner, DK::Effect, o, Q::Custom(EQ { k: K::EvoBase, p: owner, a: uid as i32, b: 2, list: vec![0, 0, 0, 0] }));
            } else {
                let card = g.take_hand(owner, uid).unwrap();
                mark_summoned(g, card.uid);
                enter(g, owner, card, How::FreeSummon, None, 0);
            }
        }
        1 => {
            // G・ストライク：相手のクリーチャーを1体選び、このターン攻撃できない
            let l = targetable(g, 1 - owner);
            if !l.is_empty() {
                ask(g, K::GStrike, owner, 0, 0, l);
            }
        }
        _ => {}
    }
}

// ---------- 呪文 ----------

fn cast(g: &mut Game, p: usize, card: Card, side: u8, free: bool) {
    g.ps[p].spells_this_turn += 1;
    first_event(g, p, side_of(card.c, side).cost);
    if free {
        g.ps[p].no_free_this_turn = true;
    }
    // 効果の後で墓地（水晶チャージャーはマナ）
    let to_mana = card.c == C::Cotton && side == 1;
    g.stack.push(Task::Fx(Fx::Cast { p, uid: card.uid, side, free }));
    // 呪文カードは解決中は墓地の手前に置く：ここでは先に墓地/マナへ置いておく
    if to_mana {
        g.ps[p].mana.push(Mana { card, tapped: true, facedown: true });
    } else {
        g.ps[p].grave.push(card);
    }
}

fn spell_effect(g: &mut Game, p: usize, c: C, side: u8) {
    let op = 1 - p;
    if let Some(pr) = crate::textfx::prog(c, crate::textfx::Ev::Spell(side)) {
        crate::textfx::start(g, p, 0, pr);
        return;
    }
    match (c, side) {
        (C::Kiyuri, _) => {
            let top = top_n(g, p, 5);
            g.check_state();
            let l = stash(g, p, &top);
            ask(g, K::Kiyuri, p, 0, 2, l);
        }
        (C::Cotton, 1) => {
            let n = g.ps[op].mana.len() as u8;
            let l: Vec<u16> = g.ps[op].battle.iter().filter(|x| !x.jd && cost_of(x.card.c) > n).map(|x| x.card.uid).collect();
            if !l.is_empty() {
                ask(g, K::CottonSpell, p, 0, 0, l);
            }
        }
        (C::Gyakuten, _) => {
            let l = targetable(g, op);
            if !l.is_empty() {
                ask(g, K::Gyakuten, p, 0, 0, l);
            }
        }
        (C::Javert, 1) => {
            g.mill_to_mana(p, 1, true);
            let all_abyss = g.ps[p].mana.iter().all(|m| !m.facedown && race(m.card.c) & R_ABYSS != 0);
            if all_abyss {
                let l = targetable(g, op);
                if !l.is_empty() {
                    ask(g, K::Yabe, p, 0, 0, l);
                }
            }
        }
        (C::Kernel, 1) => {
            random_discard(g, op);
            if !g.ps[p].shields.is_empty() {
                let l: Vec<u16> = g.ps[p].shields.iter().map(|s| s.card.uid).collect();
                ask(g, K::ReturnShield, p, 0, 0, l);
            }
        }
        (C::Laffrull, 1) => {
            g.stack.push(Task::Fx(Fx::Ask(EQ { k: K::MiracleSpell, p, a: 0, b: 0, list: vec![] })));
            g.stack.push(Task::Draw { p, n: 3 });
        }
        (C::JoJoJo, _) => {
            let top = top_n(g, p, 4);
            g.check_state();
            let l = stash(g, p, &top);
            ask(g, K::JoJoJo, p, 0, 0, l);
        }
        (C::Magny, 1) => {
            let l: Vec<u16> = g.ps[op].battle.iter().filter(|x| !x.jd && cost_of(x.card.c) <= 3).map(|x| x.card.uid).collect();
            if !l.is_empty() {
                ask(g, K::Jolly, p, 0, 0, l);
            }
        }
        (C::Slotton, 1) => {
            let top = top_n(g, op, 3);
            g.check_state();
            if top.is_empty() {
                return;
            }
            let l = stash(g, op, &top);
            ask(g, K::Slot7Pick, p, 0, 0, l);
        }
        (C::Kesshin, _) => ask(g, K::Kesshin, p, 2, 0, vec![]),
        _ => {}
    }
}

// ---------- 攻撃 ----------

pub fn on_attack(g: &mut Game, att: u16, tgt: Tgt) {
    let p = g.tp;
    let op = 1 - p;
    let Some(i) = g.find(p, att) else { return };
    let c = g.ps[p].battle[i].clone();
    // 非ターン・プレイヤーの誘発（後で解決）を先に積む
    if tgt == Tgt::Player {
        // 逆転撃
        for h in g.ps[op].hand.clone() {
            if h.c == C::Gyakuten {
                g.stack.push(Task::Fx(Fx::Ask(EQ { k: K::Gyakuten, p: op, a: -1, b: h.uid as i32, list: vec![] })));
                break;
            }
        }
    }
    // 蒼斬丸：相手のクリーチャーが攻撃する時、手札を捨てて攻撃中止
    if has(g, op, C::Sozanmaru) && g.ps[op].once & 1 == 0 && !g.ps[op].hand.is_empty() {
        ask(g, K::SozanCancel, op, 0, 0, vec![]);
    }
    // ウラ・ニンジャ・ストライク6
    ninja_check(g, op);
    // ターン・プレイヤーの誘発
    if !c.ign() {
        if let Some(pr) = crate::textfx::prog(c.card.c, crate::textfx::Ev::Attack) {
            crate::textfx::start(g, p, att, pr);
        } else { match c.card.c {
            C::RedZone => kill_weakest(g, op),
            C::Magny | C::Javert => ask(g, K::SmashBurst, p, 0, 0, vec![att]),
            C::Melaventure => {
                if jokers_count(g, p) >= 7 {
                    let l: Vec<u16> = g.ps[op].battle.iter().filter(|x| g.kw(op, x) & K_BLOCKER != 0).map(|x| x.card.uid).collect();
                    for u in l {
                        g.stack.push(Task::Destroy { p: op, uid: u, by_opp: true });
                    }
                }
            }
            C::Sakki => ask(g, K::SakkiPick, p, 0, 0, vec![att]),
            C::Everlast => ask(g, K::EverShield, p, 0, 0, vec![]),
            C::Gorgi => {
                // 含まれるカードが10枚以上なら勝つ
                if 1 + c.under.len() >= 10 && !g.ps[op].no_lose {
                    g.winner = Some(p);
                    return;
                }
            }
            _ => {}
        } }
    }
    // 革命チェンジ・D・D・D（手札から）
    let mut rev = vec![];
    for h in g.ps[p].hand.iter() {
        let ok = match h.c {
            C::DogiGyaku => def(c.card.c).main.civ & (L | N) != 0 && g.power(p, i) >= 8000,
            C::Laffrull => def(c.card.c).main.civ & (L | W) != 0 && is_dragon(c.card.c),
            C::Jogiragon => is_jokers(c.card.c) && cost_of(c.card.c) >= 5,
            C::Aofesy => cost_of(c.card.c) >= 3 && race(c.card.c) & (R_GIANT | R_MAGIC) != 0,
            _ => false,
        };
        if ok && !rev.iter().any(|&u| hand_card(g, p, u) == Some(h.c)) {
            rev.push(h.uid);
        }
    }
    if !rev.is_empty() {
        ask(g, K::RevChange, p, 0, att as i32, rev);
    }
    if !creature_blocked(g, p) {
        if let Some(h) = g.ps[p].hand.iter().find(|h| h.c == C::RedZone) {
            if g.pay_plan(p, F, 4).is_some() {
                ask(g, K::DDD, p, h.uid as i32, att as i32, vec![]);
            }
        }
        // TAKASUGI の D・D・D [自然(2)]（シネラリアで軽減）
        if let Some(h) = g.ps[p].hand.iter().find(|h| h.c == C::Takasugi) {
            if !small_locked(g, p, C::Takasugi) && g.pay_plan(p, N, ddd_taka_cost(g, p)).is_some() {
                ask(g, K::DDD, p, h.uid as i32, att as i32, vec![1]);
            }
        }
    }
}

fn ninja_check(g: &mut Game, op: usize) {
    if g.attack_ninja_used {
        return;
    }
    let mana_ok = g.ps[op].mana.len() >= 6
        && g.ps[op].mana.iter().any(|m| mcv(m) & L != 0)
        && g.ps[op].mana.iter().any(|m| mcv(m) & W != 0);
    if !mana_ok {
        return;
    }
    if let Some(h) = g.ps[op].hand.iter().find(|h| h.c == C::Sozanmaru) {
        ask(g, K::Ninja, op, h.uid as i32, 0, vec![]);
    }
}

pub fn on_block(g: &mut Game, op: usize, _b: u16) {
    // ウラ・ニンジャ・ストライク：相手のクリーチャーがブロックした時
    let p = 1 - op;
    let _ = p;
    // 自分（ターン・プレイヤー）側の蒼斬丸は相手ブロック時に使える
    ninja_check(g, g.tp);
}

pub fn attack_end(g: &mut Game, att: u16) {
    let p = g.tp;
    // キリフダッシュ（モウジャキンガ）
    if g.attack_broke {
        if let Some(h) = g.ps[p].hand.iter().find(|h| h.c == C::Moujakinga) {
            if g.pay_plan(p, W | D | F, 5).is_some() {
                ask(g, K::Kirifuda, p, h.uid as i32, 0, vec![]);
            }
        }
    }
    let Some(i) = g.find(p, att) else { return };
    let c = g.ps[p].battle[i].clone();
    if c.ign() {
        return;
    }
    if c.card.c == C::Dagura && c.hyper {
        ask(g, K::DaguraMana, p, 0, 0, vec![]);
    }
    if c.card.c == C::Moujakinga {
        push(g, Fx::MoujaEnd { p, uid: att });
    }
}

// ---------- ターンのはじめ・終わり ----------

pub fn untapped(g: &mut Game, p: usize, list: Vec<u16>) {
    for u in list {
        if let Some(i) = g.find(p, u) {
            if g.ps[p].battle[i].card.c == C::Kusabira && !g.ps[p].battle[i].ign() {
                kusabira(g, p, u);
            }
        }
    }
}

pub fn turn_start(g: &mut Game) {
    let p = g.tp;
    let op = 1 - p;
    // 逆転撃の支払い
    if g.ps[p].debt > 0 {
        let n = g.ps[p].debt as usize;
        g.ps[p].debt = 0;
        match g.pay_plan(p, D, n) {
            Some(plan) => g.pay(p, &plan),
            None => {
                if !g.ps[p].no_lose {
                    g.winner = Some(op);
                }
                return;
            }
        }
    }
    // シャコガイル（相手のターンのはじめ）
    let n = g.ps[op].battle.iter().filter(|x| x.card.c == C::Shacogail && !x.ign()).count();
    for _ in 0..n {
        ask(g, K::Discard, op, 3, 0, vec![]);
        g.stack.push(Task::Draw { p: op, n: 5 });
    }
}

pub fn turn_end(g: &mut Game) {
    let p = g.tp;
    let op = 1 - p;
    // 非ターン・プレイヤー側（後で解決）
    // PP-「P」と超魂X：相手のターンの終わりに、相手のタップしているクリーチャーがなければ
    if !g.ps[p].battle.iter().any(|x| x.tapped) {
        let n = g.ps[op].battle.iter().filter(|x| !x.ign() && (x.card.c == C::PPP || x.under.iter().any(|u| u.c == C::PPP))).count();
        for _ in 0..n {
            ask(g, K::PPPEnd, op, 0, 0, vec![]);
        }
    }
    // ガイアッシュ
    if g.ps[p].no_free_this_turn && !has(g, op, C::Gaiash) {
        if let Some(h) = g.ps[op].hand.iter().find(|h| h.c == C::Gaiash) {
            ask(g, K::GaiashSummon, op, h.uid as i32, 0, vec![]);
        }
    }
    // ターン・プレイヤー側
    // 富士山ッピング（手札）
    if let Some(h) = g.ps[p].hand.iter().find(|h| h.c == C::Fujisan) {
        let total: u32 = g.ps[p].battle.iter().filter(|x| is_jokers(x.card.c)).map(|x| cost_of(x.card.c) as u32).sum();
        if total >= 7 {
            ask(g, K::FujisanReturn, p, 0, h.uid as i32, vec![]);
        }
    }
    // ドギラゴン逆
    let dogi = g.ps[p].battle.iter().filter(|x| x.card.c == C::DogiGyaku && !x.ign()).count();
    for _ in 0..dogi {
        ask(g, K::DogiEnd, p, 0, 0, vec![]);
    }
    // グリーン・グランドクロス：自分のクリーチャーが相手より多ければ1枚引いてもよい（近似：山札が5枚より多ければ引く）
    if g.ps[p].battle.len() > g.ps[op].battle.len() {
        let n = g.ps[p].shields.iter().filter(|s| s.faceup && s.card.c == C::GrandCross).count();
        for _ in 0..n {
            if g.ps[p].deck.len() > 5 {
                g.stack.push(Task::Draw { p, n: 1 });
            }
        }
    }
    // ターンの終わりに山札の下へ
    let list: Vec<(u16, u8)> = g.ps[p].battle.iter().filter(|x| x.bottom > 0).map(|x| (x.card.uid, x.bottom)).collect();
    let mut draws = 0u8;
    for (u, b) in list {
        let put = leave(g, p, u, Dest::DeckBottom, false);
        if b == 2 || (b == 3 && put) {
            draws += 1;
        }
    }
    if draws > 0 {
        g.stack.push(Task::Draw { p, n: draws });
    }
}

// ---------- 遅延効果の実行 ----------

pub fn run_fx(g: &mut Game, fx: Fx) {
    match fx {
        Fx::Enter { p, uid } => {
            let Some(i) = g.find(p, uid) else { return };
            let c = g.ps[p].battle[i].card.c;
            if g.ps[p].battle[i].ign() {
                return;
            }
            if let Some(k) = g.summoned.iter().position(|&u| u == uid) {
                g.summoned.remove(k);
                summon_ability(g, p, uid, c);
            }
            enter_ability(g, p, uid, c);
        }
        Fx::Ability { p, src, c } => {
            if c == C::Gaiash {
                g.stack.push(Task::Draw { p, n: 2 });
                return;
            }
            enter_ability(g, p, src, c);
        }
        Fx::Cast { p, uid, side, free: _ } => {
            let c = g.ps[p].grave.iter().find(|c| c.uid == uid).map(|c| c.c).or_else(|| g.ps[p].mana.iter().find(|m| m.card.uid == uid).map(|m| m.card.c));
            if let Some(c) = c {
                spell_effect(g, p, c, side);
            }
        }
        Fx::Vm(vm) => crate::textfx::run(g, *vm),
        Fx::Ask(eq) => {
            let opts = options_for(g, &eq);
            if !opts.is_empty() {
                let p = eq.p;
                g.ask(p, DK::Effect, opts, Q::Custom(eq));
            }
        }
        Fx::Zrush { p } => {
            for c in g.ps[p].battle.iter_mut() {
                if (c.card.c == C::Chakra || c.card.c == C::Dagura) && !c.ign() {
                    c.hyper = true;
                }
            }
        }
        Fx::ShieldPlaced { p, n } => {
            // ザゼ・ゼーン：自分のシールドゾーンにカードが置かれた時（1枚ごと。45467）
            let zz: Vec<u16> = g.ps[p].battle.iter().filter(|x| x.card.c == C::Zaze && !x.ign()).map(|x| x.card.uid).collect();
            for _ in 0..n {
                for &z in zz.iter() {
                    ask(g, K::ZazeMode, p, 0, 0, vec![z]);
                }
            }
            let k = g.ps[p].battle.iter().filter(|x| x.card.c == C::Chakra && !x.ign()).count() * n as usize;
            for _ in 0..k {
                let l = targetable(g, 1 - p);
                if !l.is_empty() {
                    ask(g, K::ChakraTap, p, 0, 0, l);
                }
            }
        }
        Fx::LeftAbyss { p } => {
            if has(g, p, C::Javert) && g.ps[p].once & 2 == 0 {
                g.ps[p].once |= 2;
                let op = 1 - p;
                if !g.ps[op].hand.is_empty() {
                    ask(g, K::Discard, op, 1, 0, vec![]);
                }
            }
        }
        Fx::LeftJokers { p, c } => {
            // 富士山ッピング：自分の他のジョーカーズが離れた時、それの出た時能力を1つ使ってもよい（近似：必ず使う）
            if c != C::Fujisan && has(g, p, C::Fujisan) && g.ps[p].once & 4 == 0 && has_enter_ability(c) {
                g.ps[p].once |= 4;
                let src = g.ps[p].battle.iter().find(|x| x.card.c == C::Fujisan).map(|x| x.card.uid).unwrap();
                enter_ability(g, p, src, c);
            }
        }
        Fx::DragonEntered { p } => {
            // 「各ターンに１度、自分のドラゴンが出た時」：カード1枚ごとに1ターン1回（QA 44212）。
            // 出たドラゴン自身にも誘発する（「自分のドラゴンが出た時」は自身を含む：QA 41443）。
            // 「〜てもよい」を使わなかった時は、そのターンの次のドラゴンでまた誘発する。
            let op = 1 - p;
            let turn = g.turn;
            let srcs: Vec<(u16, C)> = g.ps[p].battle.iter().filter(|x| !x.ign() && x.t1 != turn && matches!(x.card.c, C::Gyai | C::Ash | C::Suzu)).map(|x| (x.card.uid, x.card.c)).collect();
            for (u, c) in srcs.into_iter().rev() {
                match c {
                    C::Gyai => ask(g, K::GyaiMill, p, u as i32, 0, vec![]),
                    C::Ash => ask(g, K::AshDraw, p, u as i32, 0, vec![]),
                    _ => {
                        // 熊田すず：強制（相手の手札を見ないで1枚捨てさせる）
                        if let Some(i) = g.find(p, u) {
                            g.ps[p].battle[i].t1 = turn;
                        }
                        random_discard(g, op);
                    }
                }
            }
        }
        Fx::TriggerDecl { owner, uids } => {
            for u in uids.into_iter().rev() {
                if let Some(c) = hand_card(g, owner, u) {
                    if let Some(w) = trigger_kind(g, owner, c) {
                        g.stack.push(Task::Ask(Decision {
                            player: owner,
                            kind: DK::Trigger,
                            opts: vec![Opt::No, Opt::Yes],
                            q: Q::UseTrigger { owner, uid: u, what: w },
                        }));
                    }
                }
            }
        }
        Fx::TakaLook { p } => {
            let top = top_n(g, p, 4);
            g.check_state();
            if !top.is_empty() {
                let l = stash(g, p, &top);
                ask(g, K::TakaLook, p, 0, 0, l);
            }
        }
        Fx::FirstEvent { p, cost } => {
            let op = 1 - p;
            let n = g.ps[op].battle.iter().filter(|x| x.card.c == C::Willide && !x.ign()).count();
            for _ in 0..n {
                ask(g, K::WillideDraw, op, 0, cost as i32, vec![]);
            }
        }
        Fx::MoujaEnd { p, uid } => {
            g.draw1(p);
            if !g.ps[p].hand.is_empty() {
                ask(g, K::Discard, p, 1, 1, vec![uid]);
            }
        }
    }
}

fn has_enter_ability(c: C) -> bool {
    matches!(c, C::Fujisan | C::Slotton | C::Brand | C::Jogimera | C::Melaventure | C::Jogiragon | C::Moujakinga | C::Niyani | C::Sakki | C::Manhattan)
}

/// 召喚したことを記録（VAN・クサビラガミの「召喚して出した時」）
pub fn mark_summoned(g: &mut Game, uid: u16) {
    g.summoned.push(uid);
}

// ---------- 質問の選択肢と答え ----------

fn options_for(g: &Game, q: &EQ) -> Vec<Opt> {
    let p = q.p;
    let op = 1 - p;
    let yn = vec![Opt::No, Opt::Yes];
    match q.k {
        K::VmPick | K::VmYes | K::VmMode => crate::textfx::options(q),
        K::Kiyuri => {
            let l: Vec<u16> = q.list.iter().copied().filter(|&u| deck_card(g, p, u).map(|c| is_creature_card(c) && cost_of(c) <= 3).unwrap_or(false)).collect();
            if q.b <= 0 || l.is_empty() {
                vec![Opt::Pass]
            } else {
                with_pass(picks(&l))
            }
        }
        K::HumptyDiscard => picks(&g.ps[op].hand.iter().map(|c| c.uid).collect::<Vec<_>>()),
        K::HumptyKill | K::GStrike | K::ChakraTap | K::CottonSpell | K::Gyakuten | K::Yabe | K::KernelLock | K::Jolly | K::ReturnShield | K::DaguraShield => {
            let l: Vec<u16> = q.list.iter().copied().filter(|&u| {
                g.find(op, u).is_some() || g.ps[p].shields.iter().any(|s| s.card.uid == u)
            }).collect();
            if q.k == K::Gyakuten && q.a == -1 {
                return yn;
            }
            picks(&l)
        }
        K::DaguraMana => {
            let n = g.ps[p].mana.len();
            let l: Vec<u16> = g.ps[p].mana.iter().filter(|m| is_creature_card(m.card.c) && def(m.card.c).evo == Evo::None && (cost_of(m.card.c) as usize) < n).map(|m| m.card.uid).collect();
            with_pass(picks(&l))
        }
        K::AntihamPut => {
            let l: Vec<u16> = g.ps[p].hand.iter().filter(|c| is_creature_card(c.c) && def(c.c).evo == Evo::None && cost_of(c.c) <= 3).map(|c| c.uid).collect();
            with_pass(picks(&l))
        }
        K::Discard => picks(&g.ps[p].hand.iter().map(|c| c.uid).collect::<Vec<_>>()),
        K::GyaiMill | K::AshDraw | K::PPPEnd | K::GaiashSummon | K::Ninja | K::Kirifuda | K::SmashBurst | K::DogiEnd | K::DDD => yn,
        K::GyaiReturn => {
            let l: Vec<u16> = g.ps[p].mana.iter().filter(|m| is_dragon(m.card.c)).map(|m| m.card.uid).collect();
            with_pass(picks(&l))
        }
        K::MiracleSpell => {
            let l: Vec<u16> = g.ps[p].hand.iter().filter(|c| {
                let d = def(c.c);
                (d.main.kind == Kind::Spell && d.main.cost <= 5) || d.spell.map(|s| s.cost <= 5).unwrap_or(false)
            }).map(|c| c.uid).collect();
            if !can_cast_spell(g, p, 5) {
                return vec![Opt::Pass];
            }
            with_pass(picks(&l))
        }
        K::SozanDraw => vec![Opt::Mode(0), Opt::Mode(1), Opt::Mode(2)],
        K::SozanCancel => with_pass(picks(&g.ps[p].hand.iter().map(|c| c.uid).collect::<Vec<_>>())),
        K::Kusabira => vec![Opt::Mode(0), Opt::Mode(1), Opt::Mode(2)],
        K::KusaOppMana => picks(&uids(g, p)),
        K::KusaPut => {
            let l: Vec<u16> = g.ps[p].mana.iter().filter(|m| is_creature_card(m.card.c) && def(m.card.c).evo == Evo::None).map(|m| m.card.uid).collect();
            with_pass(picks(&l))
        }
        K::DarkMasters => {
            let l: Vec<u16> = g.ps[op].hand.iter().map(|c| c.uid).collect();
            if q.a <= 0 || l.is_empty() { vec![Opt::Pass] } else { with_pass(picks(&l)) }
        }
        K::JoJoJo => {
            let l: Vec<u16> = q.list.iter().copied().filter(|&u| deck_card(g, p, u).map(|c| is_jokers(c) && is_creature_card(c)).unwrap_or(false)).collect();
            with_pass(picks(&l))
        }
        K::JogimeraPick => {
            let l: Vec<u16> = q.list.iter().copied().filter(|&u| deck_card(g, p, u).map(is_jokers).unwrap_or(false)).collect();
            with_pass(picks(&l))
        }
        K::SlottonPick => {
            let l: Vec<u16> = q.list.iter().copied().filter(|&u| deck_card(g, p, u).map(is_creature_card).unwrap_or(false)).collect();
            picks(&l)
        }
        K::Slot7Pick => picks(&q.list.iter().copied().filter(|&u| deck_card(g, op, u).is_some()).collect::<Vec<_>>()),
        K::BrandDiscard => {
            let n = g.ps[p].hand.len().min(g.ps[op].battle.len()).min(6);
            (0..=n as u8).map(Opt::Mode).collect()
        }
        K::BrandKill => {
            let l: Vec<u16> = g.ps[op].battle.iter().enumerate().filter(|(i, x)| !x.jd && g.power(op, *i) <= 6000).map(|(_, x)| x.card.uid).collect();
            if l.is_empty() { vec![Opt::Pass] } else { picks(&l) }
        }
        K::MelaPut => {
            if q.a <= 0 {
                return vec![Opt::Pass];
            }
            let l: Vec<u16> = g.ps[p].hand.iter().filter(|c| is_jokers(c.c) && is_creature_card(c.c) && def(c.c).evo == Evo::None).map(|c| c.uid).collect();
            with_pass(picks(&l))
        }
        K::JogiragonUse => yn,
        K::MoujaShield => picks(&g.ps[op].shields.iter().map(|s| s.card.uid).collect::<Vec<_>>()),
        K::MoujaCre => {
            let l = targetable(g, op);
            if l.is_empty() { vec![Opt::Pass] } else { picks(&l) }
        }
        K::NiyaniMode => {
            let mut v = vec![Opt::Mode(0)];
            if !jbig_cands(g, p, q.list[0]).is_empty() {
                v.push(Opt::Mode(1));
            }
            v
        }
        K::JBig => q.list.iter().map(|&c| Opt::Mode(c as u8)).collect(),
        K::Kesshin => vec![Opt::Mode(0), Opt::Mode(1), Opt::Mode(2)],
        K::KesshinBottom => {
            let mut l = vec![];
            for pp in 0..2 {
                for x in g.ps[pp].battle.iter() {
                    if cost_of(x.card.c) <= 3 && !(pp == op && x.jd) {
                        l.push(x.card.uid);
                    }
                }
            }
            if l.is_empty() { vec![Opt::Pass] } else { picks(&l) }
        }
        K::KesshinAtk => {
            let l = uids(g, p);
            if l.is_empty() { vec![Opt::Pass] } else { picks(&l) }
        }
        K::KesshinIgnore => {
            let l = targetable(g, op);
            if l.is_empty() { vec![Opt::Pass] } else { picks(&l) }
        }
        K::SakkiPick => {
            let me = q.list[0];
            let mut l = vec![];
            for pp in 0..2 {
                for x in g.ps[pp].battle.iter() {
                    if x.card.uid != me && !(pp == op && x.jd) {
                        l.push(x.card.uid);
                    }
                }
            }
            with_pass(picks(&l))
        }
        K::SakkiPut => {
            let l: Vec<u16> = g.ps[p].hand.iter().filter(|c| is_creature_card(c.c) && def(c.c).evo == Evo::None && cost_of(c.c) as i32 <= q.b).map(|c| c.uid).collect();
            with_pass(picks(&l))
        }
        K::ManhattanKeep => {
            let l: Vec<u16> = g.ps[op].shields.iter().map(|s| s.card.uid).filter(|u| !q.list[1..].contains(u)).collect();
            picks(&l)
        }
        K::ManhattanDiscard => with_pass(picks(&g.ps[p].hand.iter().map(|c| c.uid).collect::<Vec<_>>())),
        K::DogiPut => {
            if q.b <= 0 {
                return vec![Opt::Pass];
            }
            let mut l: Vec<u16> = g.ps[p].hand.iter().filter(|c| is_creature_card(c.c) && def(c.c).evo == Evo::None && cost_of(c.c) as i32 <= q.a).map(|c| c.uid).collect();
            l.extend(g.ps[p].mana.iter().filter(|m| is_creature_card(m.card.c) && def(m.card.c).evo == Evo::None && cost_of(m.card.c) as i32 <= q.a).map(|m| m.card.uid));
            with_pass(picks(&l))
        }
        K::FujisanReturn => {
            let l: Vec<u16> = g.ps[p].battle.iter().filter(|x| is_jokers(x.card.c)).map(|x| x.card.uid).collect();
            let mut v = picks(&l);
            if q.a == 0 {
                v.insert(0, Opt::Pass);
            }
            v
        }
        K::PPPReturn => picks(&g.ps[p].mana.iter().map(|m| m.card.uid).collect::<Vec<_>>()),
        K::RevChange => with_pass(picks(&q.list)),
        K::DDDBase => {
            let l: Vec<u16> = g.ps[p].battle.iter().filter(|x| evo_ok(C::RedZone, x.card.c)).map(|x| x.card.uid).collect();
            with_pass(picks(&l))
        }
        K::SakkiAtk | K::LaffChange | K::ChooseRepl | K::FujiPick | K::EvoBase => vec![Opt::Pass],
        // ---- 2026-11-26 追加 ----
        K::SaintRepl | K::WillideDraw => yn,
        K::ZazeMode => {
            let mut v = vec![Opt::Mode(0)];
            if !targetable(g, op).is_empty() {
                v.push(Opt::Mode(1));
            }
            v.push(Opt::Mode(2));
            v
        }
        K::EverShield => {
            // 相手のクリーチャー（選べるもの）と自分のクリーチャー、または山札の上
            let mut l = targetable(g, op);
            l.extend(uids(g, p));
            let mut v = with_pass(picks(&l));
            if !g.ps[p].deck.is_empty() {
                v.push(Opt::Mode(0));
            }
            v
        }
        K::WillidePut => {
            let l: Vec<u16> = g.ps[p].hand.iter().filter(|c| is_creature_card(c.c) && def(c.c).evo != Evo::Evo && cost_of(c.c) as i32 <= q.b).map(|c| c.uid).collect();
            with_pass(picks(&l))
        }
        K::EmoName => {
            let me = q.list[0];
            let mut v: Vec<Opt> = vec![];
            for pp in [op, p] {
                for x in g.ps[pp].battle.iter() {
                    let o = Opt::Mode(x.card.c as u8);
                    if x.card.uid != me && x.card.c != C::Emotional && !v.contains(&o) {
                        v.push(o);
                    }
                }
            }
            with_pass(v)
        }
        K::GorgiReturn => {
            if q.a <= 0 {
                return vec![Opt::Pass];
            }
            with_pass(picks(&g.ps[p].mana.iter().map(|m| m.card.uid).collect::<Vec<_>>()))
        }
        K::CanzoneRaid => {
            let l: Vec<u16> = q.list.iter().copied().filter(|&u| deck_card(g, p, u).map(|c| (is_creature_card(c) && def(c).main.civ & (L | W | N) != 0) || c == C::Jasmine).unwrap_or(false)).collect();
            with_pass(picks(&l))
        }
        K::CanzoneShield => {
            let l: Vec<u16> = q.list.iter().copied().filter(|&u| g.find(op, u).is_some()).collect();
            with_pass(picks(&l))
        }
        K::CanzoneDeath => {
            let l: Vec<u16> = g.ps[p].battle.iter().filter(|x| def(x.card.c).evo != Evo::None).map(|x| x.card.uid).collect();
            with_pass(picks(&l))
        }
        K::AkanePick => {
            let mut v = vec![];
            for (k, &u) in q.list.iter().enumerate() {
                if deck_card(g, p, u).is_some() {
                    v.push(Opt::Pick(u));
                    v.push(Opt::Mode(k as u8));
                }
            }
            v
        }
        K::TakaLook => {
            let l: Vec<u16> = q.list.iter().copied().filter(|&u| deck_card(g, p, u).map(is_creature_card).unwrap_or(false)).collect();
            with_pass(picks(&l))
        }
    }
}

fn jbig_cands(g: &Game, p: usize, me: u16) -> Vec<u16> {
    let mut v: Vec<u16> = vec![];
    for x in g.ps[p].battle.iter() {
        if x.card.uid != me && is_jokers(x.card.c) && has_enter_ability(x.card.c) && x.card.c != C::Niyani && !v.contains(&(x.card.c as u16)) {
            v.push(x.card.c as u16);
        }
    }
    v
}

pub fn answer(g: &mut Game, _player: usize, q: EQ, o: Opt) {
    let p = q.p;
    let op = 1 - p;
    let pick = if let Opt::Pick(u) = o { Some(u) } else { None };
    match q.k {
        K::VmPick | K::VmYes | K::VmMode => crate::textfx::answer(g, q, o),
        K::EvoBase => {
            // list = [支払うか, 面, how, 攻撃中のクリーチャー(D・D・D)]。b: 0=召喚 1=J・O・E 2=コストなしの召喚
            let uid = q.a as u16;
            let Some(gi) = g.ps[p].grave.iter().position(|c| c.uid == uid) else { return };
            let c = g.ps[p].grave[gi].c;
            if q.list.first() == Some(&1) {
                let (cost, civ) = play_cost_b(g, p, c, q.list[1] as u8, q.list[2] as u8, pick);
                let Some(plan) = g.pay_plan(p, civ, cost) else {
                    let card = g.ps[p].grave.remove(gi);
                    g.ps[p].hand.push(card);
                    return;
                };
                g.pay(p, &plan);
            }
            let card = g.ps[p].grave.remove(gi);
            mark_summoned(g, uid);
            let how = if q.b == 2 { How::FreeSummon } else { How::Summon };
            enter(g, p, card, how, pick, if q.b == 1 { 2 } else { 0 });
            // D・D・D で攻撃中のクリーチャーに重ねた場合は攻撃を続ける
            let att = q.list.get(3).copied().unwrap_or(0);
            if att != 0 && pick == Some(att) && g.find(p, uid).is_some() {
                retarget_attacker(g, att, uid);
            }
        }
        K::Kiyuri => {
            // 選んだカードは一時置き場へ。残りを山札の下に置いてから、まとめて出す
            if let Some(u) = pick {
                let card = take_deck(g, p, u).unwrap();
                g.limbo.push(card);
                let mut l = q.list.clone();
                l.retain(|&x| x != u);
                push(g, Fx::Ask(EQ { k: K::Kiyuri, p, a: 0, b: q.b - 1, list: l }));
            } else {
                send_looked_to_bottom(g, p, &q.list);
                let cards = std::mem::take(&mut g.limbo);
                for card in cards.into_iter().rev() {
                    enter(g, p, card, How::Put, None, 0);
                }
            }
        }
        K::HumptyDiscard => {
            if let Some(u) = pick {
                let d = def(hand_card(g, op, u).unwrap());
                discard(g, op, u);
                let costs = [d.main.cost, d.spell.map(|s| s.cost).unwrap_or(255)];
                let l: Vec<u16> = g.ps[op].battle.iter().filter(|x| !x.jd && costs.contains(&cost_of(x.card.c))).map(|x| x.card.uid).collect();
                if !l.is_empty() {
                    ask(g, K::HumptyKill, p, 0, 0, l);
                }
            }
        }
        K::HumptyKill | K::Jolly => {
            if let Some(u) = pick {
                g.stack.push(Task::Destroy { p: op, uid: u, by_opp: true });
            }
        }
        K::GStrike => {
            if let Some(u) = pick {
                if let Some(i) = g.find(op, u) {
                    g.ps[op].battle[i].cant_attack = true;
                }
            }
        }
        K::ChakraTap => {
            if let Some(u) = pick {
                if let Some(i) = g.find(op, u) {
                    g.ps[op].battle[i].tapped = true;
                    g.ps[op].battle[i].no_untap = true;
                }
            }
        }
        K::DaguraShield => {
            if let Some(u) = pick {
                if let Some((s, under)) = take_shield(g, p, u) {
                    if s.faceup && def(s.card.c).main.kind == Kind::GCastle {
                        g.ps[p].grave.push(s.card);
                    } else {
                        g.put_mana(p, s.card, false);
                    }
                    for c in under {
                        g.put_mana(p, c, false);
                    }
                    push(g, Fx::Zrush { p });
                }
            }
        }
        K::DaguraMana | K::KusaPut | K::DogiPut => {
            if let Some(u) = pick {
                let from_mana = g.ps[p].mana.iter().any(|m| m.card.uid == u);
                let card = if from_mana { g.take_mana(p, u).unwrap() } else { g.take_hand(p, u).unwrap() };
                let cost = cost_of(card.c) as i32;
                if q.k == K::KusaPut && q.a > 1 {
                    push(g, Fx::Ask(EQ { k: K::KusaPut, p, a: q.a - 1, b: 0, list: vec![] }));
                }
                if q.k == K::DogiPut {
                    push(g, Fx::Ask(EQ { k: K::DogiPut, p, a: q.a - cost, b: q.b - 1, list: vec![] }));
                }
                enter(g, p, card, How::Put, None, 0);
            }
        }
        K::AntihamPut => {
            if let Some(u) = pick {
                let card = g.take_hand(p, u).unwrap();
                if enter(g, p, card, How::Put, None, 0) {
                    if let Some(i) = g.find(p, u) {
                        g.ps[p].battle[i].sa = true;
                        g.ps[p].battle[i].power_mod += 3000;
                    }
                }
            }
        }
        K::CottonSpell => {
            if let Some(u) = pick {
                leave(g, op, u, Dest::DeckBottom, true);
            }
        }
        K::Discard => {
            if let Some(u) = pick {
                let c = discard(g, p, u);
                if q.a > 1 {
                    push(g, Fx::Ask(EQ { k: K::Discard, p, a: q.a - 1, b: q.b, list: q.list.clone() }));
                }
                match (q.b, c) {
                    (1, Some(c)) => {
                        // モウジャキンガ：多色を捨てたらアンタップ
                        if is_multi(c) {
                            if let Some(i) = g.find(p, q.list[0]) {
                                g.ps[p].battle[i].tapped = false;
                            }
                        }
                    }
                    (2, Some(c)) => {
                        // ジョギラゴン：ジョーカーズなら出た時能力を1つ使ってもよい
                        if is_jokers(c) && has_enter_ability(c) {
                            ask(g, K::JogiragonUse, p, c as i32, q.list[0] as i32, vec![]);
                        }
                    }
                    _ => {}
                }
            }
        }
        K::JogiragonUse => {
            if o == Opt::Yes {
                let c = all()[q.a as usize];
                enter_ability(g, p, q.b as u16, c);
            }
        }
        K::GyaiMill => {
            // 前半だけ・後半だけでもよい。どちらかを使えばそのターンはもう使えない（QA 49213）
            if g.find(p, q.a as u16).map(|i| g.ps[p].battle[i].t1 == g.turn).unwrap_or(false) {
                return; // 同じターンに先に使った（別のドラゴンの誘発の解決で）
            }
            if o == Opt::Yes {
                t1_mark(g, p, q.a as u16);
                g.mill_to_mana(p, 2, true);
            }
            ask(g, K::GyaiReturn, p, q.a, 0, vec![]);
        }
        K::GyaiReturn => {
            if let Some(u) = pick {
                if let Some(c) = g.take_mana(p, u) {
                    g.ps[p].hand.push(c);
                }
                t1_mark(g, p, q.a as u16);
            }
        }
        K::PPPReturn => {
            if let Some(u) = pick {
                if let Some(c) = g.take_mana(p, u) {
                    g.ps[p].hand.push(c);
                }
            }
        }
        K::AshDraw => {
            if o == Opt::Yes && !g.find(p, q.a as u16).map(|i| g.ps[p].battle[i].t1 == g.turn).unwrap_or(false) {
                t1_mark(g, p, q.a as u16);
                g.draw1(p);
            }
        }
        K::Gyakuten => {
            if q.a == -1 {
                // 逆転撃を使うか
                if o == Opt::Yes && !g.attack_gyakuten_used {
                    g.attack_gyakuten_used = true;
                    if let Some(card) = g.take_hand(p, q.b as u16) {
                        if can_cast_spell(g, p, 4) {
                            g.ps[p].debt += 3;
                            cast(g, p, card, 0, true);
                        } else {
                            g.ps[p].hand.push(card);
                        }
                    }
                }
            } else if let Some(u) = pick {
                if let Some(i) = g.find(op, u) {
                    g.ps[op].battle[i].minus_inf = true;
                }
            }
        }
        K::Yabe => {
            if let Some(u) = pick {
                if let Some(i) = g.find(op, u) {
                    g.ps[op].battle[i].power_mod -= 2000;
                }
            }
        }
        K::KernelLock => {
            if let Some(u) = pick {
                if let Some(i) = g.find(op, u) {
                    g.ps[op].battle[i].lock = true;
                }
            }
        }
        K::ReturnShield => {
            if let Some(u) = pick {
                if let Some((s, under)) = take_shield(g, p, u) {
                    if s.faceup && def(s.card.c).main.kind == Kind::GCastle {
                        g.ps[p].grave.push(s.card);
                    } else {
                        g.ps[p].hand.push(s.card);
                    }
                    g.ps[p].hand.extend(under);
                    push(g, Fx::Zrush { p });
                }
            }
        }
        K::MiracleSpell => {
            if let Some(u) = pick {
                let c = hand_card(g, p, u).unwrap();
                let side = if def(c).main.kind == Kind::Spell { 0 } else { 1 };
                let card = g.take_hand(p, u).unwrap();
                cast(g, p, card, side, true);
            }
        }
        K::SozanDraw => {
            if let Opt::Mode(n) = o {
                g.stack.push(Task::Draw { p, n });
            }
        }
        K::SozanCancel => {
            if let Some(u) = pick {
                discard(g, p, u);
                g.ps[p].once |= 1;
                g.attack_cancel = true;
            }
        }
        K::Kusabira => {
            if let Opt::Mode(n) = o {
                g.mill_to_mana(p, n as usize, false);
            }
            if g.ps[p].mana.len() >= 12 {
                push(g, Fx::Ask(EQ { k: K::KusaPut, p, a: 2, b: 0, list: vec![] }));
                let n = g.ps[op].battle.len().min(2);
                if n > 0 {
                    push(g, Fx::Ask(EQ { k: K::KusaOppMana, p: op, a: n as i32, b: 0, list: vec![] }));
                }
            }
        }
        K::KusaOppMana => {
            if let Some(u) = pick {
                leave(g, p, u, Dest::Mana, true);
                if q.a > 1 {
                    push(g, Fx::Ask(EQ { k: K::KusaOppMana, p, a: q.a - 1, b: 0, list: vec![] }));
                }
            }
        }
        K::DarkMasters => {
            if let Some(u) = pick {
                discard(g, op, u);
                push(g, Fx::Ask(EQ { k: K::DarkMasters, p, a: q.a - 1, b: 0, list: vec![] }));
            }
        }
        K::JoJoJo | K::JogimeraPick => {
            if let Some(u) = pick {
                let c = take_deck(g, p, u).unwrap();
                g.ps[p].hand.push(c);
            }
            let rest: Vec<u16> = q.list.iter().copied().filter(|&x| Some(x) != pick).collect();
            send_looked_to_bottom(g, p, &rest);
        }
        K::SlottonPick => {
            let u = pick.unwrap();
            let card = take_deck(g, p, u).unwrap();
            let rest: Vec<u16> = q.list.iter().copied().filter(|&x| x != u).collect();
            send_looked_to_bottom(g, p, &rest);
            if enter(g, p, card, How::Put, None, 0) {
                if let Some(i) = g.find(p, u) {
                    g.ps[p].battle[i].player_ok = true;
                }
            }
        }
        K::Slot7Pick => {
            let u = pick.unwrap();
            let c = deck_card(g, op, u).unwrap();
            let d = def(c);
            let costs = [d.main.cost, d.spell.map(|s| s.cost).unwrap_or(255)];
            send_looked_to_bottom(g, op, &q.list);
            let l: Vec<u16> = g.ps[op].battle.iter().filter(|x| costs.contains(&cost_of(x.card.c))).map(|x| x.card.uid).collect();
            for u in l {
                leave(g, op, u, Dest::DeckBottom, true);
            }
        }
        K::BrandDiscard => {
            if let Opt::Mode(n) = o {
                if n > 0 {
                    push(g, Fx::Ask(EQ { k: K::BrandKill, p, a: n as i32, b: 0, list: vec![] }));
                    ask(g, K::Discard, p, n as i32, 0, vec![]);
                }
            }
        }
        K::BrandKill => {
            if let Some(u) = pick {
                g.stack.push(Task::Destroy { p: op, uid: u, by_opp: true });
                if q.a > 1 {
                    // 次の破壊は今の破壊の後で選ぶ
                    let i = g.stack.len() - 1;
                    g.stack.insert(i, Task::Fx(Fx::Ask(EQ { k: K::BrandKill, p, a: q.a - 1, b: 0, list: vec![] })));
                }
            }
        }
        K::MelaPut => {
            if let Some(u) = pick {
                let card = g.take_hand(p, u).unwrap();
                push(g, Fx::Ask(EQ { k: K::MelaPut, p, a: q.a - 1, b: 0, list: vec![] }));
                if enter(g, p, card, How::Put, None, 0) {
                    if let Some(i) = g.find(p, u) {
                        g.ps[p].battle[i].sa = true;
                        g.ps[p].battle[i].bottom = 3;
                    }
                }
            }
        }
        K::MoujaShield => {
            let s = pick;
            ask(g, K::MoujaCre, p, 0, 0, vec![s.unwrap_or(0)]);
        }
        K::MoujaCre => {
            // シールド1つ・クリーチャー1体・手札1枚（見ないで）を墓地へ
            let su = q.list.first().copied().unwrap_or(0);
            if let Some((s, under)) = take_shield(g, op, su) {
                g.ps[op].grave.push(s.card);
                g.ps[op].grave.extend(under);
                push(g, Fx::Zrush { p: op });
            }
            if let Some(u) = pick {
                leave(g, op, u, Dest::Grave, true);
            }
            random_discard(g, op);
        }
        K::NiyaniMode => match o {
            Opt::Mode(0) => {
                for c in g.ps[op].battle.iter_mut() {
                    c.tapped = true;
                }
            }
            _ => {
                let l = jbig_cands(g, p, q.list[0]);
                ask(g, K::JBig, p, 0, q.list[0] as i32, l);
            }
        },
        K::JBig => {
            if let Opt::Mode(ci) = o {
                enter_ability(g, p, q.b as u16, all()[ci as usize]);
            }
        }
        K::Kesshin => {
            if q.a > 1 {
                push(g, Fx::Ask(EQ { k: K::Kesshin, p, a: q.a - 1, b: 0, list: vec![] }));
            }
            let k = match o {
                Opt::Mode(0) => K::KesshinBottom,
                Opt::Mode(1) => K::KesshinAtk,
                _ => K::KesshinIgnore,
            };
            ask(g, k, p, 0, 0, vec![]);
        }
        K::KesshinBottom => {
            if let Some(u) = pick {
                if let Some((pp, _)) = g.find_any(u) {
                    leave(g, pp, u, Dest::DeckBottom, pp != p);
                }
            }
        }
        K::KesshinAtk => {
            if let Some(u) = pick {
                if let Some(i) = g.find(p, u) {
                    g.ps[p].battle[i].player_ok = true;
                }
            }
        }
        K::KesshinIgnore => {
            if let Some(u) = pick {
                if let Some(i) = g.find(op, u) {
                    g.ps[op].battle[i].ignored = true;
                }
            }
        }
        K::SakkiPick => {
            if let Some(u) = pick {
                if let Some((pp, i)) = g.find_any(u) {
                    let cost = cost_of(g.ps[pp].battle[i].card.c) as i32;
                    if leave(g, pp, u, Dest::DeckBottom, pp != p) {
                        g.draw1(pp);
                        ask(g, K::SakkiPut, pp, 0, cost, vec![]);
                    }
                }
            }
        }
        K::SakkiPut => {
            if let Some(u) = pick {
                let card = g.take_hand(p, u).unwrap();
                enter(g, p, card, How::Put, None, 0);
            }
        }
        K::ManhattanKeep => {
            if let Some(u) = pick {
                let mut l = q.list.clone();
                l.push(u);
                if q.a > 1 {
                    ask(g, K::ManhattanKeep, p, q.a - 1, 0, l);
                } else {
                    if let Some(i) = g.find(p, l[0]) {
                        g.ps[p].battle[i].keep = l[1..].to_vec();
                    }
                    ask(g, K::ManhattanDiscard, p, 0, 0, vec![]);
                }
            }
        }
        K::ManhattanDiscard => {
            if let Some(u) = pick {
                discard(g, p, u);
                g.ps[op].creature_limit_next_turn = true;
            }
        }
        K::DogiEnd => {
            if o == Opt::Yes {
                g.mill_to_mana(p, 2, false);
            }
            ask(g, K::DogiPut, p, 7, 2, vec![]);
        }
        K::FujisanReturn => {
            if let Some(u) = pick {
                let cost = g.find(p, u).map(|i| cost_of(g.ps[p].battle[i].card.c) as i32).unwrap_or(0);
                leave(g, p, u, Dest::Hand, false);
                let total = q.a + cost;
                if total >= 7 {
                    if let Some(card) = g.take_hand(p, q.b as u16) {
                        enter(g, p, card, How::Put, None, 0);
                    }
                } else {
                    ask(g, K::FujisanReturn, p, total, q.b, vec![]);
                }
            }
        }
        K::PPPEnd => {
            if o == Opt::Yes {
                g.mill_to_mana(p, 2, false);
                ask(g, K::PPPReturn, p, 0, 0, vec![]);
            }
        }
        K::GaiashSummon => {
            if o == Opt::Yes {
                if let Some(card) = g.take_hand(p, q.a as u16) {
                    mark_summoned(g, card.uid);
                    enter(g, p, card, How::FreeSummon, None, 0);
                }
            }
        }
        K::Ninja => {
            if o == Opt::Yes && !g.attack_ninja_used {
                g.attack_ninja_used = true;
                if let Some(card) = g.take_hand(p, q.a as u16) {
                    let u = card.uid;
                    if enter(g, p, card, How::FreeSummon, None, 0) {
                        if let Some(i) = g.find(p, u) {
                            g.ps[p].battle[i].bottom = 1;
                        }
                    }
                }
            }
        }
        K::Kirifuda => {
            if o == Opt::Yes {
                if let Some(plan) = g.pay_plan(p, W | D | F, 5) {
                    g.pay(p, &plan);
                    if let Some(card) = g.take_hand(p, q.a as u16) {
                        mark_summoned(g, card.uid);
                        enter(g, p, card, How::Summon, None, 0);
                    }
                }
            }
        }
        K::SmashBurst => {
            if o == Opt::Yes {
                let u = q.list[0];
                if let Some(i) = g.find(p, u) {
                    let c = g.ps[p].battle[i].card.c;
                    let s = side_of(c, 1);
                    if can_cast_spell(g, p, s.cost) {
                        g.ps[p].spells_this_turn += 1;
                        g.ps[p].no_free_this_turn = true;
                        first_event(g, p, s.cost);
                        spell_effect(g, p, c, 1);
                    }
                }
            }
        }
        K::RevChange => {
            if let Some(u) = pick {
                let att = q.b as u16;
                rev_change(g, p, att, u);
            }
        }
        K::DDD => {
            if o == Opt::Yes {
                if q.list.first() == Some(&1) {
                    // TAKASUGI：支払ってから進化元を選ぶ
                    let Some(plan) = g.pay_plan(p, N, ddd_taka_cost(g, p)) else { return };
                    let Some(card) = g.take_hand(p, q.a as u16) else { return };
                    g.pay(p, &plan);
                    let o = evo_choices(g, p, C::Takasugi, 0, 0, false);
                    g.ps[p].grave.push(card);
                    g.ask(p, DK::Effect, o, Q::Custom(EQ { k: K::EvoBase, p, a: q.a, b: 0, list: vec![0, 0, 0, q.b as u16] }));
                } else {
                    ask(g, K::DDDBase, p, q.a, q.b, vec![]);
                }
            }
        }
        K::DDDBase => {
            let Some(plan) = g.pay_plan(p, F, 4) else { return };
            let Some(card) = g.take_hand(p, q.a as u16) else { return };
            g.pay(p, &plan);
            mark_summoned(g, card.uid);
            enter(g, p, card, How::Summon, pick, 0);
            // 攻撃中のクリーチャーに重ねた場合は攻撃を続ける（uid を差し替え）
            if pick == Some(q.b as u16) {
                retarget_attacker(g, q.b as u16, card.uid);
            }
        }
        K::SakkiAtk | K::LaffChange | K::ChooseRepl | K::FujiPick => {}
        // ---- 2026-11-26 追加 ----
        K::SaintRepl => {
            if o == Opt::Yes {
                // 積んであるブレイク（または全部ブレイク）を、セイント・キャッスル1つのブレイクに置き換える
                let su = q.a as u16;
                if g.ps[p].shields.iter().any(|s| s.card.uid == su) {
                    if matches!(g.stack.last(), Some(Task::Break { .. }) | Some(Task::ShieldsToHand { .. })) {
                        g.stack.pop();
                    }
                    g.saint = Some(su);
                    g.stack.push(Task::ShieldsToHand { owner: p, uids: vec![su], broken: true });
                }
            }
        }
        K::ZazeMode => {
            let z = q.list[0];
            if let Some(i) = g.find(p, z) {
                g.ps[p].battle[i].tapped = false;
            }
            match o {
                Opt::Mode(0) => {
                    g.draw1(p);
                }
                Opt::Mode(1) => {
                    let l = targetable(g, op);
                    if !l.is_empty() {
                        ask(g, K::ChakraTap, p, 0, 0, l);
                    }
                }
                _ => {
                    if let Some(i) = g.find(p, z) {
                        g.ps[p].battle[i].stay = true;
                    }
                }
            }
        }
        K::EverShield => match o {
            Opt::Pick(u) => {
                if let Some((pp, _)) = g.find_any(u) {
                    leave(g, pp, u, Dest::Shield, pp != p);
                }
            }
            Opt::Mode(0) => {
                shieldify_top(g, p);
            }
            _ => {}
        },
        K::WillideDraw => {
            if o == Opt::Yes {
                g.draw1(p);
            }
            ask(g, K::WillidePut, p, 0, q.b, vec![]);
        }
        K::WillidePut => {
            if let Some(u) = pick {
                if let Some(card) = g.take_hand(p, u) {
                    enter(g, p, card, How::Put, None, 0);
                }
            }
        }
        K::EmoName => {
            if let Opt::Mode(ci) = o {
                if let Some(i) = g.find(p, q.list[0]) {
                    g.ps[p].battle[i].emo = Some(all()[ci as usize]);
                }
                recompute_emo(g);
            }
        }
        K::GorgiReturn => {
            if let Some(u) = pick {
                if let Some(c) = g.take_mana(p, u) {
                    g.ps[p].hand.push(c);
                }
                if q.a > 1 {
                    ask(g, K::GorgiReturn, p, q.a - 1, 0, vec![]);
                }
            }
        }
        K::CanzoneRaid => {
            if let Some(u) = pick {
                let card = take_deck(g, p, u).unwrap();
                if let Some(i) = g.find(p, q.b as u16) {
                    g.ps[p].battle[i].under.push(card);
                } else {
                    g.ps[p].deck.insert(0, card);
                }
            }
            let rest: Vec<u16> = q.list.iter().copied().filter(|&x| Some(x) != pick).collect();
            send_looked_to_bottom(g, p, &rest);
        }
        K::CanzoneShield => {
            if let Some(u) = pick {
                leave(g, op, u, Dest::Shield, true);
            }
        }
        K::CanzoneDeath => {
            let mut cards = vec![];
            for &u in q.list.iter() {
                if let Some(gi) = g.ps[p].grave.iter().position(|c| c.uid == u) {
                    cards.push(g.ps[p].grave.remove(gi));
                }
            }
            if cards.is_empty() {
                return;
            }
            match pick.and_then(|u| g.find(p, u)) {
                Some(i) => g.ps[p].battle[i].under.extend(cards),
                None => {
                    if fujin_blocks(g, p) {
                        g.ps[p].grave.extend(cards);
                    } else {
                        shield_stack(g, p, cards, true);
                    }
                }
            }
        }
        K::AkanePick => {
            let chosen = match o {
                Opt::Pick(u) => Some((u, false)),
                Opt::Mode(k) => q.list.get(k as usize).map(|&u| (u, true)),
                _ => None,
            };
            if let Some((u, to_mana)) = chosen {
                if let Some(card) = take_deck(g, p, u) {
                    if to_mana {
                        g.put_mana(p, card, false);
                    } else {
                        g.ps[p].hand.push(card);
                    }
                }
                let rest: Vec<u16> = q.list.iter().copied().filter(|&x| x != u).collect();
                send_looked_to_bottom(g, p, &rest);
            } else {
                send_looked_to_bottom(g, p, &q.list);
            }
        }
        K::TakaLook => {
            if let Some(u) = pick {
                let c = take_deck(g, p, u).unwrap();
                g.ps[p].hand.push(c);
            }
            let rest: Vec<u16> = q.list.iter().copied().filter(|&x| Some(x) != pick).collect();
            send_looked_to_bottom(g, p, &rest);
        }
    }
}

/// 革命チェンジ：攻撃中のクリーチャーと手札のカードを入れ替える
fn rev_change(g: &mut Game, p: usize, att: u16, hand_uid: u16) {
    let Some(i) = g.find(p, att) else { return };
    let Some(c) = hand_card(g, p, hand_uid) else { return };
    // 入れ替え先が置換効果で正常に出られないなら入れ替えは行われない（701.26b）
    if enter_replacement(g, p, c, How::Put).is_some() {
        return;
    }
    if creature_blocked(g, p) {
        return;
    }
    let old = g.ps[p].battle.remove(i);
    g.ps[p].hand.push(old.card);
    for u in old.under {
        g.ps[p].hand.push(u);
    }
    let card = g.take_hand(p, hand_uid).unwrap();
    g.revch.push(hand_uid);
    enter(g, p, card, How::Put, None, 0);
    g.revch.retain(|&u| u != hand_uid || g.stack.iter().any(|t| matches!(t, Task::Fx(Fx::Enter { uid, .. }) if *uid == hand_uid)));
    if let Some(j) = g.find(p, hand_uid) {
        g.ps[p].battle[j].tapped = true; // 攻撃状態を引き継ぐ
    }
    retarget_attacker(g, att, hand_uid);
}

fn retarget_attacker(g: &mut Game, old: u16, new: u16) {
    for t in g.stack.iter_mut() {
        match t {
            Task::BlockStep { att, .. } | Task::Hit { att, .. } | Task::AttackEnd { att } if *att == old => *att = new,
            _ => {}
        }
    }
}
