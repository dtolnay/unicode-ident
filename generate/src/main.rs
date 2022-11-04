// To regenerate tables, run the following in the repo root:
//
// $ cargo install ucd-generate
// $ curl -LO https://www.unicode.org/Public/zipped/15.0.0/UCD.zip
// $ unzip UCD.zip -d UCD
// $ ucd-generate property-bool UCD --include XID_Start,XID_Continue > generate/src/ucd.rs
// $ ucd-generate property-bool UCD --include XID_Start,XID_Continue --fst-dir tests/fst
// $ ucd-generate property-bool UCD --include XID_Start,XID_Continue --trie-set > tests/trie/trie.rs
// $ cargo run --manifest-path generate/Cargo.toml

#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation, // https://github.com/rust-lang/rust-clippy/issues/9613
    clippy::match_wild_err_arm,
    clippy::too_many_lines,
    clippy::uninlined_format_args
)]

#[rustfmt::skip]
#[allow(dead_code, clippy::all, clippy::pedantic)]
mod ucd;

mod output;
mod write;

use std::cmp::Ordering;
use std::collections::{BTreeMap as Map, VecDeque};
use std::convert::TryFrom;
use std::fs;
use std::io;
use std::path::Path;

const CHUNK: usize = 64;
const PATH: &str = "../src/tables.rs";

fn is_xid_start(ch: char) -> bool {
    search(ch, ucd::XID_START)
}

fn is_xid_continue(ch: char) -> bool {
    search(ch, ucd::XID_CONTINUE)
}

fn search(ch: char, table: &[(u32, u32)]) -> bool {
    table
        .binary_search_by(|&(lo, hi)| {
            if lo > ch as u32 {
                Ordering::Greater
            } else if hi < ch as u32 {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        })
        .is_ok()
}

fn main() -> io::Result<()> {
    let mut chunkmap = Map::<[u8; CHUNK], u8>::new();
    let mut dense = Vec::<[u8; CHUNK]>::new();
    let mut new_chunk = |chunk| {
        if let Some(prev) = chunkmap.get(&chunk) {
            *prev
        } else {
            dense.push(chunk);
            let new = match u8::try_from(chunkmap.len()) {
                Ok(byte) => byte,
                Err(_) => panic!("exceeded 256 unique chunks"),
            };
            chunkmap.insert(chunk, new);
            new
        }
    };

    let empty_chunk = [0u8; CHUNK];
    new_chunk(empty_chunk);

    let mut index_start = Vec::<u8>::new();
    let mut index_continue = Vec::<u8>::new();
    for i in 0..(u32::from(char::MAX) + 1) / CHUNK as u32 / 8 {
        let mut start_bits = empty_chunk;
        let mut continue_bits = empty_chunk;
        for j in 0..CHUNK as u32 {
            let this_start = &mut start_bits[j as usize];
            let this_continue = &mut continue_bits[j as usize];
            for k in 0..8u32 {
                let code = (i * CHUNK as u32 + j) * 8 + k;
                if code >= 0x80 {
                    if let Some(ch) = char::from_u32(code) {
                        *this_start |= (is_xid_start(ch) as u8) << k;
                        *this_continue |= (is_xid_continue(ch) as u8) << k;
                    }
                }
            }
        }
        index_start.push(new_chunk(start_bits));
        index_continue.push(new_chunk(continue_bits));
    }

    while let Some(0) = index_start.last() {
        index_start.pop();
    }
    while let Some(0) = index_continue.last() {
        index_continue.pop();
    }

    let mut halfchunkmap = Map::new();
    for chunk in &dense {
        let mut front = [0u8; CHUNK / 2];
        let mut back = [0u8; CHUNK / 2];
        front.copy_from_slice(&chunk[..CHUNK / 2]);
        back.copy_from_slice(&chunk[CHUNK / 2..]);
        halfchunkmap
            .entry(front)
            .or_insert_with(VecDeque::new)
            .push_back(back);
    }

    let mut halfdense = Vec::<u8>::new();
    let mut dense_to_halfdense = Map::<u8, u8>::new();
    for chunk in &dense {
        let original_pos = chunkmap[chunk];
        if dense_to_halfdense.contains_key(&original_pos) {
            continue;
        }
        let mut front = [0u8; CHUNK / 2];
        let mut back = [0u8; CHUNK / 2];
        front.copy_from_slice(&chunk[..CHUNK / 2]);
        back.copy_from_slice(&chunk[CHUNK / 2..]);
        dense_to_halfdense.insert(
            original_pos,
            match u8::try_from(halfdense.len() / (CHUNK / 2)) {
                Ok(byte) => byte,
                Err(_) => panic!("exceeded 256 half-chunks"),
            },
        );
        halfdense.extend_from_slice(&front);
        halfdense.extend_from_slice(&back);
        while let Some(next) = halfchunkmap.get_mut(&back).and_then(VecDeque::pop_front) {
            let mut concat = empty_chunk;
            concat[..CHUNK / 2].copy_from_slice(&back);
            concat[CHUNK / 2..].copy_from_slice(&next);
            let original_pos = chunkmap[&concat];
            if dense_to_halfdense.contains_key(&original_pos) {
                continue;
            }
            dense_to_halfdense.insert(
                original_pos,
                match u8::try_from(halfdense.len() / (CHUNK / 2) - 1) {
                    Ok(byte) => byte,
                    Err(_) => panic!("exceeded 256 half-chunks"),
                },
            );
            halfdense.extend_from_slice(&next);
            back = next;
        }
    }

    for index in &mut index_start {
        *index = dense_to_halfdense[index];
    }
    for index in &mut index_continue {
        *index = dense_to_halfdense[index];
    }

    let out = write::output(&index_start, &index_continue, &halfdense);
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(PATH);
    fs::write(path, out)
}
