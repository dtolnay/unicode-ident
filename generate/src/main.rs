// To regenerate tables, run the following in the repo root:
//
// $ cargo install ucd-generate
// $ curl -LO https://www.unicode.org/Public/zipped/14.0.0/UCD.zip
// $ unzip UCD.zip -d UCD
// $ ucd-generate property-bool UCD --include XID_Start,XID_Continue > generate/src/ucd.rs
// $ cargo run --manifest-path generate/Cargo.toml

#[rustfmt::skip]
#[allow(dead_code)]
mod ucd;

fn main() {
    /* TODO */
}
