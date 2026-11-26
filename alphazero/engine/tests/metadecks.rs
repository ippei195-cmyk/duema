//! DMラボ環境デッキ 第1段階の新カード10種（2026-11-26）の場面テスト。
//! 根拠は /mnt/project-files/meta-decks/裁定メモ.md（公式Q&A番号つき）
use dmengine::cards::*;
use dmengine::effects::*;
use dmengine::engine::*;

fn blank(d0: &str, d1: &str, tp: usize) -> Game {
    let mut g = Game::new(&deck(d0), &deck(d1), 1);
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

fn counts_ok(g: &Game) {
    for p in 0..2 {
        assert_eq!(g.total_cards(p), 40, "p{p} のカードが40枚でない");
    }
}

#[test]
fn odotta_joe_cost_and_battle() {
    let mut g = blank("jokers_meta", "zaze", 0);
    let od = take(&mut g, 0, C::Odotta);
    assert_eq!(play_cost(&g, 0, C::Odotta, 0, 1).0, 2, "J・O・E 1 で 3-1");
    let cad = take(&mut g, 1, C::Cotton); // 2000（キャディだとコスト3>マナ2でマナ送りになる）
    let cu = put_battle(&mut g, 1, cad);
    mana_of(&mut g, 0, C::Brand);
    mana_of(&mut g, 0, C::Brand);
    g.ps[0].hand.push(od);
    play_from_hand(&mut g, 0, od.uid, 0, 1);
    g.run();
    assert_eq!(k_of(&g), Some(K::OdottaBattle));
    choose(&mut g, Opt::Pick(cu));
    assert!(g.find(1, cu).is_none(), "3000 と 2000 のバトルでコットンが負ける");
    let i = g.find(0, od.uid).unwrap();
    assert_eq!(g.ps[0].battle[i].bottom, 2, "ターンの終わりに山札の下へ置いて1枚引く");
    counts_ok(&g);
}

#[test]
fn stella_triggers_on_summon_not_on_put() {
    // 49665/49666：召喚・呪文で誘発、効果で出すだけでは誘発しない。46964：召喚したクリーチャーも攻撃できない
    let mut g = blank("bike_meta", "seitokai", 0);
    let st = take(&mut g, 1, C::Stella);
    put_battle(&mut g, 1, st);
    let hand1 = g.ps[1].hand.len();
    let cad = take(&mut g, 0, C::Caddy);
    mark_summoned(&mut g, cad.uid);
    enter(&mut g, 0, cad, How::Summon, None, 0);
    g.run();
    assert_eq!(g.ps[1].hand.len(), hand1 + 1, "ステラの持ち主が1枚引く");
    assert!(g.ps[0].battle[g.find(0, cad.uid).unwrap()].cant_attack);
    // 効果で出す（Put）は誘発しない
    let pp = take(&mut g, 0, C::Chakra);
    enter(&mut g, 0, pp, How::Put, None, 0);
    g.run();
    assert_eq!(g.ps[1].hand.len(), hand1 + 1);
    assert!(!g.ps[0].battle[g.find(0, pp.uid).unwrap()].cant_attack, "解決後に出たクリーチャーは攻撃できる（46963）");
}

#[test]
fn deaddam_invades_attacking_command_and_removes() {
    let mut g = blank("seitokai", "bike_meta", 0);
    let gy = take(&mut g, 0, C::Gyai); // 自然のコマンド
    let gu = put_battle(&mut g, 0, gy);
    let dd = take(&mut g, 0, C::Deaddam);
    g.ps[0].hand.push(dd);
    let cad = take(&mut g, 1, C::Chakra);
    let cu = put_battle(&mut g, 1, cad);
    for _ in 0..3 {
        let c = g.ps[1].deck.remove(0);
        g.ps[1].shields.push(Shield { card: c, faceup: false });
    }
    g.stack.push(Task::Attack { att: gu, tgt: Tgt::Player });
    g.run();
    while let Some(k) = k_of(&g) {
        match k {
            K::Invade => choose(&mut g, Opt::Yes),
            K::DeaddamPick => choose(&mut g, Opt::Pick(cu)),
            K::DeaddamDest => choose(&mut g, Opt::Mode(2)),
            _ => g.step(0),
        }
        if g.find(0, dd.uid).is_some() && g.find(1, cu).is_none() {
            break;
        }
    }
    let i = g.find(0, dd.uid).expect("デッドダムドが上に重なる");
    assert_eq!(g.ps[0].battle[i].under.len(), 1);
    assert!(g.ps[1].hand.iter().any(|c| c.uid == cu), "チャクラは手札に戻る");
    counts_ok(&g);
}

#[test]
fn seikimatsu_discards_and_casts_opponent_spell() {
    let mut g = blank("dogi4c", "bike_meta", 0);
    let se = take(&mut g, 0, C::Seikimatsu);
    // 相手の手札はキユリのASMラジオだけ（捨てさせると呪文）
    let ki = take(&mut g, 1, C::Kiyuri);
    g.ps[1].hand.push(ki);
    enter(&mut g, 0, se, How::Put, None, 0);
    g.run();
    assert_eq!(k_of(&g), Some(K::SeikiDiscard));
    let h = g.ps[0].hand[0].uid;
    choose(&mut g, Opt::Pick(h));
    assert!(g.ps[1].grave.iter().any(|c| c.uid == ki.uid));
    // 次に捨てるか（Pass）／相手の呪文を唱えるか
    let mut cast = false;
    while let Some(k) = k_of(&g) {
        match k {
            K::SeikiCast => {
                choose(&mut g, Opt::Mode(0));
                cast = true;
            }
            K::SeikiDiscard => choose(&mut g, Opt::Pass),
            _ => g.step(0),
        }
    }
    assert!(cast, "相手が捨てた呪文を唱えられる");
    assert!(g.ps[1].grave.iter().any(|c| c.uid == ki.uid), "唱えた呪文は持ち主の墓地へ");
    counts_ok(&g);
}

#[test]
fn sixfall_puts_creature_under_gorgi() {
    let mut g = blank("gorgi_trv", "bike_meta", 0);
    let gor = take(&mut g, 0, C::Gorgi);
    let gu = put_battle(&mut g, 0, gor);
    let ak = take(&mut g, 0, C::Akane);
    let au = put_battle(&mut g, 0, ak);
    let six = take(&mut g, 0, C::SixFall);
    enter(&mut g, 0, six, How::Put, None, 0);
    g.run();
    // 相手側（クリーチャーなし）→ 自分側
    while let Some(k) = k_of(&g) {
        match k {
            K::SixUnder => {
                let d = g.decision.as_ref().unwrap();
                if d.opts.contains(&Opt::Pick(au)) { choose(&mut g, Opt::Pick(au)) } else { g.step(0) }
            }
            K::SixTop => choose(&mut g, Opt::Pick(gu)),
            _ => g.step(0),
        }
    }
    let i = g.find(0, gu).unwrap();
    assert_eq!(g.ps[0].battle[i].under.len(), 1, "アカネがゴルギーの下に入る");
    assert!(g.find(0, au).is_none());
    counts_ok(&g);
}

#[test]
fn paradiso_returns_two_water() {
    let mut g = blank("gorgi_trv", "bike_meta", 0);
    let kp = take(&mut g, 0, C::Konpeki);
    let ku = put_battle(&mut g, 0, kp);
    let pa = take(&mut g, 0, C::Paradiso);
    enter(&mut g, 0, pa, How::Put, Some(ku), 0);
    g.run();
    // 水のクリーチャーは自分だけ（紺碧は下）→ 自身を戻す（31933）。候補が1つなので自動
    while k_of(&g).is_some() {
        g.step(0);
    }
    assert!(g.ps[0].hand.iter().any(|c| c.uid == pa.uid), "{:?} {:?}", g.ps[0].battle, g.decision);
    counts_ok(&g);
}

#[test]
fn sekkei_takes_neo_and_fusioner() {
    let mut g = blank("gorgi_trv", "bike_meta", 0);
    // 山札の上6枚：ゴルギー・シックスフォール＋他4枚
    let a = take(&mut g, 0, C::Gorgi);
    let b = take(&mut g, 0, C::SixFall);
    g.ps[0].deck.push(a);
    g.ps[0].deck.push(b);
    let sk = take(&mut g, 0, C::Sekkei);
    for c in [C::Caddy, C::Caddy] {
        mana_of(&mut g, 0, c);
    }
    g.ps[0].hand.push(sk);
    play_from_hand(&mut g, 0, sk.uid, 0, 0);
    g.run();
    assert_eq!(k_of(&g), Some(K::SekkeiNeo));
    choose(&mut g, Opt::Pick(a.uid));
    assert_eq!(k_of(&g), Some(K::SekkeiFus));
    choose(&mut g, Opt::Pick(b.uid));
    assert!(g.ps[0].hand.iter().any(|c| c.uid == a.uid));
    assert!(g.ps[0].hand.iter().any(|c| c.uid == b.uid));
    counts_ok(&g);
}

#[test]
fn jolagon_neo_cost_and_put() {
    let mut g = blank("jokers_meta", "zaze", 0);
    let od = take(&mut g, 0, C::Odotta);
    let ou = put_battle(&mut g, 0, od);
    assert_eq!(play_cost_b(&g, 0, C::Jolagon, 0, 0, Some(ou)).0, 5, "NEO進化なら 7-2");
    assert_eq!(play_cost_b(&g, 0, C::Jolagon, 0, 0, None).0, 7);
    let jo = take(&mut g, 0, C::Jolagon);
    let fu = take(&mut g, 0, C::Fujisan);
    g.ps[0].hand.push(fu);
    enter(&mut g, 0, jo, How::Put, Some(ou), 0);
    g.run();
    // 選べるのは「手札から出す」だけで、出せるのも富士山だけなので自動で決まる
    while k_of(&g).is_some() {
        g.step(0);
    }
    assert!(g.find(0, fu.uid).is_some(), "コスト5の富士山を出す");
}

// ---- 第2段階：デアリデモニオ ----

/// 決定を最初の選択肢で進め、積まれた処理がなくなった時の引き分け扱いを戻す
fn settle(g: &mut Game) {
    g.run();
    while g.decision.is_some() {
        g.step(0);
    }
    if g.winner == Some(2) && g.stack.is_empty() {
        g.winner = None;
    }
}


fn shields(g: &mut Game, p: usize, n: usize) {
    for _ in 0..n {
        let c = g.ps[p].deck.remove(0);
        g.ps[p].shields.push(Shield { card: c, faceup: false });
    }
}

#[test]
fn greatest_shield_cost_and_hand() {
    let mut g = blank("demonio", "bike_meta", 0);
    shields(&mut g, 0, 3);
    assert_eq!(play_cost(&g, 0, C::Greatest, 0, 32).0, 3, "シールド2つで 5-2");
    let he = take(&mut g, 0, C::Helcop);
    g.ps[0].tama.push(he);
    let gr = take(&mut g, 0, C::Greatest);
    g.ps[0].pend_sh = 2;
    let hand = g.ps[0].hand.len();
    enter(&mut g, 0, gr, How::Put, Some(he.uid), 0);
    settle(&mut g);
    assert_eq!(g.ps[0].shields.len(), 1);
    assert_eq!(g.ps[0].hand.len(), hand + 2, "選んだシールド2つを手札へ（S・トリガーなし）");
    let i = g.find(0, gr.uid).unwrap();
    assert_eq!(g.ps[0].battle[i].under.len(), 1, "ヘルコプ太の上に進化");
    counts_ok(&g);
}

#[test]
fn star_evolution_rebuilds_on_destroy() {
    // スター進化：離れる時、かわりに一番上だけ離れ、下のタマシードはタマシードとして残る
    let mut g = blank("demonio", "bike_meta", 1);
    let he = take(&mut g, 0, C::Helcop);
    g.ps[0].tama.push(he);
    let sh = take(&mut g, 0, C::Shurai);
    enter(&mut g, 0, sh, How::Put, Some(he.uid), 0);
    settle(&mut g);
    assert!(g.ps[0].tama.iter().all(|t| t.uid != he.uid));
    g.stack.push(Task::Destroy { p: 0, uid: sh.uid, by_opp: true });
    settle(&mut g);
    assert!(g.ps[0].grave.iter().any(|c| c.uid == sh.uid), "終来王鬼だけが墓地へ");
    assert!(g.ps[0].tama.iter().any(|t| t.uid == he.uid), "ヘルコプ太はタマシードに戻る");
    counts_ok(&g);
}

#[test]
fn cry_replaces_losing_with_shield() {
    let mut g = blank("bike_meta", "demonio", 0);
    let cr = take(&mut g, 1, C::Cry);
    put_battle(&mut g, 1, cr);
    shields(&mut g, 1, 1);
    let cad = take(&mut g, 0, C::Chakra);
    let cu = put_battle(&mut g, 0, cad);
    // シールドが1つ残った状態でとどめをさされる
    g.stack.push(Task::DirectHit { winner: 0 });
    settle(&mut g);
    assert!(g.winner.is_none(), "CRY-S-MAX：負けるかわりにシールドを墓地へ");
    assert!(g.ps[1].shields.is_empty());
    let _ = cu;
    counts_ok(&g);
}

#[test]
fn dmc_does_not_leave_with_few_under() {
    let mut g = blank("demonio", "bike_meta", 1);
    let dm = take(&mut g, 0, C::Dmc);
    let du = put_battle(&mut g, 0, dm);
    g.stack.push(Task::Destroy { p: 0, uid: du, by_opp: true });
    settle(&mut g);
    assert!(g.find(0, du).is_some(), "下のカードが7枚以下なら離れない");
    // パワーを0以下にしても、ルール処理で破壊を繰り返さない
    let i = g.find(0, du).unwrap();
    g.ps[0].battle[i].power_mod = -99999;
    g.stack.push(Task::MainStep);
    g.run();
    assert!(g.winner.is_none() && g.steps < 1000, "破壊の繰り返しにならない");
    assert!(g.find(0, du).is_some());
    counts_ok(&g);
}

#[test]
fn bakuro_shield_to_mana_and_put_demonio() {
    let mut g = blank("demonio", "bike_meta", 0);
    shields(&mut g, 0, 2);
    for c in [C::Helcop, C::Helcop, C::Helcop] { mana_of(&mut g, 0, c); }
    let gr = take(&mut g, 0, C::Greatest);
    g.ps[0].mana.push(Mana { card: gr, tapped: false, facedown: false });
    let ba = take(&mut g, 0, C::Bakuro);
    g.ps[0].hand.push(ba);
    play_from_hand(&mut g, 0, ba.uid, 0, 0);
    g.run();
    while let Some(k) = k_of(&g) {
        match k {
            K::BakuroPut => choose(&mut g, Opt::Pick(gr.uid)),
            _ => g.step(0),
        }
    }
    g.winner = None;
    assert_eq!(g.ps[0].shields.len(), 1, "シールド1つをマナへ");
    let i = g.find(0, gr.uid).expect("マナ5枚以下のグレイテストを出す");
    assert_eq!(g.ps[0].battle[i].under[0].uid, ba.uid, "バクロ法師の封の上に");
    counts_ok(&g);
}

#[test]
fn deathlost_modes_not_repeated() {
    // 47462：選んだモードは記録され、次のターンには選べない
    let mut g = blank("demonio", "bike_meta", 0);
    let dl = take(&mut g, 0, C::DeathLost);
    g.ps[0].tama.push(dl);
    shields(&mut g, 1, 2);
    g.stack.push(Task::EndTriggers);
    g.run();
    assert_eq!(k_of(&g), Some(K::LostMode));
    choose(&mut g, Opt::Mode(0));
    while k_of(&g) == Some(K::LostShield) { g.step(0); }
    g.winner = None;
    assert_eq!(g.ps[1].shields.len(), 1);
    g.stack.clear();
    g.decision = None;
    g.winner = None;
    g.stack.push(Task::EndTriggers);
    g.run();
    let d = g.decision.as_ref().unwrap();
    assert!(!d.opts.contains(&Opt::Mode(0)), "{:?}", d.opts);
    counts_ok(&g);
}

// ---- 第2段階：創世竜 ----

#[test]
fn drache_executes_tama_from_mana_with_reduction() {
    let mut g = blank("sousei", "bike_meta", 0);
    let dr = take(&mut g, 0, C::Drache);
    put_battle(&mut g, 0, dr);
    assert_eq!(play_cost(&g, 0, C::Levya, 0, 0).0, 3, "タマシードは 5-2");
    assert_eq!(play_cost(&g, 0, C::Jasmine, 0, 0).0, 1, "0以下にはならない");
    for c in [C::Akane, C::Akane, C::Chiryu] { mana_of(&mut g, 0, c); }
    let pe = take(&mut g, 0, C::Pearly);
    g.ps[0].mana.push(Mana { card: pe, tapped: false, facedown: false });
    let mut o = vec![];
    main_options(&g, &mut o);
    assert!(o.contains(&Opt::Play { uid: pe.uid, side: 0, how: 7 }), "マナのパーリ騎士を実行できる {:?} {:?}", o, g.ps[0].mana);
    let mana0 = g.ps[0].mana.len();
    play_from_hand(&mut g, 0, pe.uid, 0, 7);
    g.run();
    while let Some(k) = k_of(&g) {
        match k {
            K::DracheUnder => choose(&mut g, Opt::Pick(dr.uid)),
            _ => g.step(0),
        }
    }
    g.winner = None;
    let i = g.find(0, dr.uid).unwrap();
    assert!(g.ps[0].battle[i].under.iter().any(|c| c.uid == pe.uid), "出たタマシードを創世竜の下に置ける");
    assert_eq!(g.ps[0].mana.len(), mana0 + 1, "マナから1枚出て、山札から1枚、パーリのシンカパワー（下に置いた時もトリガー 48122）で2枚置いて1枚戻す");
    assert_eq!(g.ps[0].drache_done, vec![dr.uid]);
    let mut o = vec![];
    main_options(&g, &mut o);
    assert!(!o.iter().any(|x| matches!(x, Opt::Play { how: 7, .. })), "創世竜1体につき1回");
    counts_ok(&g);
}

#[test]
fn trap_sends_cost4_to_owner_mana() {
    let mut g = blank("sousei", "bike_meta", 0);
    let cad = take(&mut g, 1, C::Caddy);
    let cu = put_battle(&mut g, 1, cad);
    for c in [C::Akane, C::Akane, C::Akane] { mana_of(&mut g, 0, c); }
    let tr = take(&mut g, 0, C::Trap);
    g.ps[0].hand.push(tr);
    play_from_hand(&mut g, 0, tr.uid, 0, 0);
    g.run();
    while let Some(k) = k_of(&g) {
        match k {
            K::TrapPick => choose(&mut g, Opt::Pick(cu)),
            _ => g.step(0),
        }
    }
    assert!(g.ps[1].mana.iter().any(|m| m.card.uid == cu), "キャディは持ち主のマナへ {:?} {:?} {:?}", g.ps[1].battle, g.ps[0].tama, g.decision);
    counts_ok(&g);
}

#[test]
fn levya_sinka_power_on_evolve() {
    let mut g = blank("sousei", "bike_meta", 0);
    let lv = take(&mut g, 0, C::Levya);
    g.ps[0].tama.push(lv);
    let ak = take(&mut g, 0, C::Akane);
    g.ps[0].mana.push(Mana { card: ak, tapped: false, facedown: false });
    let dr = take(&mut g, 0, C::Drache);
    enter(&mut g, 0, dr, How::Put, Some(lv.uid), 0);
    g.run();
    let mut put = false;
    while let Some(k) = k_of(&g) {
        match k {
            K::LevyaPut => { choose(&mut g, Opt::Pick(ak.uid)); put = true; }
            _ => g.step(0),
        }
    }
    assert!(put || g.find(0, ak.uid).is_some(), "シンカパワーでマナからコスト3以下");
    assert!(g.find(0, ak.uid).is_some());
    counts_ok(&g);
}

#[test]
fn valhalla_prevents_deckout_and_alspa_locks_cost() {
    let mut g = blank("sousei", "bike_meta", 0);
    let va = take(&mut g, 0, C::Valhalla);
    g.ps[0].tama.push(va);
    let rest: Vec<Card> = g.ps[0].deck.drain(..).collect();
    g.ps[0].grave.extend(rest);
    g.check_state();
    assert!(g.winner.is_none(), "山札が0枚でも負けない");
    // ♪立ち上がる：コスト2を選ぶと相手のコスト2の呪文を唱えられず、コスト2のクリーチャーは無視
    let mut g = blank("sousei", "bike_meta", 0);
    let cot = take(&mut g, 1, C::Cotton);
    let cu = put_battle(&mut g, 1, cot);
    let al = take(&mut g, 0, C::Alspa);
    g.ps[0].hand.push(al);
    for c in [C::Akane, C::Akane, C::Chiryu, C::Chiryu] { mana_of(&mut g, 0, c); }
    play_from_hand(&mut g, 0, al.uid, 0, 0);
    g.run();
    assert_eq!(k_of(&g), Some(K::AlspaNum));
    let k = cost_of(C::Cotton);
    choose(&mut g, Opt::Mode(k));
    assert_eq!(g.ps[1].alspa, k);
    assert!(g.ps[1].battle[g.find(1, cu).unwrap()].ign(), "同じコストのクリーチャーの能力を無視");
}

#[test]
fn sousei_mana_boost_loop_from_note() {
    // JO「創世竜ループ一覧」のマナブーストループ：場 創世竜／手 パーリ／マナ パーリ パーリ レヴィヤ トラップ → 1周でマナ+4
    let mut g = blank("sousei", "bike_meta", 0);
    let dr = take(&mut g, 0, C::Drache);
    put_battle(&mut g, 0, dr);
    let pa = take(&mut g, 0, C::Pearly);
    g.ps[0].hand.push(pa);
    let pb = take(&mut g, 0, C::Pearly);
    let pc = take(&mut g, 0, C::Pearly);
    let lv = take(&mut g, 0, C::Levya);
    let tr = take(&mut g, 0, C::Trap);
    let ak = take(&mut g, 0, C::Akane); // 近似：支払うマナは自動で選ぶので1枚余分に置く
    for c in [pb, pc, lv, tr, ak] {
        g.ps[0].mana.push(Mana { card: c, tapped: false, facedown: false });
    }
    let mana0 = g.ps[0].mana.len();
    let mut returns = 0;
    let mut levya = 0;
    let mut drive = |g: &mut Game, returns: &mut i32, levya: &mut i32| {
        while let Some(k) = k_of(g) {
            match k {
                K::DracheUnder | K::TrapPick => choose(g, Opt::Pick(dr.uid)),
                K::PearlyReturn => {
                    *returns += 1;
                    let target = if *returns == 1 { lv.uid } else { pb.uid };
                    choose(g, Opt::Pick(target));
                }
                K::PearlyPlayer => choose(g, Opt::Mode(0)),
                K::LevyaPut => {
                    *levya += 1;
                    choose(g, Opt::Pick(if *levya == 1 { pc.uid } else { tr.uid }));
                }
                _ => g.step(0),
            }
        }
        g.winner = None;
    };
    play_from_hand(&mut g, 0, pa.uid, 0, 0);
    g.run();
    drive(&mut g, &mut returns, &mut levya);
    assert!(g.ps[0].hand.iter().any(|c| c.uid == lv.uid), "パーリのシンカパワーでレヴィヤを回収");
    play_from_hand(&mut g, 0, lv.uid, 0, 0);
    g.run();
    drive(&mut g, &mut returns, &mut levya);
    let i = g.find(0, dr.uid).expect("創世竜は残る（G-NEO：かわりに下のカードがマナへ）");
    assert!(g.ps[0].battle[i].under.is_empty());
    assert!(g.ps[0].hand.iter().any(|c| c.uid == pb.uid), "タップしているパーリを回収して初期盤面へ");
    assert_eq!(g.ps[0].mana.len(), mana0 + 4, "1周でマナが4枚増える");
    for u in [pa.uid, pc.uid, lv.uid, tr.uid] {
        assert!(g.ps[0].mana.iter().any(|m| m.card.uid == u));
    }
    counts_ok(&g);
}

// ---- 第3段階：4Cギリガザミ ----

#[test]
fn petens_breaks_own_shields_and_giri_gets_xb() {
    // ペテンズ・ゲートで自分のシールドをすべてブレイク → シールド0でギリガザミをスーパー・S・トリガー → xB で手札の呪文
    let mut g = blank("giri4c", "bike_meta", 0);
    let cad = take(&mut g, 1, C::Caddy);
    let cu = put_battle(&mut g, 1, cad);
    let gi = take(&mut g, 0, C::Giri);
    g.ps[0].shields.push(Shield { card: gi, faceup: false });
    let kk = take(&mut g, 0, C::Kamigami);
    g.ps[0].hand.push(kk);
    let pt = take(&mut g, 0, C::Petens);
    g.ps[0].hand.push(pt);
    for _ in 0..8 { let c = g.ps[0].deck.remove(0); g.ps[0].mana.push(Mana { card: c, tapped: false, facedown: false }); }
    // 色：光・水・闇が必要なので文明をそろえたカードを足す
    for c in [C::Shinkirou, C::Shinkirou] { mana_of(&mut g, 0, c); }
    play_from_hand(&mut g, 0, pt.uid, 0, 0);
    g.run();
    let mut xb_cast = false;
    while let Some(d) = g.decision.as_ref() {
        match &d.q {
            Q::UseTrigger { uid, .. } if *uid == gi.uid => { let i = d.opts.iter().position(|o| *o == Opt::Yes).unwrap_or(0); g.step(i); }
            Q::Custom(e) if e.k == K::GiriCast => { xb_cast = true; choose(&mut g, Opt::Pick(kk.uid)); }
            Q::Custom(e) if e.k == K::GiriBounce => choose(&mut g, Opt::Pick(cu)),
            _ => g.step(0),
        }
    }
    g.winner = None;
    assert!(g.ps[0].shields.is_empty());
    assert!(g.ps[1].battle.iter().all(|x| x.card.uid != cu), "キャディを手札へ");
    assert!(xb_cast, "シールド0でトリガーしたので xB が使える");
    assert!(g.ps[0].mana.is_empty() && g.ps[1].mana.is_empty(), "神々の逆流で全員のマナが手札へ");
    counts_ok(&g);
}

#[test]
fn shinkirou_executes_trigger_from_grave_to_bottom() {
    let mut g = blank("giri4c", "bike_meta", 0);
    let kg = take(&mut g, 0, C::Kakugo);
    g.ps[0].grave.push(kg);
    let sh = take(&mut g, 0, C::Shinkirou);
    g.ps[0].hand.push(sh);
    let ak = take(&mut g, 0, C::Akashic);
    g.ps[0].tama.push(ak);
    let mut o = vec![];
    main_options(&g, &mut o);
    assert!(o.contains(&Opt::Play { uid: sh.uid, side: 0, how: 0 }), "アカシック3：マナが無くても呪文を唱えられる");
    play_from_hand(&mut g, 0, sh.uid, 0, 0);
    g.run();
    let mut n = 0;
    while let Some(k) = k_of(&g) {
        match k {
            K::ShinMode => { n += 1; choose(&mut g, Opt::Mode(if n == 1 { 1 } else { 0 })); }
            K::ShinTrig => choose(&mut g, Opt::Pick(kg.uid)),
            _ => g.step(0),
        }
    }
    g.winner = None;
    assert_eq!(g.ps[0].deck[0].uid, kg.uid, "墓地から唱えた覚悟の決闘は山札の下へ");
    counts_ok(&g);
}

#[test]
fn kurin_limits_opponent_to_two_creatures() {
    let mut g = blank("bike_meta", "giri4c", 0);
    let ku = take(&mut g, 1, C::Kurin);
    put_battle(&mut g, 1, ku);
    g.ps[0].entered_this_turn = 2;
    assert!(creature_blocked(&g, 0), "クリン・ラビィ：相手は各ターン2体まで");
    g.ps[0].entered_this_turn = 1;
    assert!(!creature_blocked(&g, 0));
}
