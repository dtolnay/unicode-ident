// To regenerate tables, run the following in the repo root:
//
// $ cargo install ucd-generate
// $ curl -LO https://www.unicode.org/Public/zipped/15.1.0/UCD.zip
// $ unzip UCD.zip -d UCD
// $ ucd-generate property-bool UCD --include ID_Start,ID_Continue > tests/tables/tables.rs
// $ ucd-generate property-bool UCD --include ID_Start,ID_Continue --fst-dir tests/fst
// $ ucd-generate property-bool UCD --include ID_Start,ID_Continue --trie-set > tests/trie/trie.rs
// $ cargo run --manifest-path generate/Cargo.toml

#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation, // https://github.com/rust-lang/rust-clippy/issues/9613
    clippy::let_underscore_untyped,
    clippy::match_wild_err_arm,
    clippy::module_name_repetitions,
    clippy::too_many_lines,
    clippy::uninlined_format_args
)]

mod output;
mod parse;
mod write;

use crate::parse::parse_id_properties;
use std::collections::{BTreeMap as Map, VecDeque};
use std::convert::TryFrom;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;

const CHUNK: usize = 64;
const UCD: &str = "UCD";
const TABLES: &str = "src/tables.rs";

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let unicode_ident_dir = manifest_dir.parent().unwrap();
    let ucd_dir = unicode_ident_dir.join(UCD);
    let properties = parse_id_properties(&ucd_dir);

    let mut chunkmap = Map::<[u8; CHUNK], u8>::new();
    let mut dense = Vec::<[u8; CHUNK]>::new();
    let mut new_chunk = |chunk| {
        if let Some(prev) = chunkmap.get(&chunk) {
            *prev
        } else {
            dense.push(chunk);
            let Ok(new) = u8::try_from(chunkmap.len()) else {
                panic!("exceeded 256 unique chunks");
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
                        *this_start |= (properties.is_id_start(ch) as u8) << k;
                        *this_continue |= (properties.is_id_continue(ch) as u8) << k;
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

    let out = write::output(&properties, &index_start, &index_continue, &halfdense);
    let path = unicode_ident_dir.join(TABLES);
    if let Err(err) = fs::write(&path, out) {
        let _ = writeln!(io::stderr(), "{}: {err}", path.display());
        process::exit(1);
    }
}
