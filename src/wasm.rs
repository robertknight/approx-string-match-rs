// This module defines the WASM API for the library.

use std::alloc::{alloc, dealloc, Layout};

use crate::search_impl;
use crate::Match;

// Use `wee_alloc` as the global allocator to reduce library size.
extern crate wee_alloc;
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

static mut LAST_MATCHES: Vec<Match> = Vec::new();

#[no_mangle]
pub extern "C" fn match_start(index: usize) -> usize {
    unsafe { LAST_MATCHES[index].start }
}

#[no_mangle]
pub extern "C" fn match_end(index: usize) -> usize {
    unsafe { LAST_MATCHES[index].end }
}

#[no_mangle]
pub extern "C" fn match_errors(index: usize) -> usize {
    unsafe { LAST_MATCHES[index].errors }
}

#[no_mangle]
pub extern "C" fn clear_matches() {
    unsafe {
        LAST_MATCHES.clear();
    }
}

#[no_mangle]
pub extern "C" fn alloc_char_buffer(len: usize) -> *mut u16 {
    unsafe {
        let layout = Layout::from_size_align(len * 2, 4).unwrap();
        alloc(layout) as *mut u16
    }
}

#[no_mangle]
pub extern "C" fn free_char_buffer(ptr: *mut u16) {
    unsafe {
        dealloc(ptr as *mut u8, Layout::new::<u16>());
    }
}

#[no_mangle]
pub extern "C" fn search(
    text_buf: *mut u16,
    text_len: usize,
    pat_buf: *mut u16,
    pat_len: usize,
    max_errors: u32,
) -> usize {
    let text = unsafe { std::slice::from_raw_parts(text_buf, text_len) };
    let pat = unsafe { std::slice::from_raw_parts(pat_buf, pat_len) };

    let search_matches = search_impl(text, pat, max_errors);

    unsafe {
        LAST_MATCHES.clear();
        LAST_MATCHES.extend_from_slice(&search_matches);
        LAST_MATCHES.len()
    }
}
