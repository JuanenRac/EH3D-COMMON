//! Finding the server on the local network with multicast DNS (feature `mdns`, off by default).
//!
//! A small client of its own over UDP, with no dependency: it asks who offers `_eh3d._tcp.local` and reads the answers. It is a *one-shot* query
//! as a program that is not a responder makes it (RFC 6762, section 5.1 and 6.7): the question goes from a port that is not 5353, so the servers answer
//! to that port directly and nothing has to join the multicast group. The limits are fixed: the whole search lasts at most ten seconds, at most
//! `max_packets` answers are read, a packet is at most 9000 bytes, a name at most 255 bytes, a compression pointer is followed at most sixteen times and a
//! packet is read for at most 128 records. Only what the network says is reported: a server needs a pointer record to the service **and** a service
//! record with a port; the address is the one of an address record when there is one, else the name of the host that the service record gives.
//!
//! What it finds is a *candidate*: the program shows it to the person, who accepts it; nothing connects by itself and nothing is trusted because it
//! answered on the network (any device of the network can answer).

use crate::client::Discover;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

/// The service the server announces.
pub const SERVICE: &str = "_eh3d._tcp.local";
/// The multicast group and port of mDNS.
pub const GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
pub const PORT: u16 = 5353;
/// The longest search: `find` never waits more than this.
pub const MAX_WAIT: Duration = Duration::from_secs(10);

const MAX_PACKET: usize = 9000;
const MAX_RECORDS: usize = 128;
const MAX_NAME: usize = 255;
const MAX_JUMPS: usize = 16;
const MAX_KEPT: usize = 32;
const MAX_ADDRS: usize = 64;

const T_A: u16 = 1;
const T_PTR: u16 = 12;
const T_SRV: u16 = 33;
const T_AAAA: u16 = 28;
const CLASS_IN: u16 = 1;

fn encode_name(name: &str, out: &mut Vec<u8>) -> Result<(), String> {
    let name = name.trim_end_matches('.');
    if name.len() > MAX_NAME {
        return Err("the name is too long".to_string());
    }
    for label in name.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(format!("a label of {name:?} is empty or longer than 63"));
        }
        out.push(label.len() as u8);
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
    Ok(())
}

/// The question "who offers `service`?" as a packet: a header with one question of type PTR, class IN.
pub fn build_query(service: &str) -> Result<Vec<u8>, String> {
    let mut p = vec![0u8; 12];
    p[5] = 1; // one question
    encode_name(service, &mut p)?;
    p.extend_from_slice(&T_PTR.to_be_bytes());
    p.extend_from_slice(&CLASS_IN.to_be_bytes());
    Ok(p)
}

/// An answer as a responder gives it: the pointer record of `service`, and the service record and the address record of the host in the same packet.
/// It exists for the tests of the programs and for simulators of a server; it is not the announcement of the real one.
pub fn build_response(
    service: &str,
    instance: &str,
    host: &str,
    port: u16,
    addr: Ipv4Addr,
    ttl: u32,
) -> Result<Vec<u8>, String> {
    let full = format!("{instance}.{service}");
    let mut p = vec![0u8; 12];
    p[2] = 0x84; // a response, authoritative
    p[7] = 1; // one answer
    p[11] = 2; // two additional records
    let record = |p: &mut Vec<u8>, owner: &str, kind: u16, rdata: &[u8]| -> Result<(), String> {
        encode_name(owner, p)?;
        p.extend_from_slice(&kind.to_be_bytes());
        p.extend_from_slice(&CLASS_IN.to_be_bytes());
        p.extend_from_slice(&ttl.to_be_bytes());
        p.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
        p.extend_from_slice(rdata);
        Ok(())
    };
    let mut ptr = Vec::new();
    encode_name(&full, &mut ptr)?;
    record(&mut p, service, T_PTR, &ptr)?;
    let mut srv = vec![0, 0, 0, 0];
    srv.extend_from_slice(&port.to_be_bytes());
    encode_name(host, &mut srv)?;
    record(&mut p, &full, T_SRV, &srv)?;
    record(&mut p, host, T_A, &addr.octets())?;
    Ok(p)
}

/// Reads a name that may be compressed. Gives the name (labels joined with dots) and the offset just after it where it was read.
fn read_name(p: &[u8], start: usize) -> Option<(String, usize)> {
    let mut pos = start;
    let mut after = None;
    let mut jumps = 0;
    let mut name = String::new();
    loop {
        let len = usize::from(*p.get(pos)?);
        if len == 0 {
            pos += 1;
            break;
        }
        match len & 0xC0 {
            0 => {
                let label = p.get(pos + 1..pos + 1 + len)?;
                if name.len() + len + 1 > MAX_NAME {
                    return None;
                }
                if !name.is_empty() {
                    name.push('.');
                }
                name.push_str(std::str::from_utf8(label).ok()?);
                pos += 1 + len;
            }
            0xC0 => {
                let low = usize::from(*p.get(pos + 1)?);
                if after.is_none() {
                    after = Some(pos + 2);
                }
                jumps += 1;
                if jumps > MAX_JUMPS {
                    return None;
                }
                let target = ((len & 0x3F) << 8) | low;
                if target >= p.len() {
                    return None;
                }
                pos = target;
            }
            _ => return None,
        }
    }
    Some((name, after.unwrap_or(pos)))
}

fn u16_at(p: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*p.get(at)?, *p.get(at + 1)?]))
}

/// One server that answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The name the owner gave it (`EH3D Server on cm5`), without the service.
    pub instance: String,
    /// The host that the service record names (`cm5.local`).
    pub host: String,
    pub port: u16,
    /// The addresses of the host that came with the answers.
    pub addrs: Vec<IpAddr>,
}

fn usable(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(a) => !(a.is_unspecified() || a.is_multicast() || a.is_broadcast()),
        // a link-local IPv6 address cannot be used without the interface it belongs to
        IpAddr::V6(a) => {
            !(a.is_unspecified() || a.is_multicast() || (a.segments()[0] & 0xffc0) == 0xfe80)
        }
    }
}

impl Found {
    /// `host:port` as the settings take it: the first usable IPv4 address, else the first usable IPv6 one (in brackets), else the name of the host.
    pub fn address(&self) -> String {
        let v4 = self.addrs.iter().find(|a| a.is_ipv4() && usable(a));
        let v6 = self.addrs.iter().find(|a| a.is_ipv6() && usable(a));
        match (v4, v6) {
            (Some(a), _) => format!("{a}:{}", self.port),
            (None, Some(a)) => format!("[{a}]:{}", self.port),
            _ => format!("{}:{}", self.host, self.port),
        }
    }
}

/// What several packets have said so far; the packets of one answer may come apart (the pointer in one, the service record in another).
#[derive(Debug, Default, Clone)]
pub struct Collected {
    ptrs: Vec<String>,
    srvs: Vec<(String, String, u16)>,
    addrs: Vec<(String, IpAddr)>,
}

impl Collected {
    /// Reads one packet. A packet that is not a response, is cut short or is hostile adds nothing (or only what was read before the damage).
    pub fn absorb(&mut self, p: &[u8], service: &str) {
        let Some(flags) = u16_at(p, 2) else {
            return;
        };
        if flags & 0x8000 == 0 || flags & 0x000F != 0 {
            return;
        }
        let (Some(qd), Some(an), Some(ns), Some(ar)) =
            (u16_at(p, 4), u16_at(p, 6), u16_at(p, 8), u16_at(p, 10))
        else {
            return;
        };
        let mut pos = 12;
        for _ in 0..qd.min(16) {
            let Some((_, after)) = read_name(p, pos) else {
                return;
            };
            pos = after + 4;
        }
        let total = (usize::from(an) + usize::from(ns) + usize::from(ar)).min(MAX_RECORDS);
        for _ in 0..total {
            let Some((owner, after)) = read_name(p, pos) else {
                return;
            };
            let (Some(kind), Some(class), Some(rdlen)) =
                (u16_at(p, after), u16_at(p, after + 2), u16_at(p, after + 8))
            else {
                return;
            };
            let Some(ttl) = p
                .get(after + 4..after + 8)
                .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
            else {
                return;
            };
            let start = after + 10;
            let end = start + usize::from(rdlen);
            let Some(rdata) = p.get(start..end) else {
                return;
            };
            pos = end;
            // a record of ttl 0 says the service is gone
            if class & 0x7FFF != CLASS_IN || ttl == 0 {
                continue;
            }
            match kind {
                T_PTR if owner.eq_ignore_ascii_case(service) => {
                    if let Some((name, stop)) = read_name(p, start) {
                        if stop <= end && self.ptrs.len() < MAX_KEPT && !self.ptrs.contains(&name) {
                            self.ptrs.push(name);
                        }
                    }
                }
                T_SRV if rdata.len() >= 7 => {
                    let port = u16::from_be_bytes([rdata[4], rdata[5]]);
                    if let Some((target, stop)) = read_name(p, start + 6) {
                        if stop <= end
                            && port != 0
                            && !target.is_empty()
                            && self.srvs.len() < MAX_KEPT
                        {
                            self.srvs.push((owner, target, port));
                        }
                    }
                }
                T_A if rdata.len() == 4 && self.addrs.len() < MAX_ADDRS => {
                    let a = Ipv4Addr::new(rdata[0], rdata[1], rdata[2], rdata[3]);
                    self.addrs.push((owner, IpAddr::V4(a)));
                }
                T_AAAA if rdata.len() == 16 && self.addrs.len() < MAX_ADDRS => {
                    let mut octets = [0u8; 16];
                    octets.copy_from_slice(rdata);
                    self.addrs.push((owner, IpAddr::V6(Ipv6Addr::from(octets))));
                }
                _ => {}
            }
        }
    }

    /// The servers that have both a pointer and a service record, in the order they were first heard.
    pub fn found(&self, service: &str) -> Vec<Found> {
        let suffix = format!(".{service}");
        let mut out: Vec<Found> = Vec::new();
        for full in &self.ptrs {
            let Some((_, host, port)) = self
                .srvs
                .iter()
                .find(|(owner, _, _)| owner.eq_ignore_ascii_case(full))
            else {
                continue;
            };
            let instance = full
                .strip_suffix(&suffix)
                .unwrap_or(full.as_str())
                .to_string();
            let mut addrs: Vec<IpAddr> = Vec::new();
            for (owner, ip) in &self.addrs {
                if owner.eq_ignore_ascii_case(host) && !addrs.contains(ip) {
                    addrs.push(*ip);
                }
            }
            let one = Found {
                instance,
                host: host.clone(),
                port: *port,
                addrs,
            };
            if !out.iter().any(|f| f.address() == one.address()) {
                out.push(one);
            }
        }
        out
    }
}

/// The servers a single packet announces.
pub fn parse_response(packet: &[u8], service: &str) -> Vec<Found> {
    let mut c = Collected::default();
    c.absorb(packet, service);
    c.found(service)
}

/// The resolver: gives it to a client with [`crate::Client::with_discovery`] (or [`crate::Client::with_mdns`]); it works only when `server_mdns` is on.
#[derive(Debug, Clone)]
pub struct MdnsResolver {
    service: String,
    target: SocketAddr,
    bind: SocketAddr,
    max_packets: usize,
}

impl Default for MdnsResolver {
    fn default() -> MdnsResolver {
        MdnsResolver {
            service: SERVICE.to_string(),
            target: SocketAddr::new(IpAddr::V4(GROUP), PORT),
            bind: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            max_packets: 64,
        }
    }
}

impl MdnsResolver {
    /// Another service name (a test, another product of the suite).
    pub fn with_service(mut self, service: &str) -> MdnsResolver {
        self.service = service.to_string();
        self
    }

    /// Where the question goes (the multicast group by default; a test points it at a responder of its own).
    pub fn with_target(mut self, target: SocketAddr) -> MdnsResolver {
        self.target = target;
        self
    }

    /// The local address the question leaves from (any, with a port the system picks, by default).
    pub fn with_bind(mut self, bind: SocketAddr) -> MdnsResolver {
        self.bind = bind;
        self
    }

    /// The most packets read in one search (at least 1).
    pub fn with_max_packets(mut self, n: usize) -> MdnsResolver {
        self.max_packets = n.max(1);
        self
    }

    /// The servers that answered within `within` (never more than [`MAX_WAIT`]), with their addresses.
    pub fn search(&self, within: Duration) -> Vec<Found> {
        let within = within.min(MAX_WAIT);
        let Ok(query) = build_query(&self.service) else {
            return Vec::new();
        };
        let Ok(socket) = UdpSocket::bind(self.bind) else {
            return Vec::new();
        };
        let _ = socket.set_multicast_ttl_v4(255);
        let started = Instant::now();
        let mut collected = Collected::default();
        let mut last_sent: Option<Instant> = None;
        let mut packets = 0;
        let mut buf = vec![0u8; MAX_PACKET];
        while started.elapsed() < within {
            if last_sent.is_none_or(|t| t.elapsed() >= Duration::from_secs(1)) {
                let _ = socket.send_to(&query, self.target);
                last_sent = Some(Instant::now());
            }
            let left = within.saturating_sub(started.elapsed());
            let slice = left
                .min(Duration::from_millis(250))
                .max(Duration::from_millis(1));
            if socket.set_read_timeout(Some(slice)).is_err() {
                break;
            }
            match socket.recv_from(&mut buf) {
                Ok((n, _)) => {
                    packets += 1;
                    collected.absorb(&buf[..n], &self.service);
                    if packets >= self.max_packets {
                        break;
                    }
                    let found = collected.found(&self.service);
                    if !found.is_empty() && found.iter().all(|f| !f.addrs.is_empty()) {
                        break;
                    }
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                    // Windows reports an unreachable target this way; the next question is sent a second later
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(_) => break,
            }
        }
        collected.found(&self.service)
    }
}

impl Discover for MdnsResolver {
    fn find(&self, within: Duration) -> Vec<String> {
        let mut list: Vec<String> = self.search(within).iter().map(Found::address).collect();
        list.sort();
        list.dedup();
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(ttl: u32) -> Vec<u8> {
        build_response(
            SERVICE,
            "EH3D Server on cm5",
            "cm5.local",
            18200,
            Ipv4Addr::new(192, 168, 0, 180),
            ttl,
        )
        .unwrap()
    }

    #[test]
    fn the_question_is_a_ptr_in_class_in_for_the_service() {
        let q = build_query(SERVICE).unwrap();
        assert_eq!(&q[..12], &[0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&q[12..13], &[5]);
        assert_eq!(&q[13..18], b"_eh3d");
        assert_eq!(&q[q.len() - 4..], &[0, 12, 0, 1]);
        assert!(build_query("a..b").is_err());
        assert!(build_query(&"a".repeat(64)).is_err());
    }

    #[test]
    fn an_answer_gives_the_address_and_the_port() {
        let found = parse_response(&packet(120), SERVICE);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].instance, "EH3D Server on cm5");
        assert_eq!(found[0].host, "cm5.local");
        assert_eq!(found[0].port, 18200);
        assert_eq!(found[0].address(), "192.168.0.180:18200");
    }

    #[test]
    fn without_an_address_record_the_name_of_the_host_is_given_not_an_invented_address() {
        let found = Found {
            instance: "x".to_string(),
            host: "cm5.local".to_string(),
            port: 18200,
            addrs: vec![],
        };
        assert_eq!(found.address(), "cm5.local:18200");
    }

    #[test]
    fn addresses_that_cannot_be_used_are_skipped() {
        let f = |addrs: Vec<IpAddr>| Found {
            instance: String::new(),
            host: "h.local".to_string(),
            port: 1,
            addrs,
        };
        let any: IpAddr = Ipv4Addr::UNSPECIFIED.into();
        let link_local: IpAddr = "fe80::1".parse::<Ipv6Addr>().unwrap().into();
        let global: IpAddr = "2001:db8::7".parse::<Ipv6Addr>().unwrap().into();
        let lan: IpAddr = Ipv4Addr::new(10, 0, 0, 2).into();
        assert_eq!(f(vec![any, link_local]).address(), "h.local:1");
        assert_eq!(f(vec![link_local, global]).address(), "[2001:db8::7]:1");
        assert_eq!(f(vec![global, lan]).address(), "10.0.0.2:1");
    }

    #[test]
    fn a_goodbye_a_question_and_an_error_say_nothing() {
        assert!(parse_response(&packet(0), SERVICE).is_empty());
        let mut question = packet(120);
        question[2] = 0; // not a response
        assert!(parse_response(&question, SERVICE).is_empty());
        let mut refused = packet(120);
        refused[3] = 3; // an error code
        assert!(parse_response(&refused, SERVICE).is_empty());
    }

    #[test]
    fn another_service_is_not_ours() {
        assert!(parse_response(&packet(120), "_other._tcp.local").is_empty());
        // the owner of the pointer is compared without regard to case
        assert_eq!(parse_response(&packet(120), "_EH3D._TCP.LOCAL").len(), 1);
    }

    #[test]
    fn a_service_record_without_a_port_is_not_a_server() {
        let found = parse_response(
            &build_response(SERVICE, "x", "h.local", 0, Ipv4Addr::new(10, 0, 0, 1), 120).unwrap(),
            SERVICE,
        );
        assert!(found.is_empty());
    }

    #[test]
    fn a_packet_cut_anywhere_never_panics() {
        let whole = packet(120);
        for n in 0..whole.len() {
            let _ = parse_response(&whole[..n], SERVICE);
        }
        assert!(parse_response(&whole[..whole.len() - 1], SERVICE).len() <= 1);
    }

    #[test]
    fn a_pointer_that_loops_or_points_outside_is_refused() {
        // a question whose name is a pointer to itself
        let mut p = vec![0u8; 12];
        p[2] = 0x84;
        p[5] = 1;
        p.extend_from_slice(&[0xC0, 12, 0, 12, 0, 1]);
        assert!(parse_response(&p, SERVICE).is_empty());
        assert!(read_name(&[0xC0, 0xFF], 0).is_none());
        assert!(read_name(&[0xC0, 0], 0).is_none());
        // a label that claims more bytes than there are
        assert!(read_name(&[9, b'a'], 0).is_none());
        // the reserved label types
        assert!(read_name(&[0x40, 0], 0).is_none());
    }

    #[test]
    fn a_name_is_limited_to_255_bytes() {
        let mut name = Vec::new();
        for _ in 0..5 {
            name.push(63);
            name.extend_from_slice(&[b'a'; 63]);
        }
        name.push(0);
        assert!(read_name(&name, 0).is_none());
        assert!(encode_name(&"a".repeat(300), &mut Vec::new()).is_err());
    }

    #[test]
    fn a_packet_that_claims_a_flood_of_records_is_read_up_to_the_limit() {
        let mut p = packet(120);
        p[6] = 0xFF;
        p[7] = 0xFF; // 65535 answers claimed; the first three are real
        let found = parse_response(&p, SERVICE);
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn the_packets_of_one_answer_may_come_apart() {
        let whole = packet(120);
        let mut c = Collected::default();
        c.absorb(&whole, SERVICE);
        c.absorb(&whole, SERVICE);
        // the same server heard twice is one
        assert_eq!(c.found(SERVICE).len(), 1);
    }

    #[test]
    fn the_resolver_finds_a_responder_on_the_loopback() {
        let responder = UdpSocket::bind("127.0.0.1:0").unwrap();
        let target = responder.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let mut buf = [0u8; 512];
            responder
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let (n, from) = responder.recv_from(&mut buf).unwrap();
            assert_eq!(&buf[..n], &build_query(SERVICE).unwrap()[..]);
            responder.send_to(&packet(120), from).unwrap();
        });
        let resolver = MdnsResolver::default()
            .with_target(target)
            .with_bind("127.0.0.1:0".parse().unwrap());
        let list = resolver.find(Duration::from_secs(5));
        handle.join().unwrap();
        assert_eq!(list, vec!["192.168.0.180:18200".to_string()]);
    }

    #[test]
    fn the_resolver_gives_up_when_nobody_answers() {
        let silent = UdpSocket::bind("127.0.0.1:0").unwrap();
        let resolver = MdnsResolver::default()
            .with_target(silent.local_addr().unwrap())
            .with_bind("127.0.0.1:0".parse().unwrap());
        let started = Instant::now();
        assert!(resolver.find(Duration::from_millis(300)).is_empty());
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(resolver.find(Duration::ZERO).is_empty());
    }

    #[test]
    fn the_wait_is_never_longer_than_ten_seconds() {
        assert_eq!(MAX_WAIT, Duration::from_secs(10));
        assert_eq!(MdnsResolver::default().with_max_packets(0).max_packets, 1);
    }
}
