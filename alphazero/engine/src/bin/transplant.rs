//! 5デッキ（58種類）で学習した重み（S=680, O=176）を、環境デッキ入り15デッキ（120種類）の入力に移す（train5 → train6）。
//! 前からあった特徴の重みはそのまま写し、新しいカード・効果の質問の特徴は 0 から学ぶ。
//! カードの種類（C）と効果の質問（K）は前からのものの番号が変わっていない（新しいものは後ろに追加）ことを確認済み。
//! 使い方: transplant <旧.bin> <新.bin>
use dmengine::features::*;
use dmengine::net::{H, P};

const ONC: usize = 58; // 旧カード種類
const OKN: usize = 100; // 旧 KN
const OS_: usize = 680;
const OO_: usize = 176;
const OK_: usize = 73; // 旧 K の数（ChooseRepl を除く）
// 旧の選択肢の位置
const O_OC: usize = 10;
const O_OZ: usize = O_OC + ONC + 1;
const O_OT: usize = O_OZ + 12 + 3;
const O_OTP: usize = O_OT + ONC + 1;

fn map_state(i: usize) -> Option<usize> {
    let nc = dmengine::cards::NCARDS;
    if i < 9 * ONC {
        return Some((i / ONC) * nc + i % ONC);
    }
    if i < 9 * ONC + 40 {
        return Some(9 * nc + (i - 9 * ONC));
    }
    let j = i - (9 * ONC + 40);
    match j {
        0..=7 => Some(DBASE + j),
        _ if j >= 8 && j < 8 + OKN => (j - 8 < OK_).then(|| DBASE + 8 + (j - 8)),
        _ if j >= 8 + OKN && j < 8 + OKN + 7 => Some(DBASE + 8 + KN + (j - 8 - OKN)),
        _ => None,
    }
}

fn map_opt(i: usize) -> Option<usize> {
    let nc = dmengine::cards::NCARDS;
    let card = |c: usize| if c == ONC { nc } else { c };
    if i < O_OC {
        return Some(i);
    }
    if i < O_OZ {
        return Some(OC + card(i - O_OC));
    }
    if i < O_OT {
        return Some(OZ + (i - O_OZ)); // ゾーン12・タップ/パワー2・プレイヤー攻撃1 は並びが同じ
    }
    if i < O_OTP {
        return Some(OT + card(i - O_OT));
    }
    if i < O_OTP + 2 + 1 + 8 + 1 + 16 {
        return Some(OTP + (i - O_OTP)); // OTP2・面1・出し方8・GC1・モード16
    }
    None
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let b = std::fs::read(&a[1]).unwrap();
    let u = |i: usize| u32::from_le_bytes(b[i * 4..i * 4 + 4].try_into().unwrap());
    assert_eq!(u(0), 0x444d_4e31);
    assert_eq!((u(1) as usize, u(2) as usize, u(3) as usize, u(4) as usize), (OS_, OO_, H, P), "旧の大きさが違う");
    assert_eq!(OS_, 680);
    let f: Vec<f32> = (5..b.len() / 4).map(|i| f32::from_le_bytes(b[i * 4..i * 4 + 4].try_into().unwrap())).collect();
    let mut pos = 0;
    let mut take = |n: usize| {
        let v = f[pos..pos + n].to_vec();
        pos += n;
        v
    };
    let (l1w, l1b) = (take(OS_ * H), take(H));
    let (l2w, l2b) = (take(H * H), take(H));
    let (vw, vb) = (take(H), take(1));
    let (p1w, p1b) = (take((H + OO_) * P), take(P));
    let (p2w, p2b) = (take(P), take(1));
    assert_eq!(pos, f.len());
    let mut n1 = vec![0f32; S * H];
    for o in 0..H {
        for i in 0..OS_ {
            if let Some(j) = map_state(i) {
                n1[o * S + j] = l1w[o * OS_ + i];
            }
        }
    }
    let mut np = vec![0f32; (H + O) * P];
    for o in 0..P {
        for i in 0..H {
            np[o * (H + O) + i] = p1w[o * (H + OO_) + i];
        }
        for i in 0..OO_ {
            if let Some(j) = map_opt(i) {
                np[o * (H + O) + H + j] = p1w[o * (H + OO_) + H + i];
            }
        }
    }
    let mut out: Vec<u8> = vec![];
    for x in [0x444d_4e31u32, S as u32, O as u32, H as u32, P as u32] {
        out.extend(x.to_le_bytes());
    }
    for v in [&n1, &l1b, &l2w, &l2b, &vw, &vb, &np, &p1b, &p2w, &p2b] {
        for x in v.iter() {
            out.extend(x.to_le_bytes());
        }
    }
    std::fs::write(&a[2], &out).unwrap();
    let net = dmengine::net::Net::load(&a[2]).expect("新しい重みが読めない");
    let _ = net;
    // 確認：学習用5デッキの局面で、0 でない特徴がすべて旧の特徴から写されているか（＝5デッキでは旧の重みと同じ評価になる）
    let mut img_s = vec![false; S];
    (0..OS_).filter_map(map_state).for_each(|j| img_s[j] = true);
    let mut img_o = vec![false; O];
    (0..OO_).filter_map(map_opt).for_each(|j| img_o[j] = true);
    let (mut miss, mut tot) = (0u64, 0u64);
    for seed in 0..300u64 {
        let da = dmengine::cards::deck(dmengine::cards::DECKS[(seed % 5) as usize]);
        let db = dmengine::cards::deck(dmengine::cards::DECKS[(seed / 5 % 5) as usize]);
        let mut g = dmengine::engine::Game::new(&da, &db, seed);
        let mut r = dmengine::rng::Rng::new(seed);
        while g.winner.is_none() {
            let d = g.decision.clone().unwrap();
            let st = state(&g, d.player);
            for (j, &x) in st.iter().enumerate() { if x != 0.0 { tot += 1; if !img_s[j] { miss += 1; } } }
            for &o in &d.opts {
                for (j, &x) in option(&g, d.player, o).iter().enumerate() { if x != 0.0 { tot += 1; if !img_o[j] { miss += 1; } } }
            }
            g.step(r.below(d.opts.len()));
        }
    }
    println!("5デッキの局面：0でない特徴 {tot} のうち旧に無いもの {miss}");
    println!("ok: {} → {} ({} bytes)", a[1], a[2], out.len());
}
