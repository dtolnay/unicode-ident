Unicode ident
=============

Implementation of [Unicode Standard Annex #31][tr31] for determining which
`char` values are valid in programming language identifiers.

[tr31]: https://www.unicode.org/reports/tr31/

This crate is a better optimized implementation of the older `unicode-xid`
crate. This crate uses less static storage, and is able to classify both ASCII
and non-ASCII codepoints with better performance, 2&ndash;10&times; faster than
`unicode-xid`.

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
