use std::collections::BTreeSet as Set;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;

pub struct Properties {
    xid_start: Set<u32>,
    xid_continue: Set<u32>,
}

impl Properties {
    pub fn is_xid_start(&self, ch: char) -> bool {
        self.xid_start.contains(&(ch as u32))
    }

    pub fn is_xid_continue(&self, ch: char) -> bool {
        self.xid_continue.contains(&(ch as u32))
    }
}

pub fn parse_xid_properties(ucd_dir: &Path) -> Properties {
    let mut properties = Properties {
        xid_start: Set::new(),
        xid_continue: Set::new(),
    };

    let filename = "DerivedCoreProperties.txt";
    let path = ucd_dir.join(filename);
    let contents = fs::read_to_string(path).unwrap_or_else(|err| {
        let suggestion =
            "Download from https://www.unicode.org/Public/zipped/l5.0.0/UCD.zip and unzip.";
        let _ = writeln!(io::stderr(), "{}: {err}\n{suggestion}", ucd_dir.display());
        process::exit(1);
    });

    for (i, line) in contents.lines().enumerate() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let (lo, hi, name) = parse_line(line).unwrap_or_else(|| {
            let _ = writeln!(io::stderr(), "{filename} line {i} is unexpected:\n{line}");
            process::exit(1);
        });
        let set = match name {
            "XID_Start" => &mut properties.xid_start,
            "XID_Continue" => &mut properties.xid_continue,
            _ => continue,
        };
        set.extend(lo..=hi);
    }

    properties
}

fn parse_line(line: &str) -> Option<(u32, u32, &str)> {
    let (mut codepoint, rest) = line.split_once(';')?;

    let (lo, hi);
    codepoint = codepoint.trim();
    if let Some((a, b)) = codepoint.split_once("..") {
        lo = parse_codepoint(a)?;
        hi = parse_codepoint(b)?;
    } else {
        lo = parse_codepoint(codepoint)?;
        hi = lo;
    }

    let name = rest.trim().split('#').next()?.trim_end();
    Some((lo, hi, name))
}

fn parse_codepoint(s: &str) -> Option<u32> {
    u32::from_str_radix(s, 16).ok()
}
