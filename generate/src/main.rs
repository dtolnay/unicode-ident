// To regenerate tables, run the following in the repo root:
//
// $ cargo install ucd-generate
// $ curl -LO https://www.unicode.org/Public/zipped/14.0.0/UCD.zip
// $ unzip UCD.zip -d UCD
// $ ucd-generate property-bool UCD --include XID_Start,XID_Continue > generate/src/ucd.rs
// $ ucd-generate property-bool UCD --include XID_Start,XID_Continue --fst-dir tests/fst
// $ ucd-generate property-bool UCD --include XID_Start,XID_Continue --trie-set > tests/trie/trie.rs
// $ cargo run --manifest-path generate/Cargo.toml

#[rustfmt::skip]
#[allow(dead_code)]
mod ucd;

mod output;

use crate::output::Output;
use std::cmp::Ordering;
use std::collections::{BTreeMap as Map, VecDeque};
use std::fs;
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

fn main() {
    let mut chunkmap = Map::<[u8; CHUNK], u8>::new();
    let mut dense = Vec::<[u8; CHUNK]>::new();
    let mut new_chunk = |chunk| {
        if let Some(prev) = chunkmap.get(&chunk) {
            *prev
        } else {
            dense.push(chunk);
            let new = u8::try_from(chunkmap.len()).unwrap();
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
            u8::try_from(halfdense.len() / (CHUNK / 2)).unwrap(),
        );
        halfdense.extend_from_slice(&front);
        halfdense.extend_from_slice(&back);
        while let Some(next) = halfchunkmap
            .get_mut(&back)
            .and_then(|deque| deque.pop_front())
        {
            let mut concat = empty_chunk;
            concat[..CHUNK / 2].copy_from_slice(&back);
            concat[CHUNK / 2..].copy_from_slice(&next);
            let original_pos = chunkmap[&concat];
            if dense_to_halfdense.contains_key(&original_pos) {
                continue;
            }
            dense_to_halfdense.insert(
                original_pos,
                u8::try_from(halfdense.len() / (CHUNK / 2) - 1).unwrap(),
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

    let mut out = Output::new();
    writeln!(out, "const T: bool = true;");
    writeln!(out, "const F: bool = false;");
    writeln!(out);

    writeln!(out, "#[repr(C, align(8))]");
    writeln!(out, "pub(crate) struct Align8<T>(pub(crate) T);");
    writeln!(out, "#[repr(C, align(64))]");
    writeln!(out, "pub(crate) struct Align64<T>(pub(crate) T);");
    writeln!(out);

    writeln!(
        out,
        "pub(crate) static ASCII_START: Align64<[bool; 128]> = Align64([",
    );
    for i in 0u8..4 {
        write!(out, "   ");
        for j in 0..32 {
            let ch = (i * 32 + j) as char;
            write!(out, " {},", if is_xid_start(ch) { 'T' } else { 'F' });
        }
        writeln!(out);
    }
    writeln!(out, "]);");
    writeln!(out);

    writeln!(
        out,
        "pub(crate) static ASCII_CONTINUE: Align64<[bool; 128]> = Align64([",
    );
    for i in 0u8..4 {
        write!(out, "   ");
        for j in 0..32 {
            let ch = (i * 32 + j) as char;
            write!(out, " {},", if is_xid_continue(ch) { 'T' } else { 'F' });
        }
        writeln!(out);
    }
    writeln!(out, "]);");
    writeln!(out);

    writeln!(out, "pub(crate) const CHUNK: usize = {};", CHUNK);
    writeln!(out);

    writeln!(
        out,
        "pub(crate) static TRIE_START: Align8<[u8; {}]> = Align8([",
        index_start.len(),
    );
    for line in index_start.chunks(16) {
        write!(out, "   ");
        for byte in line {
            write!(out, " 0x{:02X},", byte);
        }
        writeln!(out);
    }
    writeln!(out, "]);");
    writeln!(out);

    writeln!(
        out,
        "pub(crate) static TRIE_CONTINUE: Align8<[u8; {}]> = Align8([",
        index_continue.len(),
    );
    for line in index_continue.chunks(16) {
        write!(out, "   ");
        for byte in line {
            write!(out, " 0x{:02X},", byte);
        }
        writeln!(out);
    }
    writeln!(out, "]);");
    writeln!(out);

    writeln!(
        out,
        "pub(crate) static LEAF: Align64<[u8; {}]> = Align64([",
        halfdense.len(),
    );
    for line in halfdense.chunks(16) {
        write!(out, "   ");
        for byte in line {
            write!(out, " 0x{:02X},", byte);
        }
        writeln!(out);
    }
    writeln!(out, "]);");

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(PATH);
    fs::write(path, out).unwrap();
}
