use std::collections::BTreeSet as Set;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;

pub struct Properties {
    id_start: Set<u32>,
    id_continue: Set<u32>,
}

impl Properties {
    pub fn is_id_start(&self, ch: char) -> bool {
        self.id_start.contains(&(ch as u32))
    }

    pub fn is_id_continue(&self, ch: char) -> bool {
        self.id_continue.contains(&(ch as u32))
    }
}

pub fn parse_id_properties(ucd_dir: &Path) -> Properties {
    let mut properties = Properties {
        id_start: Set::new(),
        id_continue: Set::new(),
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
            "ID_Start" => &mut properties.id_start,
            "ID_Continue" => &mut properties.id_continue,
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
