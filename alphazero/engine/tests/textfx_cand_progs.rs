// textfx/compile.py が作るファイル（手で書き換えない）。効果文から作った手順の一覧。
#[allow(unused_imports)]
use dmengine::cards::*;
use dmengine::textfx::*;

#[allow(unused_imports)]
use dmengine::textfx::Op::*;

pub static PROGS: &[(C, Ev, &[Op])] = &[
    // キユリのASMラジオ spell
    (C::Kiyuri, Ev::Spell(0), &[Look { who: Who::Me, n: 5, var: 0 }, Choose { from: Src::Var(0), f: Filt { kinds: 1, max_cost: 3, ..ANY }, min: 0, max: 2, var: 1, by: Who::Me }, Move { var: 0, except: 1, to: To::DeckBottom }, Move { var: 1, except: 255, to: To::Battle }, End]),
    // ピザスターのアンティハムト etb
    (C::Antiham, Ev::Etb, &[Draw { who: Who::Me, n: 1 }, Choose { from: Src::Zone(Who::Me, Zone::Hand), f: Filt { kinds: 1, max_cost: 3, non_evo: true, ..ANY }, min: 0, max: 1, var: 0, by: Who::Me }, Move { var: 0, except: 255, to: To::Battle }, Power { var: 0, delta: 3000 }, End]),
    // 大集合！アカネ&アサギ&コハク etb
    (C::Akane, Ev::Etb, &[Look { who: Who::Me, n: 2, var: 0 }, Choose { from: Src::Var(0), f: ANY, min: 1, max: 1, var: 1, by: Who::Me }, Mode { targets: &[3, 5] }, Move { var: 1, except: 255, to: To::Hand }, Jump(7), Move { var: 1, except: 255, to: To::Mana }, Jump(7), Move { var: 0, except: 1, to: To::DeckBottom }, End]),
    // 世界のY チャクラ・デル・フィン etb
    (C::Chakra, Ev::Etb, &[TopToShield { who: Who::Me, n: 1 }, End]),
    // 轟腕のR ダグラジャパニカン etb
    (C::Dagura, Ev::Etb, &[Choose { from: Src::Zone(Who::Me, Zone::Shield), f: ANY, min: 1, max: 1, var: 0, by: Who::Me }, Move { var: 0, except: 255, to: To::Mana }, End]),
    // 流星のガイアッシュ・カイザー etb
    (C::Gaiash, Ev::Etb, &[Draw { who: Who::Me, n: 2 }, End]),
    // 悪魔龍 ダークマスターズ etb
    (C::DarkMasters, Ev::Etb, &[Choose { from: Src::Zone(Who::Opp, Zone::Hand), f: ANY, min: 0, max: 3, var: 0, by: Who::Me }, Move { var: 0, except: 255, to: To::Grave }, End]),
    // 切札混成 光隼 蒼斬丸 etb
    (C::Sozanmaru, Ev::Etb, &[Mode { targets: &[1, 2, 4] }, Jump(6), Draw { who: Who::Me, n: 1 }, Jump(6), Draw { who: Who::Me, n: 2 }, Jump(6), End]),
    // 水上第九院 シャコガイル etb
    (C::Shacogail, Ev::Etb, &[Filter { from: Src::Zone(Who::Me, Zone::Grave), f: ANY, var: 0 }, Move { var: 0, except: 255, to: To::DeckBottom }, End]),
    // ジョジョジョ・ジョーカーズ spell
    (C::JoJoJo, Ev::Spell(0), &[Look { who: Who::Me, n: 4, var: 0 }, Choose { from: Src::Var(0), f: Filt { kinds: 1, race: R_JOKERS, ..ANY }, min: 0, max: 1, var: 1, by: Who::Me }, Move { var: 1, except: 255, to: To::Hand }, Move { var: 0, except: 1, to: To::DeckBottom }, End]),
    // ジョリー・ザ・スクラッパー spell
    (C::Magny, Ev::Spell(1), &[Choose { from: Src::Zone(Who::Opp, Zone::Battle), f: Filt { max_cost: 3, ..ANY }, min: 1, max: 1, var: 0, by: Who::Me }, Destroy { var: 0 }, End]),
    // Dr.富士山ッピング etb
    (C::Fujisan, Ev::Etb, &[Look { who: Who::Me, n: 3, var: 0 }, Filter { from: Src::Var(0), f: Filt { race: R_JOKERS, ..ANY }, var: 1 }, Move { var: 1, except: 255, to: To::Hand }, Move { var: 0, except: 1, to: To::DeckBottom }, End]),
    // ジョギメラゴン＆メラビート etb
    (C::Jogimera, Ev::Etb, &[Look { who: Who::Me, n: 4, var: 0 }, Choose { from: Src::Var(0), f: Filt { race: R_JOKERS, ..ANY }, min: 0, max: 1, var: 1, by: Who::Me }, Move { var: 1, except: 255, to: To::Hand }, Move { var: 0, except: 1, to: To::DeckBottom }, End]),
    // 勝敵逆夢王 モウジャキンガ etb
    (C::Moujakinga, Ev::Etb, &[Choose { from: Src::Zone(Who::Opp, Zone::Shield), f: ANY, min: 1, max: 1, var: 0, by: Who::Me }, Choose { from: Src::Zone(Who::Opp, Zone::Battle), f: ANY, min: 1, max: 1, var: 1, by: Who::Me }, Random { from: Src::Zone(Who::Opp, Zone::Hand), f: ANY, n: 1, var: 2 }, Move { var: 0, except: 255, to: To::Grave }, Move { var: 1, except: 255, to: To::Grave }, Move { var: 2, except: 255, to: To::Grave }, End]),
    // サイレント・ジェラシー spell
    (C::Kernel, Ev::Spell(1), &[Random { from: Src::Zone(Who::Opp, Zone::Hand), f: ANY, n: 1, var: 0 }, Move { var: 0, except: 255, to: To::Grave }, Choose { from: Src::Zone(Who::Me, Zone::Shield), f: ANY, min: 1, max: 1, var: 1, by: Who::Me }, Move { var: 1, except: 255, to: To::Hand }, End]),
    // アシステスト・シネラリア etb
    (C::Cineraria, Ev::Etb, &[If { who: Who::Me, zone: Zone::Battle, f: Filt { race: R_GIANT, ..ANY }, cmp: Cmp::Ge, n: 2, els: 2 }, Draw { who: Who::Me, n: 1 }, End]),
];
