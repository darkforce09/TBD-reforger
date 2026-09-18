//! `TRUSTED_PROXIES` entries: parsing one address or CIDR block, and testing whether a peer
//! address falls inside it.

use std::net::IpAddr;

/// One trusted reverse proxy: a bare address (`127.0.0.1`) or a CIDR block (`10.0.0.0/8`).
///
/// # Why this is hand-rolled rather than `ipnet`
///
/// Two functions — parse and "does this address fall inside" — over `[u8; 4]` / `[u8; 16]`. A
/// dependency for that would be more supply chain than arithmetic, and the arithmetic is unit
/// tested beside it.
///
/// # The rules, and what each one refuses
///
/// * **A bare address is a single host** (`/32`, `/128`). It is *not* silently widened to the
///   surrounding network, which is the classic way a trusted-proxy list ends up trusting a whole
///   datacentre.
/// * **A CIDR must be written as its network address.** `10.0.0.5/8` is refused rather than
///   quietly read as `10.0.0.0/8`, because that reading trusts 16 million addresses the operator
///   did not type. The error names the form to write instead.
/// * **IPv4-mapped IPv6 is canonicalised** (`::ffff:127.0.0.1` → `127.0.0.1`), on both the
///   configured address and the address being tested. A dual-stack listener hands axum the mapped
///   form, and without this an operator who correctly wrote `127.0.0.1` would get *no* match —
///   silently falling back to the shared-bucket behaviour they were trying to fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProxyNet {
    base: IpAddr,
    prefix_len: u8,
}

impl ProxyNet {
    /// Parse one `TRUSTED_PROXIES` entry. `Err` carries a reason fit to print at boot.
    pub fn parse(entry: &str) -> Result<Self, &'static str> {
        let entry = entry.trim();
        let Some((addr, len)) = entry.split_once('/') else {
            // Bare address: exactly this host. Canonicalised so either spelling of a mapped
            // IPv4 address matches a peer that arrives in either spelling.
            let base = entry
                .parse::<IpAddr>()
                .map_err(|_| "not an IP address or CIDR block")?
                .to_canonical();
            return Ok(Self {
                prefix_len: full_prefix_len(&base),
                base,
            });
        };
        // With an explicit prefix the family is the one the operator wrote — canonicalising here
        // would turn `::ffff:10.0.0.0/104` into an IPv4 base carrying an IPv6 prefix length.
        // Write IPv4 proxies in IPv4 form.
        let base = addr
            .parse::<IpAddr>()
            .map_err(|_| "the part before `/` is not an IP address")?;
        let prefix_len = len
            .parse::<u8>()
            .map_err(|_| "the part after `/` is not a prefix length")?;
        if prefix_len > full_prefix_len(&base) {
            return Err("prefix length is longer than the address family allows");
        }
        if !host_bits_are_zero(&base, prefix_len) {
            return Err(
                "host bits are set — write the network address (e.g. `10.0.0.0/8`, not \
                 `10.0.0.5/8`), so the entry cannot trust more than it says",
            );
        }
        Ok(Self { base, prefix_len })
    }

    /// True when `ip` falls inside this network.
    ///
    /// A mismatched family is `false`, never a panic and never a match: an IPv6 peer does not
    /// belong to an IPv4 proxy's network however the two are spelled.
    pub fn contains(&self, ip: IpAddr) -> bool {
        match (self.base, ip.to_canonical()) {
            (IpAddr::V4(base), IpAddr::V4(ip)) => {
                prefix_eq(&base.octets(), &ip.octets(), self.prefix_len)
            }
            (IpAddr::V6(base), IpAddr::V6(ip)) => {
                prefix_eq(&base.octets(), &ip.octets(), self.prefix_len)
            }
            _ => false,
        }
    }
}

/// Parse every entry. `Err` names the first bad one — `(entry, why)`.
///
/// Used twice on purpose: once by `Config::validate` so a typo is a boot failure, and once by
/// [`crate::core::middleware::RateLimitState::new`] so the middleware holds parsed networks rather than
/// re-parsing strings per request.
pub fn parse_trusted_proxies(entries: &[String]) -> Result<Vec<ProxyNet>, (String, &'static str)> {
    entries
        .iter()
        .map(|e| ProxyNet::parse(e).map_err(|why| (e.clone(), why)))
        .collect()
}

/// Bits in a full address of this family.
fn full_prefix_len(ip: &IpAddr) -> u8 {
    match ip {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    }
}

/// True when the first `prefix_len` bits of `a` and `b` are equal.
fn prefix_eq(a: &[u8], b: &[u8], prefix_len: u8) -> bool {
    let whole = usize::from(prefix_len / 8);
    let rest = prefix_len % 8;
    if a[..whole] != b[..whole] {
        return false;
    }
    if rest == 0 {
        return true;
    }
    // Compare only the leading `rest` bits of the next byte.
    let mask = 0xffu8 << (8 - rest);
    (a[whole] ^ b[whole]) & mask == 0
}

/// True when every bit past `prefix_len` is zero — i.e. the address IS its network address.
fn host_bits_are_zero(ip: &IpAddr, prefix_len: u8) -> bool {
    fn check(octets: &[u8], prefix_len: u8) -> bool {
        let whole = usize::from(prefix_len / 8);
        let rest = prefix_len % 8;
        if rest != 0 && octets[whole] & (0xffu8 >> rest) != 0 {
            return false;
        }
        let tail = if rest == 0 { whole } else { whole + 1 };
        octets[tail..].iter().all(|b| *b == 0)
    }
    match ip {
        IpAddr::V4(v4) => check(&v4.octets(), prefix_len),
        IpAddr::V6(v6) => check(&v6.octets(), prefix_len),
    }
}

#[cfg(test)]
#[path = "tests/proxy_network.rs"]
mod tests;
