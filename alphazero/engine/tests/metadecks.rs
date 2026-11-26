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
