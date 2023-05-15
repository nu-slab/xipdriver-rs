//! hwh_parser
use anyhow::{Context, Result};
use std::collections::HashMap;

macro_rules! get_attr {
    ($node: expr, $attr: expr) => {
        $node
            .attribute($attr)
            .with_context(|| format!("Attribute not found: {}", $attr))
    };
}

#[derive(Clone, Debug)]
struct Modules {
    fullname: String,
    vlnv: String,
    bdtype: String,
}

#[derive(Clone, Debug)]
pub struct Ip {
    pub fullname: String,
    pub instname: String,
    pub vlnv: String,
    pub bdtype: String,
    pub phys_addr: usize,
    pub addr_range: usize,
}

fn parse_with_prefix(s: &str) -> Result<usize> {
    let mut radix = 10;
    if s.len() > 2 {
        radix = match s
            .chars()
            .nth(1)
            .context("Radix could not be determined.")?
            .to_ascii_lowercase()
        {
            'b' => 2,
            'o' => 8,
            'x' => 16,
            _ => 10,
        };
    }
    if radix == 10 {
        Ok(s.parse()?)
    } else {
        Ok(usize::from_str_radix(&s[2..], radix)?)
    }
}

pub fn parse(hwh_path: &str) -> Result<HashMap<String, Ip>> {
    let reader = std::fs::read_to_string(hwh_path)?;
    let hwh = roxmltree::Document::parse(&reader)?;
    let mut inst2attr = HashMap::new();

    for node in hwh.descendants() {
        if node.tag_name().name() == "MODULE" {
            inst2attr.insert(
                String::from(get_attr!(node, "INSTANCE")?),
                Modules {
                    fullname: String::from(get_attr!(node, "FULLNAME")?),
                    vlnv: String::from(get_attr!(node, "VLNV")?),
                    bdtype: String::from(get_attr!(node, "BDTYPE").unwrap_or("")),
                },
            );
        }
    }

    let mut map: HashMap<String, Ip> = HashMap::new();
    for node in hwh.descendants() {
        if node.tag_name().name() == "MEMRANGE" {
            if let Some(attr) = inst2attr.get(get_attr!(node, "INSTANCE")?) {
                let base_addr = parse_with_prefix(get_attr!(node, "BASEVALUE")?)?;
                let high_addr = parse_with_prefix(get_attr!(node, "HIGHVALUE")?)?;
                map.insert(
                    attr.fullname.clone(),
                    Ip {
                        fullname: attr.fullname.clone(),
                        instname: String::from(get_attr!(node, "INSTANCE")?),
                        vlnv: attr.vlnv.clone(),
                        bdtype: attr.bdtype.clone(),
                        phys_addr: base_addr,
                        addr_range: high_addr - base_addr + 1,
                    },
                );
            }
        }
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_with_prefix_test() {
        assert_eq!(parse_with_prefix("1").unwrap(), 1);
        assert_eq!(parse_with_prefix("01").unwrap(), 1);
        assert_eq!(parse_with_prefix("12").unwrap(), 12);
        assert_eq!(parse_with_prefix("0120").unwrap(), 120);
        assert_eq!(parse_with_prefix("120").unwrap(), 120);
        assert_eq!(parse_with_prefix("0b1111000").unwrap(), 120);
        assert_eq!(parse_with_prefix("0o170").unwrap(), 120);
        assert_eq!(parse_with_prefix("0x78").unwrap(), 120);
        parse_with_prefix("x078").unwrap_err();
        parse_with_prefix("00x078").unwrap_err();
        parse_with_prefix("0x").unwrap_err();
        parse_with_prefix("0z").unwrap_err();
    }
}
