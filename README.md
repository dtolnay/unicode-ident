Unicode ident
=============

Implementation of [Unicode Standard Annex #31][tr31] for determining which
`char` values are valid in programming language identifiers.

[tr31]: https://www.unicode.org/reports/tr31/

This crate is a better optimized implementation of the older `unicode-xid`
crate. This crate uses less static storage, and is able to classify both ASCII
and non-ASCII codepoints with better performance, 2&ndash;10&times; faster than
`unicode-xid`.

<br>

## Comparison of performance

The following table shows a comparison between five Unicode identifier
implementations.

- `unicode-ident` is this crate;
- [`unicode-xid`] is a widely used crate run by the "unicode-rs" org;
- `ucd-trie` and `fst` are two data structures supported by the [`ucd-generate`] tool;
- [`roaring`] is a Rust implementation of Roaring bitmap.

The *static storage* column shows the total size of `static` tables that the
crate bakes into your binary, measured in 1000s of bytes.

The remaining columns show the **cost per call** to evaluate whether a single
`char` has the XID\_Start or XID\_Continue Unicode property, comparing across
different ratios of ASCII to non-ASCII codepoints in the input data.

[`unicode-xid`]: https://github.com/unicode-rs/unicode-xid
[`ucd-generate`]: https://github.com/BurntSushi/ucd-generate
[`roaring`]: https://github.com/RoaringBitmap/roaring-rs

| | static storage | 0% nonascii | 1% | 10% | 100% nonascii |
|---|---|---|---|---|---|
| **`unicode-ident`** | 9.75 K | 0.96 ns | 0.95 ns | 1.09 ns | 1.55 ns |
| **`unicode-xid`** | 11.34 K | 1.88 ns | 2.14 ns | 3.48 ns | 15.63 ns |
| **`ucd-trie`** | 9.95 K | 1.29 ns | 1.28 ns | 1.36 ns | 2.15 ns |
| **`fst`** | 133 K | 55.1 ns | 54.9 ns | 53.2 ns | 28.5 ns |
| **`roaring`** | 66.1 K | 2.78 ns | 3.09 ns | 3.37 ns | 4.70 ns |

Source code for the benchmark is provided in the *bench* directory of this repo
and may be repeated by running `cargo criterion`.

<br>

## Comparison of data structures

#### unicode-xid

They use a sorted array of character ranges, and do a binary search to look up
whether a given character lands inside one of those ranges.

```rust
static XID_Continue_table: [(char, char); 763] = [
    ('\u{30}', '\u{39}'),  // 0-9
    ('\u{41}', '\u{5a}'),  // A-Z
    …
    ('\u{e0100}', '\u{e01ef}'),
];
```

The static storage used by this data structure scales with the number of
contiguous ranges of identifier codepoints in Unicode. Every table entry
consumes 8 bytes, because it consists of a pair of 32-bit `char` values.

In some ranges of the Unicode codepoint space, this is quite a sparse
representation &ndash; there are some ranges where tens of thousands of adjacent
codepoints are all valid identifier characters. In other places, the
representation is quite inefficient. A characater like `µ` (U+00B5) which is
surrounded by non-identifier codepoints consumes 64 bits in the table, while it
would be just 1 bit in a dense bitmap.

On a system with 64-byte cache lines, binary searching the table touches 7 cache
lines on average. Each cache line fits only 8 table entries. Additionally, the
branching performed during the binary search is probably mostly unpredictable to
the branch predictor.

Overall, the crate ends up being about 10&times; slower on non-ASCII input
compared to the fastest crate.

A potential improvement would be to pack the table entries more compactly.
Rust's `char` type is a 21-bit integer padded to 32 bits, which means every
table entry is holding 22 bits of wasted space, adding up to 3.9 K. They could
instead fit every table entry into 6 bytes, leaving out some of the padding, for
a 25% improvement in space used. With some cleverness it may be possible to fit
in 5 bytes or even 4 bytes by storing a low char and an extent, instead of low
char and high char. I don't expect that performance would improve much but this
could be the most efficient for space across all the libraries, needing only
about 7 K to store.

#### ucd-trie

Their data structure is a compressed trie set specifically tailored for Unicode
codepoints. The design is credited to Raph Levien in [rust-lang/rust#33098].

[rust-lang/rust#33098]: https://github.com/rust-lang/rust/pull/33098

```rust
pub struct TrieSet {
    tree1_level1: &'static [u64; 32],
    tree2_level1: &'static [u8; 992],
    tree2_level2: &'static [u64],
    tree3_level1: &'static [u8; 256],
    tree3_level2: &'static [u8],
    tree3_level3: &'static [u64],
}
```

It represents codepoint sets using a trie to achieve prefix compression. The
final states of the trie are embedded in leaves or "chunks", where each chunk is
a 64-bit integer. Each bit position of the integer corresponds to whether a
particular codepoint is in the set or not. These chunks are not just a compact
representation of the final states of the trie, but are also a form of suffix
compression. In particular, if multiple ranges of 64 contiguous codepoints have
the same Unicode properties, then they all map to the same chunk in the final
level of the trie.

Being tailored for Unicode codepoints, this trie is partitioned into three
disjoint sets: tree1, tree2, tree3. The first set corresponds to codepoints \[0,
0x800), the second \[0x800, 0x10000) and the third \[0x10000, 0x110000). These
partitions conveniently correspond to the space of 1 or 2 byte UTF-8 encoded
codepoints, 3 byte UTF-8 encoded codepoints and 4 byte UTF-8 encoded codepoints,
respectively.

Lookups in this data structure are significantly more efficient than binary
search. A lookup touches either 1, 2, or 3 cache lines based on which of the
trie partitions is being accessed.

One possible performance improvement would be for this crate to expose a way to
query based on a UTF-8 encoded string, returning the Unicode property
corresponding to the first character in the string. Without such an API, the
caller is required to tokenize their UTF-8 encoded input data into `char`, hand
the `char` into `ucd-trie`, only for `ucd-trie` to undo that work by converting
back into the variable-length representation for trie traversal.

#### fst

Uses a [finite state transducer][fst]. This representation is built into
[ucd-generate] but I am not aware of any advantage over the `ucd-trie`
representation. In particular `ucd-trie` is optimized for storing Unicode
properties while `fst` is not.

[fst]: https://github.com/BurntSushi/fst
[ucd-generate]: https://github.com/BurntSushi/ucd-generate

As far as I can tell, the main thing that causes `fst` to have large size and
slow lookups for this use case relative to `ucd-trie` is that it does not
specialize for the fact that only 21 of the 32 bits in a `char` are meaningful.
There are some dense arrays in the structure with large ranges that could never
possibly be used.

<br>

## License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
