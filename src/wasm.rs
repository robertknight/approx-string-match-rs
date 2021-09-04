// This module defines the WASM API for the library.

use crate::search_impl;
use crate::Match;

// Use `wee_alloc` as the global allocator to reduce library size.
extern crate wee_alloc;
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[no_mangle]
pub extern "C" fn match_vec_alloc() -> *mut Vec<Match> {
    let box_ = Box::new(Vec::new());
    Box::into_raw(box_)
}

#[no_mangle]
pub extern "C" fn match_vec_len(mv: &Vec<Match>) -> usize {
    mv.len()
}

#[no_mangle]
pub extern "C" fn match_vec_free(mv: *mut Vec<Match>) {
    unsafe { Box::from_raw(mv) };
}

#[no_mangle]
pub extern "C" fn match_vec_get(mv: &Vec<Match>, index: usize) -> &Match {
    &mv[index]
}

#[no_mangle]
pub extern "C" fn match_start(m: &Match) -> usize {
    m.start
}

#[no_mangle]
pub extern "C" fn match_end(m: &Match) -> usize {
    m.end
}

#[no_mangle]
pub extern "C" fn match_errors(m: &Match) -> usize {
    m.errors
}

#[no_mangle]
pub extern "C" fn char_buf_alloc(len: usize) -> *mut Vec<u16> {
    let box_ = Box::new(vec![0; len]);
    Box::into_raw(box_)
}

#[no_mangle]
pub extern "C" fn char_buf_data(buf: &mut Vec<u16>) -> *mut u16 {
    buf.as_mut_ptr()
}

#[no_mangle]
pub extern "C" fn char_buf_free(buf: *mut Vec<u16>) {
    unsafe { Box::from_raw(buf) };
}

#[no_mangle]
pub extern "C" fn search(
    match_vec: &mut Vec<Match>,
    text: &Vec<u16>,
    pat: &Vec<u16>,
    max_errors: u32,
) -> usize {
    let search_matches = search_impl(&text, &pat, max_errors);
    match_vec.clear();
    match_vec.extend_from_slice(&search_matches);
    match_vec.len()
}
