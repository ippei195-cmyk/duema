//! 白緑ウィリデ・ゴルギーオージャーの18種類（2026-11-26）の場面テスト。根拠は newdecks-rulings.md
use dmengine::cards::*;
use dmengine::effects::*;
use dmengine::engine::*;

fn blank(tp: usize) -> Game {
    let mut g = Game::new(&deck("gorgi"), &deck("willide"), 1);
    for p in 0..2 {
        let q = &mut g.ps[p];
        let mut all: Vec<Card> = vec![];
        all.append(&mut q.hand);
        all.extend(q.mana.drain(..).map(|m| m.card));
        all.extend(q.shields.drain(..).map(|s| s.card));
        all.extend(q.battle.drain(..).map(|c| c.card));
        all.append(&mut q.grave);
        all.append(&mut q.tama);
        q.deck.extend(all);
    }
    g.tp = tp;
    g.turn = 5;
    g.stack.clear();
    g.decision = None;
    g
}

fn take(g: &mut Game, p: usize, c: C) -> Card {
    let i = g.ps[p].deck.iter().position(|x| x.c == c).unwrap_or_else(|| panic!("{c:?} not in deck"));
    g.ps[p].deck.remove(i)
}

fn mana_of(g: &mut Game, p: usize, c: C) {
    let card = take(g, p, c);
    g.ps[p].mana.push(Mana { card, tapped: false, facedown: false });
}

fn put_battle(g: &mut Game, p: usize, card: Card) -> u16 {
    let mut c = g.new_cre(card);
    c.sick = false;
    c.entered = 0;
    g.ps[p].battle.push(c);
    card.uid
}

fn shields(g: &mut Game, p: usize, n: usize) {
    for _ in 0..n {
        let c = g.ps[p].deck.remove(0);
        g.ps[p].shields.push(Shield { card: c, faceup: false });
    }
}

/// 今の判断の選択肢から o を選ぶ
fn choose(g: &mut Game, o: Opt) {
    let d = g.decision.as_ref().expect("no decision");
    let i = d.opts.iter().position(|&x| x == o).unwrap_or_else(|| panic!("{o:?} not in {:?} ({:?})", d.opts, d.q));
    g.step(i);
}

fn k_of(g: &Game) -> Option<K> {
    match &g.decision.as_ref()?.q {
        Q::Custom(e) => Some(e.k),
        _ => None,
    }
}

#[test]
fn gorgi_cost_depends_on_base() {
    // 進化元に含まれるカード2枚 → 9-4=5。マナ5枚（光水自然）で出せる。進化元なしでは出せない
    let mut g = blank(0);
    for c in [C::Akane, C::Akane, C::Heracles, C::Cineraria, C::Takasugi] {
        mana_of(&mut g, 0, c);
    }
    let a = take(&mut g, 0, C::Ichion);
    let b = take(&mut g, 0, C::Kohaku);
    let base = put_battle(&mut g, 0, a);
    g.ps[0].battle[0].under.push(b);
    let gor = take(&mut g, 0, C::Gorgi);
    g.ps[0].hand.push(gor);
    // コハクの超魂X（下にある）で、さらに -1
    let (cost, _) = play_cost_b(&g, 0, C::Gorgi, 0, 0, Some(base));
    assert_eq!(cost, 4);
    assert_eq!(play_cost_b(&g, 0, C::Gorgi, 0, 0, None).0, 8);
    g.stack.push(Task::MainStep);
    g.run();
    let d = g.decision.clone().unwrap();
    let o = d.opts.iter().copied().find(|o| matches!(o, Opt::Play { uid, .. } if *uid == gor.uid)).expect("gorgi playable");
    choose(&mut g, o);
    // 進化元の選択肢に Pass（進化しない）はない（払えない）
    // 進化元の候補が1つ（進化しないでは払えない）ので自動で決まる
    let i = g.find(0, gor.uid).expect("gorgi in battle");
    assert_eq!(g.ps[0].battle[i].under.len(), 2);
    assert!(!g.ps[0].battle[i].sick, "進化クリーチャーは召喚酔いしない");
}

#[test]
fn gorgi_wins_with_ten_cards() {
    let mut g = blank(0);
    let gor = take(&mut g, 0, C::Gorgi);
    let u = put_battle(&mut g, 0, gor);
    for _ in 0..9 {
        let c = g.ps[0].deck.remove(0);
        g.ps[0].battle[0].under.push(c);
    }
    shields(&mut g, 1, 5);
    g.stack.push(Task::Attack { att: u, tgt: Tgt::Player });
    g.run();
    assert_eq!(g.winner, Some(0));
}

#[test]
fn saint_castle_replaces_whole_break() {
    // W・ブレイカーの攻撃 → かわりにセイント・キャッスルだけブレイクされ、手札に加わる（48504）
    let mut g = blank(0);
    let her = take(&mut g, 0, C::Heracles);
    let u = put_battle(&mut g, 0, her);
    shields(&mut g, 1, 3);
    let sc = take(&mut g, 1, C::SaintCastle);
    g.ps[1].shields.push(Shield { card: sc, faceup: true });
    g.stack.push(Task::Attack { att: u, tgt: Tgt::Player });
    g.run();
    // ブロッカーがいないので、セイント・キャッスルの質問
    assert_eq!(k_of(&g), Some(K::SaintRepl));
    choose(&mut g, Opt::Yes);
    while g.decision.as_ref().map(|d| d.player == 1 && d.kind == DK::Trigger).unwrap_or(false) {
        g.step(0);
    }
    assert_eq!(g.ps[1].shields.len(), 3);
    assert!(g.ps[1].hand.iter().any(|c| c.c == C::SaintCastle));
}

#[test]
fn zaze_untaps_and_draws_when_shield_placed() {
    let mut g = blank(1);
    let z = take(&mut g, 1, C::Zaze);
    let zu = put_battle(&mut g, 1, z);
    g.ps[1].battle[0].tapped = true;
    let ch = take(&mut g, 1, C::Chakra);
    enter(&mut g, 1, ch, How::Summon, None, 0);
    g.run();
    // チャクラの山札シールド化 → ザゼ・ゼーンのモード
    let mut seen = false;
    while let Some(k) = k_of(&g) {
        if k == K::ZazeMode {
            seen = true;
            let h = g.ps[1].hand.len();
            choose(&mut g, Opt::Mode(0));
            assert_eq!(g.ps[1].hand.len(), h + 1);
        } else {
            g.step(0);
        }
    }
    assert!(seen);
    let i = g.find(1, zu).unwrap();
    assert!(!g.ps[1].battle[i].tapped);
}

#[test]
fn zaze_enters_tapped_and_stays_tapped_at_turn_start() {
    let mut g = blank(1);
    let z = take(&mut g, 1, C::Zaze);
    enter(&mut g, 1, z, How::Summon, None, 0);
    let i = g.find(1, z.uid).unwrap();
    assert!(g.ps[1].battle[i].tapped);
    g.stack.clear();
    g.stack.push(Task::Untap);
    g.run();
    assert!(g.ps[1].battle[g.find(1, z.uid).unwrap()].tapped);
}

#[test]
fn ichion_limits_spells_and_evolved_limits_creatures() {
    let mut g = blank(0);
    let ic = take(&mut g, 1, C::Ichion);
    put_battle(&mut g, 1, ic);
    let ky0 = g.ps[0].deck.iter().position(|c| c.c == C::Kohaku).unwrap();
    let _ = ky0;
    g.ps[0].spells_this_turn = 1;
    // 自分（0）は2回目の呪文を唱えられない：呪文の選択肢が出ない
    let mut opts = vec![];
    let ky = Card { c: C::Kiyuri, uid: 900 };
    g.ps[0].hand.push(ky);
    for _ in 0..6 {
        let c = g.ps[0].deck.remove(0);
        g.ps[0].mana.push(Mana { card: c, tapped: false, facedown: false });
    }
    main_options(&g, &mut opts);
    assert!(!opts.iter().any(|o| matches!(o, Opt::Play { uid: 900, .. })));
    // 進化していない一音の妖精：クリーチャーは何体でも
    g.ps[0].entered_this_turn = 1;
    assert!(!creature_blocked(&g, 0));
    let k = take(&mut g, 1, C::Caddy);
    g.ps[1].battle[0].under.push(k);
    assert!(creature_blocked(&g, 0));
}

#[test]
fn vt5000_bounces_small_and_locks_next_turn() {
    let mut g = blank(0);
    let a = take(&mut g, 1, C::Cotton);
    let b = take(&mut g, 1, C::Willide);
    put_battle(&mut g, 1, a);
    put_battle(&mut g, 1, b);
    let vt = take(&mut g, 0, C::Vt5000);
    enter(&mut g, 0, vt, How::Summon, None, 0);
    g.run();
    // コットン2000は手札へ、ウィリデ8000は残る
    assert!(g.ps[1].hand.iter().any(|c| c.uid == a.uid));
    assert!(g.find(1, b.uid).is_some());
    assert_eq!(g.ps[1].small_lock_turn, g.turn + 1);
    g.turn += 1;
    assert!(small_locked(&g, 1, C::Cotton));
    assert!(!small_locked(&g, 1, C::Willide));
}

#[test]
fn kohaku_does_not_leave() {
    let mut g = blank(0);
    let k = take(&mut g, 0, C::Kohaku);
    enter(&mut g, 0, k, How::Summon, None, 0);
    g.run();
    g.stack.push(Task::Destroy { p: 0, uid: k.uid, by_opp: true });
    g.run();
    assert!(g.find(0, k.uid).is_some());
}

#[test]
fn willide_draws_and_puts_on_opponents_first_spell() {
    let mut g = blank(0);
    let w = take(&mut g, 1, C::Willide);
    put_battle(&mut g, 1, w);
    let co = take(&mut g, 1, C::Cotton);
    g.ps[1].hand.push(co); // コスト2
    let h = g.ps[1].hand.len();
    let s = take(&mut g, 0, C::Akane);
    enter(&mut g, 0, s, How::Summon, None, 0); // 相手（0）のエレメントがはじめて出た（コスト3）
    g.run();
    let mut drew = false;
    let mut put = false;
    while let Some(k) = k_of(&g) {
        match k {
            K::WillideDraw => {
                choose(&mut g, Opt::Yes);
                drew = true;
            }
            K::WillidePut => {
                choose(&mut g, Opt::Pick(co.uid));
                put = true;
            }
            _ => g.step(0),
        }
    }
    assert!(drew && put);
    assert!(g.find(1, co.uid).is_some());
    assert_eq!(g.ps[1].hand.len(), h); // 1枚引いて1枚出した
}

#[test]
fn takasugi_ddd_onto_attacker_keeps_attacking() {
    let mut g = blank(0);
    let kh = take(&mut g, 0, C::Kohaku); // 光
    let att = put_battle(&mut g, 0, kh);
    let t = take(&mut g, 0, C::Takasugi);
    g.ps[0].hand.push(t);
    mana_of(&mut g, 0, C::Jasmine);
    mana_of(&mut g, 0, C::Jasmine);
    shields(&mut g, 1, 2);
    g.stack.push(Task::Attack { att, tgt: Tgt::Player });
    g.run();
    while let Some(k) = k_of(&g) {
        match k {
            K::DDD => choose(&mut g, Opt::Yes),
            K::EvoBase => choose(&mut g, Opt::Pick(att)),
            _ => g.step(0),
        }
        if g.find(0, t.uid).is_some() {
            break;
        }
    }
    let i = g.find(0, t.uid).expect("takasugi in battle");
    assert_eq!(g.ps[0].battle[i].under[0].uid, kh.uid);
    assert!(g.stack.iter().any(|x| matches!(x, Task::Hit { att, .. } if *att == t.uid)));
}

#[test]
fn neo_on_jasmine_is_not_sick() {
    let mut g = blank(0);
    let j = take(&mut g, 0, C::Jasmine);
    g.ps[0].tama.push(j);
    let h = take(&mut g, 0, C::Heracles);
    enter(&mut g, 0, h, How::Summon, Some(j.uid), 0);
    let i = g.find(0, h.uid).unwrap();
    assert!(!g.ps[0].battle[i].sick);
    assert!(g.ps[0].tama.is_empty());
    assert_eq!(g.ps[0].battle[i].under[0].c, C::Jasmine);
}

#[test]
fn canzone_death_makes_one_faceup_shield() {
    let mut g = blank(1);
    let cz = take(&mut g, 0, C::Canzone);
    let u = put_battle(&mut g, 0, cz);
    let a = take(&mut g, 0, C::Kohaku);
    let b = take(&mut g, 0, C::Akane);
    g.ps[0].battle[0].under.extend([a, b]);
    let n = g.ps[0].shields.len();
    g.stack.push(Task::Destroy { p: 0, uid: u, by_opp: true });
    g.run();
    assert_eq!(k_of(&g), None);
    assert_eq!(g.ps[0].shields.len(), n + 1);
    assert!(g.ps[0].shields.last().unwrap().faceup);
    assert_eq!(g.ps[0].sh_under.len(), 1);
    assert_eq!(g.total_cards(0), 40);
}

#[test]
fn emotional_ignores_chosen_name() {
    let mut g = blank(1);
    let k = take(&mut g, 0, C::Cineraria);
    put_battle(&mut g, 0, k);
    let e = take(&mut g, 1, C::Emotional);
    enter(&mut g, 1, e, How::Summon, None, 0);
    g.run();
    while let Some(kk) = k_of(&g) {
        if kk == K::EmoName {
            choose(&mut g, Opt::Mode(C::Cineraria as u8));
        } else {
            g.step(0);
        }
    }
    assert!(g.ps[0].battle[0].ign());
    // ハードコアが離れると無視は終わる
    leave(&mut g, 1, e.uid, Dest::Hand, false);
    recompute_emo(&mut g);
    assert!(!g.ps[0].battle[0].ign());
}

#[test]
fn cineraria_saves_giant() {
    let mut g = blank(1);
    let c = take(&mut g, 0, C::Cineraria);
    let h = take(&mut g, 0, C::Heracles);
    put_battle(&mut g, 0, c);
    put_battle(&mut g, 0, h);
    g.stack.push(Task::Destroy { p: 0, uid: h.uid, by_opp: true });
    g.run();
    assert!(g.find(0, h.uid).is_some());
    assert!(g.find(0, c.uid).is_none());
    assert!(g.ps[0].grave.iter().any(|x| x.uid == c.uid));
}

#[test]
fn everlast_shieldifies_evolved_element_as_one_shield() {
    let mut g = blank(1);
    let a = take(&mut g, 0, C::Heracles);
    let b = take(&mut g, 0, C::Kohaku);
    let u = put_battle(&mut g, 0, a);
    g.ps[0].battle[0].under.push(b);
    let n = g.ps[0].shields.len();
    let e = take(&mut g, 1, C::Everlast);
    enter(&mut g, 1, e, How::Summon, None, 0);
    g.run();
    while let Some(k) = k_of(&g) {
        if k == K::EverShield {
            choose(&mut g, Opt::Pick(u));
        } else {
            g.step(0);
        }
    }
    assert_eq!(g.ps[0].shields.len(), n + 1);
    assert_eq!(g.ps[0].sh_under.len(), 1);
    assert!(g.ps[0].battle.is_empty());
}
