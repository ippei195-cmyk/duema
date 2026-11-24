//! ブラウザ用（wasm32-unknown-unknown）。wasm-bindgen を使わず、メモリに数字の並びを置いて呼ぶ。
use crate::net::Net;
use std::sync::Mutex;

static NET: Mutex<Option<Net>> = Mutex::new(None);
static OUT: Mutex<Vec<i32>> = Mutex::new(Vec::new());

#[unsafe(no_mangle)]
pub extern "C" fn dmaz_alloc(n: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(n);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// 重み（DMN1）を読み込む。成功で 1
#[unsafe(no_mangle)]
pub extern "C" fn dmaz_load(ptr: *const u8, len: usize) -> i32 {
    let b = unsafe { std::slice::from_raw_parts(ptr, len) };
    match Net::from_bytes(b) {
        Some(n) => { *NET.lock().unwrap() = Some(n); 1 }
        None => 0,
    }
}

/// 盤面（i32 の並び）から次の一手。結果は dmaz_out() の場所に6個の i32
#[unsafe(no_mangle)]
pub extern "C" fn dmaz_decide(ptr: *const i32, len: usize) -> i32 {
    let inp = unsafe { std::slice::from_raw_parts(ptr, len) };
    let net = NET.lock().unwrap();
    let r = crate::bridge::decide(inp, net.as_ref());
    *OUT.lock().unwrap() = r;
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn dmaz_out() -> *const i32 {
    OUT.lock().unwrap().as_ptr()
}
