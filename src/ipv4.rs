//! IPv4 arithmetic: conversions, classful addressing, subnetting and CIDR.

use std::net::Ipv4Addr;

pub fn fmt(a: u32) -> String {
    Ipv4Addr::from(a).to_string()
}

/// Netmask for a prefix length (0..=32).
pub fn mask(prefix: u8) -> u32 {
    if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix as u32)
    }
}

pub fn fmt_mask(prefix: u8) -> String {
    format!("{}  (/{})", fmt(mask(prefix)), prefix)
}

/// Parses "a.b.c.d" with an optional "/prefix" or "/netmask" suffix.
pub fn parse(s: &str) -> Option<(u32, Option<u8>)> {
    let s = s.trim();
    let (addr, suffix) = match s.split_once('/') {
        Some((a, p)) => (a.trim(), Some(p.trim())),
        None => (s, None),
    };
    let addr: Ipv4Addr = addr.parse().ok()?;
    let prefix = match suffix {
        None => None,
        Some(p) => Some(parse_prefix(p)?),
    };
    Some((u32::from(addr), prefix))
}

fn parse_prefix(p: &str) -> Option<u8> {
    if let Ok(n) = p.parse::<u8>() {
        return (n <= 32).then_some(n);
    }
    let m = u32::from(p.parse::<Ipv4Addr>().ok()?);
    let n = m.leading_ones();
    (m == mask(n as u8)).then_some(n as u8)
}

// ---------------------------------------------------------------- conversions

pub fn dotted_hex(a: u32) -> String {
    let b = a.to_be_bytes();
    format!("{:02x}.{:02x}.{:02x}.{:02x}", b[0], b[1], b[2], b[3])
}

pub fn hex(a: u32) -> String {
    format!("{a:#x}")
}

pub fn dotted_bin(a: u32) -> String {
    let b = a.to_be_bytes();
    format!("{:08b}.{:08b}.{:08b}.{:08b}", b[0], b[1], b[2], b[3])
}

pub fn bin(a: u32) -> String {
    format!("{a:032b}")
}

/// Ethernet MAC address an IPv4 multicast group maps to (RFC 1112).
pub fn multicast_mac(a: u32) -> Option<String> {
    if a >> 28 != 0b1110 {
        return None;
    }
    let b = a.to_be_bytes();
    Some(format!(
        "01:00:5e:{:02x}:{:02x}:{:02x}",
        b[1] & 0x7f,
        b[2],
        b[3]
    ))
}

pub fn parse_decimal(s: &str) -> Option<u32> {
    s.trim().parse().ok()
}

pub fn parse_hex(s: &str) -> Option<u32> {
    let s = s.trim();
    let s = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    if s.is_empty() || s.len() > 8 {
        return None;
    }
    u32::from_str_radix(s, 16).ok()
}

fn parse_dotted(s: &str, radix: u32, max_len: usize) -> Option<u32> {
    let parts: Vec<&str> = s.trim().split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let mut out = 0u32;
    for p in parts {
        let p = p.trim();
        if p.is_empty() || p.len() > max_len {
            return None;
        }
        out = (out << 8) | u8::from_str_radix(p, radix).ok()? as u32;
    }
    Some(out)
}

pub fn parse_dotted_hex(s: &str) -> Option<u32> {
    parse_dotted(s, 16, 2)
}

pub fn parse_dotted_bin(s: &str) -> Option<u32> {
    parse_dotted(s, 2, 8)
}

pub fn parse_bin(s: &str) -> Option<u32> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s.is_empty() || s.len() > 32 {
        return None;
    }
    u32::from_str_radix(&s, 2).ok()
}

// ------------------------------------------------------------------- classes

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    A,
    B,
    C,
    D,
    E,
}

impl Class {
    pub const ALL: [Class; 5] = [Class::A, Class::B, Class::C, Class::D, Class::E];

    pub fn of(a: u32) -> Class {
        match a.leading_ones() {
            0 => Class::A,
            1 => Class::B,
            2 => Class::C,
            3 => Class::D,
            _ => Class::E,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Class::A => "Class A",
            Class::B => "Class B",
            Class::C => "Class C",
            Class::D => "Class D (Multicast)",
            Class::E => "Class E (Reserved)",
        }
    }

    /// Leading bits that identify the class.
    pub fn lead_bits(self) -> u8 {
        match self {
            Class::A => 1,
            Class::B => 2,
            Class::C => 3,
            Class::D | Class::E => 4,
        }
    }

    /// Classful network prefix length; `None` for D and E.
    pub fn network_bits(self) -> Option<u8> {
        match self {
            Class::A => Some(8),
            Class::B => Some(16),
            Class::C => Some(24),
            Class::D | Class::E => None,
        }
    }

    /// Address range of the class as the classic tools show it
    /// (class A without network 0 and the loopback network 127).
    pub fn range(self) -> (u32, u32) {
        let ip = |a, b, c, d| u32::from(Ipv4Addr::new(a, b, c, d));
        match self {
            Class::A => (ip(1, 0, 0, 0), ip(126, 255, 255, 255)),
            Class::B => (ip(128, 0, 0, 0), ip(191, 255, 255, 255)),
            Class::C => (ip(192, 0, 0, 0), ip(223, 255, 255, 255)),
            Class::D => (ip(224, 0, 0, 0), ip(239, 255, 255, 255)),
            Class::E => (ip(240, 0, 0, 0), ip(255, 255, 255, 255)),
        }
    }

    /// Bit usage pattern; `n` = network, `h` = host, `m` = multicast group, `r` = reserved.
    pub fn bit_usage(self, a: u32) -> String {
        let lead = self.lead_bits() as u32;
        let net = self.network_bits().unwrap_or(32) as u32;
        let fill = match self {
            Class::D => 'm',
            Class::E => 'r',
            _ => 'n',
        };
        pattern(|i| {
            if i < lead {
                bit_char(a, i)
            } else if i < net {
                fill
            } else {
                'h'
            }
        })
    }
}

fn bit_char(a: u32, i: u32) -> char {
    if a & (1 << (31 - i)) != 0 {
        '1'
    } else {
        '0'
    }
}

/// Builds a dotted 32-character bit pattern from a per-bit function.
pub fn pattern(f: impl Fn(u32) -> char) -> String {
    let mut s = String::with_capacity(35);
    for i in 0..32 {
        if i > 0 && i % 8 == 0 {
            s.push('.');
        }
        s.push(f(i));
    }
    s
}

// ---------------------------------------------------------------- subnetting

/// Usable subnets for `s` subnet bits. Without RFC 1878 the all-zeros and
/// all-ones subnets are excluded (RFC 950).
pub fn max_subnets(s: u8, allow_zero_subnets: bool) -> u64 {
    match s {
        0 => 1,
        _ if allow_zero_subnets => 1u64 << s,
        _ => (1u64 << s).saturating_sub(2),
    }
}

/// Usable hosts for `h` host bits. With RFC 3021 a /31 holds two hosts.
pub fn max_hosts(h: u8) -> u64 {
    match h {
        0 => 1,
        1 => 2,
        _ => (1u64 << h) - 2,
    }
}

/// Classful subnetting of the network containing an address.
#[derive(Clone, Copy, Debug)]
pub struct SubnetPlan {
    pub base: u32,
    pub net_bits: u8,
    pub prefix: u8,
    pub skip_edges: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Subnet {
    pub id: u32,
    pub first: u32,
    pub last: u32,
    pub broadcast: Option<u32>,
}

impl SubnetPlan {
    pub fn subnet_bits(&self) -> u8 {
        self.prefix - self.net_bits
    }

    pub fn host_bits(&self) -> u8 {
        32 - self.prefix
    }

    /// Index of the first listed subnet.
    pub fn first_index(&self) -> u64 {
        u64::from(self.skip_edges && self.subnet_bits() > 0)
    }

    pub fn count(&self) -> u64 {
        max_subnets(self.subnet_bits(), !self.skip_edges)
    }

    pub fn subnet(&self, index: u64) -> Subnet {
        block(
            self.base | ((index as u32) << self.host_bits().min(31)) & !mask(self.net_bits),
            self.prefix,
        )
    }

    pub fn index_of(&self, a: u32) -> u64 {
        if self.subnet_bits() == 0 {
            0
        } else {
            ((a & !mask(self.net_bits)) >> self.host_bits()) as u64
        }
    }
}

/// Network, host range and broadcast of `a/prefix`.
pub fn block(a: u32, prefix: u8) -> Subnet {
    let id = a & mask(prefix);
    let end = id | !mask(prefix);
    match prefix {
        32 => Subnet {
            id,
            first: id,
            last: id,
            broadcast: None,
        },
        31 => Subnet {
            id,
            first: id,
            last: end,
            broadcast: None,
        },
        _ => Subnet {
            id,
            first: id + 1,
            last: end - 1,
            broadcast: Some(end),
        },
    }
}

/// Start and end address of `a/prefix`.
pub fn range(a: u32, prefix: u8) -> (u32, u32) {
    let id = a & mask(prefix);
    (id, id | !mask(prefix))
}

// ------------------------------------------------------- special-use blocks

pub struct Special {
    pub block: &'static str,
    pub name: &'static str,
    pub rfc: &'static str,
}

/// IANA IPv4 special-purpose address registry (abridged).
pub const SPECIAL: &[Special] = &[
    Special {
        block: "0.0.0.0/8",
        name: "\"This\" network",
        rfc: "RFC 791",
    },
    Special {
        block: "10.0.0.0/8",
        name: "Private-Use",
        rfc: "RFC 1918",
    },
    Special {
        block: "100.64.0.0/10",
        name: "Shared Address Space (CGN)",
        rfc: "RFC 6598",
    },
    Special {
        block: "127.0.0.0/8",
        name: "Loopback",
        rfc: "RFC 1122",
    },
    Special {
        block: "169.254.0.0/16",
        name: "Link Local",
        rfc: "RFC 3927",
    },
    Special {
        block: "172.16.0.0/12",
        name: "Private-Use",
        rfc: "RFC 1918",
    },
    Special {
        block: "192.0.0.0/24",
        name: "IETF Protocol Assignments",
        rfc: "RFC 6890",
    },
    Special {
        block: "192.0.2.0/24",
        name: "Documentation (TEST-NET-1)",
        rfc: "RFC 5737",
    },
    Special {
        block: "192.88.99.0/24",
        name: "6to4 Relay Anycast (deprecated)",
        rfc: "RFC 7526",
    },
    Special {
        block: "192.168.0.0/16",
        name: "Private-Use",
        rfc: "RFC 1918",
    },
    Special {
        block: "198.18.0.0/15",
        name: "Benchmarking",
        rfc: "RFC 2544",
    },
    Special {
        block: "198.51.100.0/24",
        name: "Documentation (TEST-NET-2)",
        rfc: "RFC 5737",
    },
    Special {
        block: "203.0.113.0/24",
        name: "Documentation (TEST-NET-3)",
        rfc: "RFC 5737",
    },
    Special {
        block: "224.0.0.0/4",
        name: "Multicast",
        rfc: "RFC 5771",
    },
    Special {
        block: "240.0.0.0/4",
        name: "Reserved",
        rfc: "RFC 1112",
    },
    Special {
        block: "255.255.255.255/32",
        name: "Limited Broadcast",
        rfc: "RFC 919",
    },
];

/// Index into [`SPECIAL`] of the most specific block containing `a`.
pub fn special_of(a: u32) -> Option<usize> {
    SPECIAL
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let (net, p) = parse(s.block)?;
            let p = p?;
            (a & mask(p) == net).then_some((i, p))
        })
        .max_by_key(|&(_, p)| p)
        .map(|(i, _)| i)
}

pub fn reverse_dns(a: u32) -> String {
    let b = a.to_be_bytes();
    format!("{}.{}.{}.{}.in-addr.arpa", b[3], b[2], b[1], b[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> u32 {
        parse(s).unwrap().0
    }

    #[test]
    fn conversions_match_screenshot() {
        let a = ip("10.20.0.0");
        assert_eq!(a, 169082880);
        assert_eq!(dotted_hex(a), "0a.14.00.00");
        assert_eq!(hex(a), "0xa140000");
        assert_eq!(dotted_bin(a), "00001010.00010100.00000000.00000000");
        assert_eq!(bin(a), "00001010000101000000000000000000");
        assert_eq!(multicast_mac(a), None);
    }

    #[test]
    fn conversions_roundtrip() {
        let a = ip("192.168.17.254");
        assert_eq!(parse_decimal(&a.to_string()), Some(a));
        assert_eq!(parse_hex(&hex(a)), Some(a));
        assert_eq!(parse_dotted_hex(&dotted_hex(a)), Some(a));
        assert_eq!(parse_dotted_bin(&dotted_bin(a)), Some(a));
        assert_eq!(parse_bin(&bin(a)), Some(a));
        assert_eq!(parse_hex("0x1ffffffff"), None);
    }

    #[test]
    fn multicast_mac_mapping() {
        assert_eq!(
            multicast_mac(ip("239.129.2.3")).as_deref(),
            Some("01:00:5e:01:02:03")
        );
    }

    #[test]
    fn prefix_suffix() {
        assert_eq!(parse("10.0.0.1/22"), Some((ip("10.0.0.1"), Some(22))));
        assert_eq!(
            parse("10.0.0.1/255.255.252.0"),
            Some((ip("10.0.0.1"), Some(22)))
        );
        assert_eq!(parse("10.0.0.1/255.0.255.0"), None);
        assert_eq!(parse("10.0.0.1/33"), None);
    }

    #[test]
    fn classes() {
        let a = ip("10.20.0.0");
        assert_eq!(Class::of(a), Class::A);
        assert_eq!(Class::A.bit_usage(a), "0nnnnnnn.hhhhhhhh.hhhhhhhh.hhhhhhhh");
        assert_eq!(
            Class::B.bit_usage(ip("172.16.0.0")),
            "10nnnnnn.nnnnnnnn.hhhhhhhh.hhhhhhhh"
        );
        assert_eq!(Class::of(ip("224.0.0.1")), Class::D);
        assert_eq!(Class::of(ip("250.0.0.1")), Class::E);
    }

    #[test]
    fn subnets_match_screenshot() {
        let plan = SubnetPlan {
            base: ip("10.0.0.0"),
            net_bits: 8,
            prefix: 22,
            skip_edges: false,
        };
        assert_eq!(plan.subnet_bits(), 14);
        assert_eq!(plan.count(), 16384);
        assert_eq!(max_hosts(plan.host_bits()), 1022);
        let a = ip("10.20.0.0");
        assert_eq!(plan.index_of(a), 1280);
        let s = plan.subnet(1280);
        assert_eq!(
            (s.id, s.first, s.last, s.broadcast),
            (
                a,
                ip("10.20.0.1"),
                ip("10.20.3.254"),
                Some(ip("10.20.3.255"))
            )
        );
        assert_eq!(plan.subnet(1273).id, ip("10.19.228.0"));
    }

    #[test]
    fn rfc950_excludes_edge_subnets() {
        assert_eq!(max_subnets(1, false), 0);
        assert_eq!(max_subnets(2, false), 2);
        assert_eq!(max_subnets(2, true), 4);
        let plan = SubnetPlan {
            base: ip("192.168.1.0"),
            net_bits: 24,
            prefix: 26,
            skip_edges: true,
        };
        assert_eq!(plan.first_index(), 1);
        assert_eq!(plan.count(), 2);
    }

    #[test]
    fn small_blocks() {
        assert_eq!(
            block(ip("10.0.0.7"), 31),
            Subnet {
                id: ip("10.0.0.6"),
                first: ip("10.0.0.6"),
                last: ip("10.0.0.7"),
                broadcast: None
            }
        );
        assert_eq!(max_hosts(1), 2);
        assert_eq!(range(ip("10.20.0.0"), 1), (0, ip("127.255.255.255")));
    }

    #[test]
    fn special_blocks() {
        assert_eq!(
            SPECIAL[special_of(ip("10.20.0.0")).unwrap()].rfc,
            "RFC 1918"
        );
        assert_eq!(
            SPECIAL[special_of(ip("255.255.255.255")).unwrap()].name,
            "Limited Broadcast"
        );
        assert_eq!(special_of(ip("8.8.8.8")), None);
        assert_eq!(reverse_dns(ip("10.20.0.1")), "1.0.20.10.in-addr.arpa");
    }
}
