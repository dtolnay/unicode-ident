#![no_std]

#[rustfmt::skip]
mod tables;

use tables::{ASCII_CONTINUE, ASCII_START, CHUNK, LEAF, TRIE_CONTINUE, TRIE_START};

pub fn is_xid_start(ch: char) -> bool {
    if ch.is_ascii() {
        return ASCII_START.0[ch as usize];
    }
    let chunk = *TRIE_START.0.get(ch as usize / 8 / CHUNK).unwrap_or(&0);
    let offset = chunk as usize * CHUNK / 2 + ch as usize / 8 % CHUNK;
    unsafe { LEAF.0.get_unchecked(offset) }.wrapping_shr(ch as u32 % 8) & 1 != 0
}

pub fn is_xid_continue(ch: char) -> bool {
    if ch.is_ascii() {
        return ASCII_CONTINUE.0[ch as usize];
    }
    let chunk = *TRIE_CONTINUE.0.get(ch as usize / 8 / CHUNK).unwrap_or(&0);
    let offset = chunk as usize * CHUNK / 2 + ch as usize / 8 % CHUNK;
    unsafe { LEAF.0.get_unchecked(offset) }.wrapping_shr(ch as u32 % 8) & 1 != 0
}
