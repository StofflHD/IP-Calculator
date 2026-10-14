//! IPv6 address type classification and field layout.

use std::net::{Ipv4Addr, Ipv6Addr};

/// Parses an IPv6 address, tolerating brackets, a zone id and a prefix suffix.
pub fn parse(s: &str) -> Option<u128> {
    let s = s.trim().trim_start_matches('[');
    let s = s.split(']').next()?;
    let s = s.split('/').next()?;
    let s = s.split('%').next()?;
    s.trim().parse::<Ipv6Addr>().ok().map(u128::from)
}

pub fn compressed(a: u128) -> String {
    Ipv6Addr::from(a).to_string()
}

pub fn expanded(a: u128) -> String {
    Ipv6Addr::from(a)
        .segments()
        .iter()
        .map(|s| format!("{s:04x}"))
        .collect::<Vec<_>>()
        .join(":")
}

pub fn reverse_dns(a: u128) -> String {
    let mut s = String::new();
    for i in 0..32 {
        s.push_str(&format!("{:x}.", (a >> (i * 4)) & 0xf));
    }
    s + "ip6.arpa"
}

/// How the bits of an address of a given type are laid out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Layout {
    Unspecified,
    Loopback,
    LinkLocal,
    SixOverFour,
    Isatap,
    SiteLocal,
    Unformatted,
    V4Compatible,
    V4Mapped,
    Nat64,
    GlobalUnicast,
    Teredo,
    Orchid,
    Documentation,
    SixToFour,
    UniqueLocal,
    Multicast,
    SolicitedNode,
    None,
}

pub struct AddrType {
    pub name: &'static str,
    /// Empty for abstract categories.
    pub pattern: &'static str,
    pub note: &'static str,
    pub depth: u8,
    layout: Layout,
}

impl AddrType {
    pub fn is_abstract(&self) -> bool {
        self.pattern.is_empty()
    }

    /// Network address and prefix length of the pattern.
    pub fn prefix(&self) -> Option<(u128, u8)> {
        let (a, p) = self.pattern.split_once('/')?;
        Some((a.parse::<Ipv6Addr>().ok()?.into(), p.parse().ok()?))
    }

    pub fn contains(&self, a: u128) -> bool {
        self.prefix().is_some_and(|(net, p)| a & mask(p) == net)
    }
}

const fn t(
    name: &'static str,
    pattern: &'static str,
    note: &'static str,
    depth: u8,
    layout: Layout,
) -> AddrType {
    AddrType {
        name,
        pattern,
        note,
        depth,
        layout,
    }
}

/// Tree of address types in display order; `depth` gives the nesting.
pub const TYPES: &[AddrType] = &[
    t("Unspecified", "::/128", "", 0, Layout::Unspecified),
    t("Loopback", "::1/128", "", 0, Layout::Loopback),
    t("Unicast", "", "Abstract", 0, Layout::None),
    t("Link-Local", "FE80::/10", "", 1, Layout::LinkLocal),
    t("6over4", "FE80::/96", "", 2, Layout::SixOverFour),
    t("ISATAP", "FE80::5EFE:0:0/96", "", 2, Layout::Isatap),
    t(
        "Site-Local",
        "FEC0::/10",
        "Deprecated",
        1,
        Layout::SiteLocal,
    ),
    t("Unique Local", "FC00::/7", "", 1, Layout::UniqueLocal),
    t("Global", "", "Abstract", 1, Layout::None),
    t("Unformatted", "::/3", "", 2, Layout::Unformatted),
    t(
        "IPv4-Compatible",
        "::/96",
        "Deprecated",
        3,
        Layout::V4Compatible,
    ),
    t("IPv4-Mapped", "::FFFF:0:0/96", "", 3, Layout::V4Mapped),
    t("NAT64", "64:FF9B::/96", "", 3, Layout::Nat64),
    t("Global Unicast", "2000::/3", "", 2, Layout::GlobalUnicast),
    t("Teredo", "2001::/32", "", 3, Layout::Teredo),
    t("ORCHIDv2", "2001:20::/28", "", 3, Layout::Orchid),
    t(
        "Documentation",
        "2001:DB8::/32",
        "",
        3,
        Layout::Documentation,
    ),
    t("6to4", "2002::/16", "", 3, Layout::SixToFour),
    t("Multicast", "FF00::/8", "", 0, Layout::Multicast),
    t(
        "Solicited-Node",
        "FF02::1:FF00:0/104",
        "",
        1,
        Layout::SolicitedNode,
    ),
];

pub fn mask(prefix: u8) -> u128 {
    if prefix == 0 {
        0
    } else {
        u128::MAX << (128 - prefix as u32)
    }
}

/// Index into [`TYPES`] of the most specific type matching `a`.
pub fn classify(a: u128) -> Option<usize> {
    TYPES
        .iter()
        .enumerate()
        .filter_map(|(i, t)| t.prefix().filter(|_| t.contains(a)).map(|(_, p)| (i, p)))
        .max_by_key(|&(_, p)| p)
        .map(|(i, _)| i)
}

/// Index of the parent of `TYPES[i]` in the tree.
pub fn parent(i: usize) -> Option<usize> {
    let d = TYPES[i].depth;
    (0..i).rev().find(|&j| TYPES[j].depth < d)
}

pub fn has_children(i: usize) -> bool {
    TYPES.get(i + 1).is_some_and(|n| n.depth > TYPES[i].depth)
}

/// One section of the address format bar.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub bits: u8,
    pub value: String,
    pub label: String,
}

fn bits(a: u128, start: u8, len: u8) -> u128 {
    (a << start) >> (128 - len as u32)
}

fn v4(x: u128) -> String {
    Ipv4Addr::from(x as u32).to_string()
}

/// Formats a bit field: binary when not nibble aligned, otherwise hex in groups of four.
fn value(a: u128, start: u8, len: u8) -> String {
    let v = bits(a, start, len);
    if len % 4 != 0 {
        return format!("{v:0w$b}", w = len as usize);
    }
    let hex = format!("{v:0w$x}", w = len as usize / 4);
    if hex.len() <= 4 {
        return hex;
    }
    let chunks: Vec<String> = hex
        .as_bytes()
        .chunks(4)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect();
    chunks.join(":")
}

fn f(a: u128, start: u8, len: u8, label: &str) -> Field {
    Field {
        bits: len,
        value: value(a, start, len),
        label: label.into(),
    }
}

fn fv(len: u8, value: String, label: &str) -> Field {
    Field {
        bits: len,
        value,
        label: label.into(),
    }
}

fn scope_name(s: u128) -> &'static str {
    match s {
        1 => "interface",
        2 => "link",
        3 => "realm",
        4 => "admin",
        5 => "site",
        8 => "organization",
        0xe => "global",
        _ => "reserved",
    }
}

/// Field layout of `a` interpreted as `TYPES[type_idx]`.
pub fn fields(a: u128, type_idx: Option<usize>) -> Vec<Field> {
    let layout = type_idx.map_or(Layout::None, |i| TYPES[i].layout);
    match layout {
        Layout::Unspecified => vec![fv(128, "0000…0000".into(), "Unspecified Address Constant")],
        Layout::Loopback => vec![fv(128, "0000…0001".into(), "Loopback Address Constant")],
        Layout::LinkLocal | Layout::SiteLocal => vec![
            f(a, 0, 10, "Prefix"),
            f(
                a,
                10,
                54,
                if layout == Layout::LinkLocal {
                    "Zero"
                } else {
                    "Subnet ID"
                },
            ),
            f(a, 64, 64, "Interface ID"),
        ],
        Layout::SixOverFour => vec![
            f(a, 0, 96, "Link-Local Prefix"),
            fv(32, v4(bits(a, 96, 32)), "IPv4 Address"),
        ],
        Layout::Isatap => vec![
            f(a, 0, 64, "Link-Local Prefix"),
            f(a, 64, 32, "ISATAP Identifier"),
            fv(32, v4(bits(a, 96, 32)), "IPv4 Address"),
        ],
        Layout::UniqueLocal => vec![
            f(a, 0, 7, "Prefix"),
            f(a, 7, 1, "L"),
            f(a, 8, 40, "Global ID"),
            f(a, 48, 16, "Subnet ID"),
            f(a, 64, 64, "Interface ID"),
        ],
        Layout::V4Compatible => vec![
            f(a, 0, 96, "Zero"),
            fv(32, v4(bits(a, 96, 32)), "IPv4 Address"),
        ],
        Layout::V4Mapped => {
            vec![
                f(a, 0, 80, "Zero"),
                f(a, 80, 16, "FFFF"),
                fv(32, v4(bits(a, 96, 32)), "IPv4 Address"),
            ]
        }
        Layout::Nat64 => vec![
            f(a, 0, 96, "Well-Known Prefix"),
            fv(32, v4(bits(a, 96, 32)), "IPv4 Address"),
        ],
        Layout::GlobalUnicast | Layout::Documentation => vec![
            f(a, 0, 48, "Global Routing Prefix"),
            f(a, 48, 16, "Subnet ID"),
            f(a, 64, 64, "Interface ID"),
        ],
        Layout::Teredo => vec![
            f(a, 0, 32, "Teredo Prefix"),
            fv(32, v4(bits(a, 32, 32)), "Teredo Server"),
            f(a, 64, 16, "Flags"),
            fv(16, (!bits(a, 80, 16) & 0xffff).to_string(), "Client Port"),
            fv(32, v4(!bits(a, 96, 32) & 0xffff_ffff), "Client IPv4"),
        ],
        Layout::Orchid => vec![
            f(a, 0, 28, "Prefix"),
            f(a, 28, 4, "OGA ID"),
            f(a, 32, 96, "Hash"),
        ],
        Layout::SixToFour => vec![
            f(a, 0, 16, "6to4 Prefix"),
            fv(32, v4(bits(a, 16, 32)), "IPv4 Address"),
            f(a, 48, 16, "Subnet ID"),
            f(a, 64, 64, "Interface ID"),
        ],
        Layout::Multicast => {
            let scope = bits(a, 12, 4);
            vec![
                f(a, 0, 8, "Prefix"),
                f(a, 8, 4, "Flags"),
                fv(
                    4,
                    format!("{scope:x}"),
                    &format!("Scope ({})", scope_name(scope)),
                ),
                f(a, 16, 112, "Group ID"),
            ]
        }
        Layout::SolicitedNode => vec![
            f(a, 0, 104, "Solicited-Node Prefix"),
            f(a, 104, 24, "Low-Order Unicast Bits"),
        ],
        Layout::Unformatted | Layout::None => vec![fv(128, compressed(a), "Address")],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(s: &str) -> &'static str {
        classify(parse(s).unwrap()).map_or("-", |i| TYPES[i].name)
    }

    #[test]
    fn classification() {
        assert_eq!(name("::"), "Unspecified");
        assert_eq!(name("::1"), "Loopback");
        assert_eq!(name("fe80::1:2:3:4"), "Link-Local");
        assert_eq!(name("fe80::5efe:10.0.0.1"), "ISATAP");
        assert_eq!(name("::ffff:192.168.1.1"), "IPv4-Mapped");
        assert_eq!(name("::10.0.0.1"), "IPv4-Compatible");
        assert_eq!(name("2001:db8::1"), "Documentation");
        assert_eq!(name("2001:0:4136:e378:8000:63bf:3fff:fdd2"), "Teredo");
        assert_eq!(name("2002:c000:204::1"), "6to4");
        assert_eq!(name("2a00:1450::1"), "Global Unicast");
        assert_eq!(name("fd12:3456::1"), "Unique Local");
        assert_eq!(name("ff02::1:ff00:1234"), "Solicited-Node");
        assert_eq!(name("ff0e::101"), "Multicast");
        assert_eq!(name("4000::1"), "-");
    }

    #[test]
    fn parsing_variants() {
        assert_eq!(parse("[fe80::1%eth0]"), parse("fe80::1"));
        assert_eq!(parse("2001:db8::/32"), parse("2001:db8::"));
        assert_eq!(parse("10.0.0.1"), None);
    }

    #[test]
    fn tree_structure() {
        let six_over_four = TYPES.iter().position(|t| t.name == "6over4").unwrap();
        assert_eq!(TYPES[parent(six_over_four).unwrap()].name, "Link-Local");
        assert!(has_children(2));
        assert!(!has_children(0));
    }

    #[test]
    fn field_layouts() {
        let a = parse("2001:0:4136:e378:8000:63bf:3fff:fdd2").unwrap();
        let fs = fields(a, classify(a));
        assert_eq!(fs.iter().map(|f| f.bits as u32).sum::<u32>(), 128);
        assert_eq!(fs[1].value, "65.54.227.120");
        assert_eq!(fs[3].value, "40000");
        assert_eq!(fs[4].value, "192.0.2.45");

        let a = parse("fe80::1").unwrap();
        let fs = fields(a, TYPES.iter().position(|t| t.name == "Link-Local"));
        assert_eq!(fs[0].value, "1111111010");
        assert_eq!(fs[2].value, "0000:0000:0000:0001");

        for (i, _) in TYPES.iter().enumerate() {
            let total: u32 = fields(a, Some(i)).iter().map(|f| f.bits as u32).sum();
            assert_eq!(total, 128, "{}", TYPES[i].name);
        }
    }

    #[test]
    fn notations() {
        let a = parse("2001:db8::1").unwrap();
        assert_eq!(expanded(a), "2001:0db8:0000:0000:0000:0000:0000:0001");
        assert!(reverse_dns(a).starts_with("1.0.0.0."));
        assert!(reverse_dns(a).ends_with("8.b.d.0.1.0.0.2.ip6.arpa"));
    }
}
