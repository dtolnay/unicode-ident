use anyhow::Result;
use std::collections::BTreeSet as Set;
use std::path::Path;
use ucd_parse::CoreProperty;

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

pub fn parse_xid_properties(ucd_dir: &Path) -> Result<Properties> {
    let mut properties = Properties {
        xid_start: Set::new(),
        xid_continue: Set::new(),
    };

    let prop_list: Vec<CoreProperty> = ucd_parse::parse(ucd_dir)?;
    for core in prop_list {
        let set = match core.property.as_str() {
            "XID_Start" => &mut properties.xid_start,
            "XID_Continue" => &mut properties.xid_continue,
            _ => continue,
        };
        for codepoint in core.codepoints {
            set.insert(codepoint.value());
        }
    }

    Ok(properties)
}
