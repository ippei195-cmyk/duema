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
];
