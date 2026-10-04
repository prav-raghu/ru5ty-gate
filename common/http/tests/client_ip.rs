#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::net::SocketAddr;

use axum::extract::ConnectInfo;
use axum::http::Request;
use axum::http::request::Parts;
use ru5ty_gate_http::{ClientIp, TrustedProxyHops};

fn parts(forwarded: &[&str], peer: Option<&str>, hops: Option<usize>) -> Parts {
    let mut builder = Request::builder().uri("/");
    for value in forwarded {
        builder = builder.header("x-forwarded-for", *value);
    }
    let mut request = builder.body(()).unwrap();
    if let Some(peer) = peer {
        request
            .extensions_mut()
            .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    }
    if let Some(hops) = hops {
        request.extensions_mut().insert(TrustedProxyHops(hops));
    }
    request.into_parts().0
}

#[test]
fn without_trusted_proxies_the_forwarded_header_is_ignored() {
    let parts = parts(&["6.6.6.6"], Some("203.0.113.9:4000"), None);

    assert_eq!(ClientIp::resolve(&parts), "203.0.113.9");
}

#[test]
fn zero_hops_always_resolves_the_direct_peer() {
    let parts = parts(&["6.6.6.6, 7.7.7.7"], Some("203.0.113.9:4000"), Some(0));

    assert_eq!(ClientIp::resolve(&parts), "203.0.113.9");
}

#[test]
fn one_hop_resolves_the_address_the_proxy_appended_not_the_spoofed_prefix() {
    let parts = parts(&["6.6.6.6, 198.51.100.7"], Some("10.0.0.2:4000"), Some(1));

    assert_eq!(ClientIp::resolve(&parts), "198.51.100.7");
}

#[test]
fn one_hop_without_a_forwarded_header_resolves_the_peer() {
    let parts = parts(&[], Some("10.0.0.2:4000"), Some(1));

    assert_eq!(ClientIp::resolve(&parts), "10.0.0.2");
}

#[test]
fn two_hops_skip_both_proxies() {
    let parts = parts(
        &["6.6.6.6, 198.51.100.7, 10.0.0.9"],
        Some("10.0.0.2:4000"),
        Some(2),
    );

    assert_eq!(ClientIp::resolve(&parts), "198.51.100.7");
}

#[test]
fn rotating_the_spoofed_prefix_never_changes_the_resolved_client() {
    let first = parts(&["1.1.1.1, 198.51.100.7"], Some("10.0.0.2:4000"), Some(1));
    let second = parts(&["2.2.2.2, 198.51.100.7"], Some("10.0.0.2:4000"), Some(1));

    assert_eq!(ClientIp::resolve(&first), ClientIp::resolve(&second));
}

#[test]
fn repeated_forwarded_headers_are_treated_as_one_chain() {
    let parts = parts(&["6.6.6.6", "198.51.100.7"], Some("10.0.0.2:4000"), Some(1));

    assert_eq!(ClientIp::resolve(&parts), "198.51.100.7");
}

#[test]
fn a_chain_shorter_than_the_trusted_hops_falls_back_to_its_first_entry() {
    let parts = parts(&["198.51.100.7"], None, Some(3));

    assert_eq!(ClientIp::resolve(&parts), "198.51.100.7");
}

#[test]
fn non_ip_values_and_missing_data_resolve_to_unknown() {
    let garbage = parts(&["not-an-ip"], None, Some(1));
    let nothing = parts(&[], None, Some(1));

    assert_eq!(ClientIp::resolve(&garbage), "unknown");
    assert_eq!(ClientIp::resolve(&nothing), "unknown");
}

#[test]
fn ipv4_mapped_ipv6_peers_are_normalised() {
    let parts = parts(&[], Some("[::ffff:203.0.113.9]:4000"), Some(0));

    assert_eq!(ClientIp::resolve(&parts), "203.0.113.9");
}
