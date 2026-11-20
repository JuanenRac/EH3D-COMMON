//! The resolver of the feature `mdns` through the client, and the reader of packets on hostile input made by a generator with a fixed seed.
#![cfg(feature = "mdns")]

use eh3d_serverclient::mdns::{build_response, parse_response, MdnsResolver, SERVICE};
use eh3d_serverclient::sim::SimServer;
use eh3d_serverclient::{Client, ClientError, Settings};
use std::net::{Ipv4Addr, UdpSocket};
use std::time::Duration;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn answer() -> Vec<u8> {
    build_response(
        SERVICE,
        "EH3D Server on cm5",
        "cm5.local",
        18200,
        Ipv4Addr::new(192, 168, 0, 180),
        120,
    )
    .unwrap()
}

fn settings(mdns: &str) -> Settings {
    let mut s = Settings::default();
    s.set("server_mdns", mdns).unwrap();
    s
}

/// A responder on the loopback that answers one question with `packet`.
fn responder(packet: Vec<u8>) -> (std::net::SocketAddr, std::thread::JoinHandle<()>) {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let addr = socket.local_addr().unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let handle = std::thread::spawn(move || {
        let mut buf = [0u8; 512];
        if let Ok((_, from)) = socket.recv_from(&mut buf) {
            let _ = socket.send_to(&packet, from);
        }
    });
    (addr, handle)
}

#[test]
fn with_the_key_off_the_client_asks_nothing_even_with_the_resolver_given() {
    let (addr, handle) = responder(answer());
    let resolver = MdnsResolver::default()
        .with_target(addr)
        .with_bind("127.0.0.1:0".parse().unwrap());
    let c =
        Client::new(settings("off"), Box::new(SimServer::new())).with_discovery(Box::new(resolver));
    assert!(matches!(c.discover(), Err(ClientError::Config(_))));
    // the responder was never asked: unblock its wait by asking it ourselves
    let poke = UdpSocket::bind("127.0.0.1:0").unwrap();
    poke.send_to(&[0], addr).unwrap();
    handle.join().unwrap();
}

#[test]
fn with_the_key_on_the_client_gives_the_addresses_found() {
    let (addr, handle) = responder(answer());
    let resolver = MdnsResolver::default()
        .with_target(addr)
        .with_bind("127.0.0.1:0".parse().unwrap());
    let c =
        Client::new(settings("on"), Box::new(SimServer::new())).with_discovery(Box::new(resolver));
    assert_eq!(
        c.discover().unwrap(),
        vec!["192.168.0.180:18200".to_string()]
    );
    handle.join().unwrap();
}

#[test]
fn with_mdns_gives_the_resolver_of_the_crate_and_the_key_still_rules() {
    let off = Client::new(settings("off"), Box::new(SimServer::new())).with_mdns();
    assert!(matches!(off.discover(), Err(ClientError::Config(_))));
}

#[test]
fn a_hostile_answer_from_the_network_gives_nothing_and_does_not_stop_the_search() {
    // a pointer loop, then the real answer: the loop is ignored, the answer is found
    let mut evil = vec![0u8; 12];
    evil[2] = 0x84;
    evil[5] = 1;
    evil.extend_from_slice(&[0xC0, 12, 0, 12, 0, 1]);
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let addr = socket.local_addr().unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let good = answer();
    let handle = std::thread::spawn(move || {
        let mut buf = [0u8; 512];
        if let Ok((_, from)) = socket.recv_from(&mut buf) {
            let _ = socket.send_to(&evil, from);
            let _ = socket.send_to(&good, from);
        }
    });
    let resolver = MdnsResolver::default()
        .with_target(addr)
        .with_bind("127.0.0.1:0".parse().unwrap());
    let c =
        Client::new(settings("on"), Box::new(SimServer::new())).with_discovery(Box::new(resolver));
    assert_eq!(
        c.discover().unwrap(),
        vec!["192.168.0.180:18200".to_string()]
    );
    handle.join().unwrap();
}

#[test]
fn random_and_damaged_packets_never_panic_and_only_give_whole_servers() {
    let valid = answer();
    let mut rng = Rng(0x5EED_00AA);
    for round in 0..6000 {
        let mut p = if round % 2 == 0 {
            valid.clone()
        } else {
            (0..rng.below(200)).map(|_| rng.next() as u8).collect()
        };
        if round % 2 == 0 {
            for _ in 0..=rng.below(4) {
                if p.is_empty() {
                    break;
                }
                let at = rng.below(p.len());
                match rng.below(3) {
                    0 => p[at] = rng.next() as u8,
                    1 => {
                        p.remove(at);
                    }
                    _ => p.insert(at, rng.next() as u8),
                }
            }
        }
        for found in parse_response(&p, SERVICE) {
            assert!(found.port != 0, "{p:?}");
            assert!(!found.host.is_empty(), "{p:?}");
            assert!(found.address().contains(':'));
            assert!(found.host.len() <= 255 && found.instance.len() <= 255);
        }
    }
}

#[test]
fn the_search_is_bounded_in_time_whatever_it_is_asked() {
    let silent = UdpSocket::bind("127.0.0.1:0").unwrap();
    let resolver = MdnsResolver::default()
        .with_target(silent.local_addr().unwrap())
        .with_bind("127.0.0.1:0".parse().unwrap());
    let started = std::time::Instant::now();
    assert!(eh3d_serverclient::Discover::find(&resolver, Duration::from_millis(200)).is_empty());
    assert!(started.elapsed() < Duration::from_secs(3));
}
