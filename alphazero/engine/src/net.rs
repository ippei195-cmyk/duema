//! 評価＋方策ネット（CPU で推論）。重みは train.py が書き出す .bin。
//! 局面 S → 256 → 256 = h。価値 = tanh(h·wv + bv)。選択肢 = [h, opt(O)] → 128 → 1。

use crate::features::{O, S};

pub const H: usize = 256;
pub const P: usize = 128;

pub struct Linear {
    pub w: Vec<f32>, // out × in（行優先）
    pub b: Vec<f32>,
    pub inp: usize,
    pub out: usize,
}

impl Linear {
    fn forward(&self, x: &[f32], y: &mut [f32], relu: bool) {
        for o in 0..self.out {
            let row = &self.w[o * self.inp..(o + 1) * self.inp];
            let mut s = self.b[o];
            for i in 0..self.inp {
                s += row[i] * x[i];
            }
            y[o] = if relu && s < 0.0 { 0.0 } else { s };
        }
    }
}

pub struct Net {
    pub l1: Linear,
    pub l2: Linear,
    pub v: Linear,
    pub p1: Linear,
    pub p2: Linear,
}

const MAGIC: u32 = 0x444d_4e31; // "DMN1"

impl Net {
    pub fn load(path: &str) -> std::io::Result<Net> {
        let b = std::fs::read(path)?;
        let u = |i: usize| u32::from_le_bytes(b[i * 4..i * 4 + 4].try_into().unwrap());
        assert_eq!(u(0), MAGIC, "bad weights file");
        assert_eq!((u(1) as usize, u(2) as usize, u(3) as usize, u(4) as usize), (S, O, H, P), "dims");
        let mut pos = 5 * 4;
        let mut take = |inp: usize, out: usize| {
            let n = inp * out + out;
            let v: Vec<f32> = (0..n).map(|k| f32::from_le_bytes(b[pos + k * 4..pos + k * 4 + 4].try_into().unwrap())).collect();
            pos += n * 4;
            Linear { w: v[..inp * out].to_vec(), b: v[inp * out..].to_vec(), inp, out }
        };
        let l1 = take(S, H);
        let l2 = take(H, H);
        let v = take(H, 1);
        let p1 = take(H + O, P);
        let p2 = take(P, 1);
        Ok(Net { l1, l2, v, p1, p2 })
    }

    /// (価値 [-1,1]（判断するプレイヤーから見て）, 各選択肢のロジット)
    pub fn eval(&self, state: &[f32], opts: &[Vec<f32>]) -> (f32, Vec<f32>) {
        let mut h1 = vec![0f32; H];
        let mut h = vec![0f32; H];
        // 局面の特徴はほとんど 0 なので、0 でない所だけ足す
        let nz: Vec<usize> = (0..S).filter(|&i| state[i] != 0.0).collect();
        for o in 0..H {
            let row = &self.l1.w[o * S..(o + 1) * S];
            let s: f32 = self.l1.b[o] + nz.iter().map(|&i| row[i] * state[i]).sum::<f32>();
            h1[o] = s.max(0.0);
        }
        self.l2.forward(&h1, &mut h, true);
        let mut v = [0f32];
        self.v.forward(&h, &mut v, false);
        // 選択肢側：h の部分は全選択肢で共通なので先に計算
        let mut hp = vec![0f32; P];
        for o in 0..P {
            let row = &self.p1.w[o * (H + O)..o * (H + O) + H];
            hp[o] = self.p1.b[o] + row.iter().zip(&h).map(|(a, b)| a * b).sum::<f32>();
        }
        let mut logits = Vec::with_capacity(opts.len());
        let mut z = vec![0f32; P];
        for of in opts {
            for o in 0..P {
                let row = &self.p1.w[o * (H + O) + H..(o + 1) * (H + O)];
                let mut s = hp[o];
                for (k, &x) in of.iter().enumerate() {
                    if x != 0.0 {
                        s += row[k] * x;
                    }
                }
                z[o] = s.max(0.0);
            }
            let mut l = [0f32];
            self.p2.forward(&z, &mut l, false);
            logits.push(l[0]);
        }
        (v[0].tanh(), logits)
    }
}
