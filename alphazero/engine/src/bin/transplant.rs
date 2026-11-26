//! 3デッキ（40種類）で学習した重み（S=512, O=136）を、5デッキ（58種類）の入力（S=680, O=176）に移す。
//! 前からあった特徴の重みはそのまま写し、新しいカードの特徴は 0 から学ぶ。
//! 使い方: transplant <旧.bin> <新.bin>
use dmengine::features::*;
use dmengine::net::{H, P};

const OS_: usize = 512;
const OO_: usize = 136;
const ONC: usize = 40;
const OK_: usize = 61; // 旧 K の数（ChooseRepl を除く）

fn map_state(i: usize) -> Option<usize> {
    let nc = dmengine::cards::NCARDS;
    if i < 9 * ONC {
        return Some((i / ONC) * nc + i % ONC);
    }
    if i < 9 * ONC + 40 {
        return Some(9 * nc + (i - 9 * ONC));
    }
    let ob = 9 * ONC + 40; // 400
    let j = i - ob;
    match j {
        0..=7 => Some(DBASE + j),
        8..=88 => (j - 8 < OK_).then(|| DBASE + 8 + (j - 8)),
        90 | 91 => Some(DBASE + 8 + KN + (j - 90)),
        92..=96 => Some(DBASE + 10 + KN + (j - 92)),
        _ => None,
    }
}

fn map_opt(i: usize) -> Option<usize> {
    let nc = dmengine::cards::NCARDS;
    let card = |c: usize| if c == ONC { nc } else { c };
    match i {
        0..=9 => Some(i),
        10..=50 => Some(OC + card(i - 10)),
        51..=62 => Some(OZ + (i - 51)),
        63 | 64 => Some(OP + (i - 63)),
        65 => Some(OPL),
        66..=106 => Some(OT + card(i - 66)),
        107 | 108 => Some(OTP + (i - 107)),
        109 => Some(OS),
        110..=113 => Some(OH + (i - 110)),
        114..=129 => Some(OM + (i - 114)),
        _ => None,
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let b = std::fs::read(&a[1]).unwrap();
    let u = |i: usize| u32::from_le_bytes(b[i * 4..i * 4 + 4].try_into().unwrap());
    assert_eq!(u(0), 0x444d_4e31);
    assert_eq!((u(1) as usize, u(2) as usize, u(3) as usize, u(4) as usize), (OS_, OO_, H, P), "旧の大きさが違う");
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
    println!("ok: {} → {} ({} bytes)", a[1], a[2], out.len());
}
