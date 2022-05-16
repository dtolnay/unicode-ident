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
