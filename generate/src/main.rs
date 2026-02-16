// To regenerate tables, run the following in the repo root:
//
// $ cargo install ucd-generate
// $ curl -LO https://www.unicode.org/Public/17.0.0/ucd/UCD.zip
// $ unzip UCD.zip -d UCD
// $ ucd-generate property-bool UCD --include XID_Start,XID_Continue > tests/table/tables.rs
// $ ucd-generate property-bool UCD --include XID_Start,XID_Continue --fst-dir tests/fst
// $ ucd-generate property-bool UCD --include XID_Start,XID_Continue --trie-set > tests/trie/trie.rs
// $ cargo run --manifest-path generate/Cargo.toml

#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation, // https://github.com/rust-lang/rust-clippy/issues/9613
    clippy::items_after_statements,
    clippy::let_underscore_untyped,
    clippy::match_wild_err_arm,
    clippy::module_name_repetitions,
    clippy::too_many_lines,
    clippy::uninlined_format_args
)]

mod output;
mod parse;
mod write;

use crate::parse::parse_xid_properties;
use std::collections::BTreeMap as Map;
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
    let properties = parse_xid_properties(&ucd_dir);

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
                        *this_start |= (properties.is_xid_start(ch) as u8) << k;
                        *this_continue |= (properties.is_xid_continue(ch) as u8) << k;
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

    // Compress the LEAF array by overlapping chunks at half-chunk boundaries.
    //
    // If chunk i's back half equals chunk j's front half, placing them
    // adjacently saves 32 bytes. We find the maximum number of such overlaps
    // by modeling this as a bipartite matching problem (left side = back
    // halves, right side = front halves) and solving with Kuhn's algorithm.

    let num_chunks = dense.len();

    let front_of: Vec<[u8; CHUNK / 2]> = dense
        .iter()
        .map(|c| c[..CHUNK / 2].try_into().unwrap())
        .collect();
    let back_of: Vec<[u8; CHUNK / 2]> = dense
        .iter()
        .map(|c| c[CHUNK / 2..].try_into().unwrap())
        .collect();

    // Build index from front-half value to chunk indices for efficient lookup.
    let mut chunks_by_front: Map<[u8; CHUNK / 2], Vec<usize>> = Map::new();
    for (j, front) in front_of.iter().enumerate() {
        chunks_by_front.entry(*front).or_default().push(j);
    }

    // adj_list[i] = chunks whose front half matches chunk i's back half,
    // meaning they can follow chunk i with a 32-byte overlap. Exclude
    // self-edges (the all-zeros and all-ones chunks have front == back).
    let adj_list: Vec<Vec<usize>> = (0..num_chunks)
        .map(|i| {
            chunks_by_front
                .get(&back_of[i])
                .map(|js| js.iter().copied().filter(|&j| j != i).collect())
                .unwrap_or_default()
        })
        .collect();

    // Maximum bipartite matching via Kuhn's algorithm (augmenting paths).
    // prev_of[j] = Some(i) means chunk i is matched to precede chunk j.
    let mut prev_of: Vec<Option<usize>> = vec![None; num_chunks];

    // DFS for an augmenting path from `src`. If found, augments the matching
    // in-place (rehoming existing matches to preserve validity) and returns true.
    fn try_kuhn(
        src: usize,
        adj_list: &[Vec<usize>],
        visited: &mut [bool],
        prev_of: &mut [Option<usize>],
    ) -> bool {
        for &dst in &adj_list[src] {
            if !visited[dst] {
                visited[dst] = true;
                // If dst is free, or its current match can be rehomed, claim dst.
                if prev_of[dst].is_none_or(|prev| try_kuhn(prev, adj_list, visited, prev_of)) {
                    prev_of[dst] = Some(src);
                    return true;
                }
            }
        }
        false
    }

    // Try every left vertex. A failed attempt stays failed because later
    // rounds only shrink the set of free right vertices (Berge's theorem).
    for i in 0..num_chunks {
        let mut visited = vec![false; num_chunks];
        try_kuhn(i, &adj_list, &mut visited, &mut prev_of);
    }

    // Invert the matching into a forward map for chain traversal.
    let mut next_of: Vec<Option<usize>> = vec![None; num_chunks];
    for (j, &prev) in prev_of.iter().enumerate() {
        if let Some(prev) = prev {
            next_of[prev] = Some(j);
        }
    }

    // Chunk 0 (all zeros) is special and must be laid out first at halfdense position 0,
    // because the runtime defaults to index 0 for codepoints beyond the trie.
    // Remove any incoming edge so chunk 0 becomes a chain start.
    if let Some(prev) = prev_of[0] {
        next_of[prev] = None;
        prev_of[0] = None;
    }

    // Lay out chains into halfdense, starting with chunk 0's chain.
    let mut halfdense = Vec::<u8>::new();
    let mut dense_to_halfdense = Map::<u8, u8>::new();

    for start in (0..num_chunks).filter(|&i| prev_of[i].is_none()) {
        dense_to_halfdense.insert(
            start as u8,
            u8::try_from(halfdense.len() / (CHUNK / 2)).expect("exceeded 256 half-chunks"),
        );
        halfdense.extend_from_slice(&front_of[start]);
        halfdense.extend_from_slice(&back_of[start]);

        // Write the rest of the chain: each chunk's front half overlaps the
        // previous chunk's back half, so only append the back half.
        let mut curr = start;
        while let Some(next) = next_of[curr] {
            dense_to_halfdense.insert(
                next as u8,
                u8::try_from(halfdense.len() / (CHUNK / 2) - 1).expect("exceeded 256 half-chunks"),
            );
            halfdense.extend_from_slice(&back_of[next]);
            curr = next;
        }
    }

    // Each chunk can be both a predecessor (back half) and a successor
    // (front half), so next_of can form cycles with no chain start.
    // We broke chunk 0's cycle above; verify no others exist.
    assert_eq!(
        dense_to_halfdense.len(),
        num_chunks,
        "not all chunks were laid out",
    );

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
