//! Deciding whether an address is one we are allowed to talk to.
//!
//! This is the security-critical module in the project so far, and
//! [CODING_QUALITY.md](../../../docs/CODING_QUALITY.md) §6.2 lists the SSRF guard among the
//! things requiring **100% coverage with no exemption path**. It is written pure — IP in,
//! verdict out — so that standard is reachable without a network.
//!
//! # The attack
//!
//! A reader hands us a URL. We fetch it from inside our own network, on a host that can
//! reach things they cannot: the cloud metadata endpoint at `169.254.169.254`, which on many
//! providers hands out credentials to anyone who asks; a database on `127.0.0.1`; anything
//! on the private ranges of the VPC.
//!
//! **The fetch is the vulnerability.** We are a service whose entire purpose is fetching
//! URLs that strangers name, which makes this the one attack we are guaranteed to face.
//!
//! # Why "resolve then verify" is a phrase in the roadmap
//!
//! The obvious implementation is: resolve the hostname, check the IP, then hand the URL to
//! an HTTP client. That is broken, and the break has a name — **DNS rebinding**. The client
//! resolves the name a second time when it connects, and an attacker who controls the
//! authoritative server can answer differently the second time: public IP for our check,
//! `127.0.0.1` for our connection.
//!
//! The fix is to connect to **the address we verified**, never to the name. [`Verdict`]
//! therefore carries the checked [`IpAddr`] and the caller is expected to pin the connection
//! to it. A guard that returns only a boolean cannot be used safely.
//!
//! # Deny by default
//!
//! Every range that is not clearly public is refused. That inevitably refuses a handful of
//! legitimate addresses, which is the correct direction to be wrong in: a refused fetch
//! shows the reader a gap, and a permitted one can exfiltrate a credential.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Why an address was refused. One variant per reason, so a log line says which rule fired
/// rather than "blocked" — and so a false positive can be argued with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// `127.0.0.0/8`, `::1`. Our own machine.
    Loopback,
    /// `10/8`, `172.16/12`, `192.168/16`, `fc00::/7`. The network we are inside.
    Private,
    /// `169.254/16`, `fe80::/10`. **Includes the cloud metadata endpoint**, which is the
    /// single most valuable thing an SSRF can reach.
    LinkLocal,
    /// `100.64/10`. Carrier-grade NAT — shared address space, not the public internet.
    SharedAddressSpace,
    /// `0.0.0.0/8`, `::`. "This network", and on Linux a synonym for localhost.
    Unspecified,
    /// Multicast and broadcast. Nothing we want is behind one.
    NotUnicast,
    /// Documentation, benchmarking, and reserved ranges — `192.0.2/24`, `198.18/15`, and the
    /// rest. Never a real destination, and their presence in a URL suggests a probe.
    Reserved,
    /// An IPv6 address that embeds an IPv4 one — `::ffff:127.0.0.1`, `64:ff9b::/96`.
    ///
    /// A separate variant because this is the classic bypass: the guard checks IPv6 rules,
    /// the address is really IPv4, and a stack that unwraps it connects to loopback.
    EmbeddedIpv4,
}

impl Refusal {
    /// What to tell the reader. Never mentions the address — a reader who typed a hostname
    /// should not learn what it resolved to internally, and one who is probing should learn
    /// nothing at all.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::Loopback | Self::Unspecified => "That address points back at this machine.",
            Self::Private | Self::SharedAddressSpace => "That address is on a private network.",
            Self::LinkLocal => "That address is link-local and not reachable from the internet.",
            Self::NotUnicast => "That is not an address a web page can be served from.",
            Self::Reserved => "That address is in a reserved range.",
            Self::EmbeddedIpv4 => "That address wraps a private address.",
        }
    }
}

/// An address that has been checked and may be connected to.
///
/// Deliberately not a bare `bool`. The whole point of the check is that the connection goes
/// to **this** address rather than to a name that could resolve elsewhere a moment later, so
/// the type carries the address the caller must pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verdict {
    ip: IpAddr,
}

impl Verdict {
    /// The address the caller must connect to, and not re-resolve.
    #[must_use]
    pub const fn addr(&self) -> IpAddr {
        self.ip
    }
}

/// Whether we may connect to this address.
///
/// # Errors
/// The specific rule that refused it.
pub fn check(ip: IpAddr) -> Result<Verdict, Refusal> {
    match ip {
        IpAddr::V4(v4) => check_v4(v4),
        IpAddr::V6(v6) => check_v6(v6),
    }?;
    Ok(Verdict { ip })
}

fn check_v4(ip: Ipv4Addr) -> Result<(), Refusal> {
    let [a, b, ..] = ip.octets();

    if ip.is_loopback() {
        return Err(Refusal::Loopback);
    }
    if ip.is_unspecified() || a == 0 {
        return Err(Refusal::Unspecified);
    }
    if ip.is_link_local() {
        return Err(Refusal::LinkLocal);
    }
    if ip.is_private() {
        return Err(Refusal::Private);
    }
    if ip.is_multicast() || ip.is_broadcast() {
        return Err(Refusal::NotUnicast);
    }
    // 100.64.0.0/10, RFC 6598. Not covered by `is_private`.
    if a == 100 && (64..128).contains(&b) {
        return Err(Refusal::SharedAddressSpace);
    }
    // Documentation (192.0.2/24, 198.51.100/24, 203.0.113/24), benchmarking (198.18/15),
    // IETF protocol assignments (192.0.0/24), and 240/4 reserved.
    let [a, b, c, _] = ip.octets();
    let reserved = (a == 192 && b == 0 && (c == 0 || c == 2))
        || (a == 198 && (b == 18 || b == 19))
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113)
        || a >= 240;
    if reserved {
        return Err(Refusal::Reserved);
    }
    Ok(())
}

fn check_v6(ip: Ipv6Addr) -> Result<(), Refusal> {
    if ip.is_loopback() {
        return Err(Refusal::Loopback);
    }
    if ip.is_unspecified() {
        return Err(Refusal::Unspecified);
    }
    if ip.is_multicast() {
        return Err(Refusal::NotUnicast);
    }

    // Any IPv6 address carrying an IPv4 one is refused outright rather than unwrapped and
    // re-checked. `::ffff:8.8.8.8` is a legitimate public address and is refused anyway:
    // the number of stacks and proxies that disagree about how to treat these is larger
    // than the number of sites only reachable through one.
    if ip.to_ipv4_mapped().is_some() || ip.to_ipv4().is_some() {
        return Err(Refusal::EmbeddedIpv4);
    }
    // 64:ff9b::/96, NAT64 — another IPv4 address wearing a hat.
    let s = ip.segments();
    if s[0] == 0x0064 && s[1] == 0xff9b {
        return Err(Refusal::EmbeddedIpv4);
    }

    // fc00::/7 unique local, fe80::/10 link-local.
    if (s[0] & 0xfe00) == 0xfc00 {
        return Err(Refusal::Private);
    }
    if (s[0] & 0xffc0) == 0xfe80 {
        return Err(Refusal::LinkLocal);
    }
    // 2001:db8::/32 documentation.
    if s[0] == 0x2001 && s[1] == 0x0db8 {
        return Err(Refusal::Reserved);
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn refuse(s: &str) -> Refusal {
        check(s.parse().expect("test address parses")).expect_err("should be refused")
    }

    fn allow(s: &str) -> IpAddr {
        check(s.parse().expect("test address parses"))
            .expect("should be allowed")
            .addr()
    }

    #[test]
    fn the_cloud_metadata_endpoint_is_refused() {
        // The single most valuable thing an SSRF can reach: on several providers this hands
        // out credentials to anything that asks. If only one test in this file survives,
        // this is the one.
        assert_eq!(refuse("169.254.169.254"), Refusal::LinkLocal);
    }

    #[test]
    fn loopback_is_refused() {
        assert_eq!(refuse("127.0.0.1"), Refusal::Loopback);
        // The whole /8 is loopback, and 127.0.0.1 is only its most famous member.
        assert_eq!(refuse("127.13.37.1"), Refusal::Loopback);
        assert_eq!(refuse("::1"), Refusal::Loopback);
    }

    #[test]
    fn every_private_range_is_refused() {
        assert_eq!(refuse("10.0.0.1"), Refusal::Private);
        assert_eq!(refuse("172.16.0.1"), Refusal::Private);
        assert_eq!(refuse("172.31.255.254"), Refusal::Private);
        assert_eq!(refuse("192.168.1.1"), Refusal::Private);
        assert_eq!(refuse("fc00::1"), Refusal::Private);
        assert_eq!(refuse("fd12:3456::1"), Refusal::Private);
    }

    #[test]
    fn the_edges_of_the_172_range_are_right() {
        // 172.16/12 is 172.16 through 172.31 — the range people get wrong by assuming
        // 172.16 through 172.16, or the whole of 172/8.
        assert_eq!(refuse("172.16.0.0"), Refusal::Private);
        assert_eq!(refuse("172.31.255.255"), Refusal::Private);
        allow("172.15.255.255");
        allow("172.32.0.0");
    }

    #[test]
    fn zero_is_refused_because_linux_treats_it_as_localhost() {
        // 0.0.0.0 connects to localhost on Linux, which makes it a loopback bypass that
        // does not look like one.
        assert_eq!(refuse("0.0.0.0"), Refusal::Unspecified);
        assert_eq!(refuse("0.1.2.3"), Refusal::Unspecified);
        assert_eq!(refuse("::"), Refusal::Unspecified);
    }

    #[test]
    fn carrier_grade_nat_is_refused() {
        assert_eq!(refuse("100.64.0.1"), Refusal::SharedAddressSpace);
        assert_eq!(refuse("100.127.255.255"), Refusal::SharedAddressSpace);
        // The boundaries: 100.63 and 100.128 are ordinary public space.
        allow("100.63.255.255");
        allow("100.128.0.0");
    }

    #[test]
    fn an_ipv6_address_wrapping_an_ipv4_one_is_refused() {
        // The classic bypass. The guard sees IPv6, the address is really 127.0.0.1, and a
        // stack that unwraps it connects to loopback.
        assert_eq!(refuse("::ffff:127.0.0.1"), Refusal::EmbeddedIpv4);
        assert_eq!(refuse("::ffff:169.254.169.254"), Refusal::EmbeddedIpv4);
        assert_eq!(refuse("64:ff9b::7f00:1"), Refusal::EmbeddedIpv4);
        // Refused even when the wrapped address is public: too many stacks disagree about
        // how to handle these for the exception to be worth its risk.
        assert_eq!(refuse("::ffff:8.8.8.8"), Refusal::EmbeddedIpv4);
    }

    #[test]
    fn link_local_and_multicast_are_refused() {
        assert_eq!(refuse("169.254.1.1"), Refusal::LinkLocal);
        assert_eq!(refuse("fe80::1"), Refusal::LinkLocal);
        assert_eq!(refuse("224.0.0.1"), Refusal::NotUnicast);
        assert_eq!(refuse("255.255.255.255"), Refusal::NotUnicast);
        assert_eq!(refuse("ff02::1"), Refusal::NotUnicast);
    }

    #[test]
    fn documentation_and_reserved_ranges_are_refused() {
        assert_eq!(refuse("192.0.2.1"), Refusal::Reserved);
        assert_eq!(refuse("198.51.100.1"), Refusal::Reserved);
        assert_eq!(refuse("203.0.113.1"), Refusal::Reserved);
        assert_eq!(refuse("198.18.0.1"), Refusal::Reserved);
        assert_eq!(refuse("240.0.0.1"), Refusal::Reserved);
        assert_eq!(refuse("2001:db8::1"), Refusal::Reserved);
    }

    #[test]
    fn ordinary_public_addresses_are_allowed() {
        // The guard is useless if it refuses everything, so the permitted case is asserted
        // as carefully as the refused ones.
        allow("8.8.8.8");
        allow("1.1.1.1");
        allow("93.184.216.34");
        allow("2606:2800:220:1:248:1893:25c8:1946");
    }

    #[test]
    fn the_verdict_carries_the_address_that_was_checked() {
        // The property the whole design rests on: a caller must be able to connect to the
        // address that was verified rather than re-resolving a name. A guard returning only
        // a boolean cannot be used safely, and this test is what stops it becoming one.
        let ip: IpAddr = "8.8.8.8".parse().expect("parses");
        assert_eq!(check(ip).expect("allowed").addr(), ip);
    }

    #[test]
    fn every_refusal_explains_itself_without_leaking_the_address() {
        // The message goes to a reader who may be probing us. It has to be useful to an
        // honest user and useless to a dishonest one.
        for r in [
            Refusal::Loopback,
            Refusal::Private,
            Refusal::LinkLocal,
            Refusal::SharedAddressSpace,
            Refusal::Unspecified,
            Refusal::NotUnicast,
            Refusal::Reserved,
            Refusal::EmbeddedIpv4,
        ] {
            let m = r.message();
            assert!(m.len() > 20, "{r:?} has no useful message");
            assert!(m.ends_with('.'), "{r:?} message is not a sentence");
            assert!(
                !m.contains("127.") && !m.contains("169.254") && !m.contains("10."),
                "{r:?} message leaks an address: {m}"
            );
        }
    }
}
