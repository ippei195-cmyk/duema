//! netcheck <重み.bin> <データ.bin>：最初の記録でネットの出力を表示（Python と一致するかの確認用）
use dmengine::features::{O, S};
use dmengine::net::Net;
fn rd(b: &[u8], pos: &mut usize, n: usize) -> Vec<f32> {
    let mut v = vec![0f32; n];
    let nz = u16::from_le_bytes([b[*pos], b[*pos + 1]]) as usize; *pos += 2;
    for _ in 0..nz {
        let i = u16::from_le_bytes([b[*pos], b[*pos + 1]]) as usize;
        v[i] = f32::from_le_bytes(b[*pos + 2..*pos + 6].try_into().unwrap()); *pos += 6;
    }
    v
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let net = Net::load(&a[1]).unwrap();
    let b = std::fs::read(&a[2]).unwrap();
    let mut pos = 4;
    let k = u16::from_le_bytes([b[pos], b[pos + 1]]) as usize; pos += 6;
    let st = rd(&b, &mut pos, S);
    let mut ops = vec![];
    for _ in 0..k { pos += 4; ops.push(rd(&b, &mut pos, O)); }
    let (v, l) = net.eval(&st, &ops);
    println!("{v:.5} {:?}", l.iter().map(|x| format!("{x:.5}")).collect::<Vec<_>>());
}
