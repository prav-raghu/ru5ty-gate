#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ru5ty_gate_fas_server::{FasError, FasPayloadDecoder, FasToken};

fn encode(fields: &str) -> String {
    STANDARD.encode(fields)
}

const VALID: &str = "clientip=10.1.0.5, clientmac=AA:BB:CC:DD:EE:FF, gatewayname=gl-mt6000, \
    version=10.2.0, hid=abc123hash, gatewayaddress=10.1.0.1:2050, authdir=opennds_auth, \
    originurl=http%3A%2F%2Fexample.com%2Fpage%3Fx%3D1, clientif=br-lan, client_type=cpd";

fn without(field: &str) -> String {
    VALID
        .split(", ")
        .filter(|pair| !pair.starts_with(&format!("{field}=")))
        .collect::<Vec<_>>()
        .join(", ")
}

fn replacing(field: &str, value: &str) -> String {
    VALID
        .split(", ")
        .map(|pair| {
            if pair.starts_with(&format!("{field}=")) {
                format!("{field}={value}")
            } else {
                pair.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[test]
fn decode_reads_an_openn_ds_level_one_payload() {
    let payload = FasPayloadDecoder::decode(&encode(VALID)).unwrap();

    assert_eq!(payload.clientip, "10.1.0.5");
    assert_eq!(payload.clientmac, "aa:bb:cc:dd:ee:ff");
    assert_eq!(payload.gatewayname, "gl-mt6000");
    assert_eq!(payload.hid, "abc123hash");
    assert_eq!(payload.gatewayaddress, "10.1.0.1:2050");
    assert_eq!(payload.authdir, "opennds_auth");
    assert_eq!(
        payload.originurl.as_deref(),
        Some("http://example.com/page?x=1")
    );
    assert_eq!(payload.clientif.as_deref(), Some("br-lan"));
}

#[test]
fn decode_tolerates_plus_signs_that_arrive_as_spaces() {
    let encoded = (0..64)
        .map(|padding| encode(&format!("{VALID}, x={}", ">".repeat(padding))))
        .find(|candidate| candidate.contains('+'))
        .unwrap();

    let payload = FasPayloadDecoder::decode(&encoded.replace('+', " ")).unwrap();

    assert_eq!(payload.hid, "abc123hash");
}

#[test]
fn decode_keeps_the_first_occurrence_of_a_repeated_field() {
    let doubled = format!("{VALID}, hid=second");

    let payload = FasPayloadDecoder::decode(&encode(&doubled)).unwrap();

    assert_eq!(payload.hid, "abc123hash");
}

#[test]
fn decode_rejects_empty_oversized_and_malformed_input() {
    assert_eq!(
        FasPayloadDecoder::decode("").unwrap_err(),
        FasError::MissingPayload
    );
    assert_eq!(
        FasPayloadDecoder::decode("%%%not-base64%%%").unwrap_err(),
        FasError::MalformedPayload
    );
    assert_eq!(
        FasPayloadDecoder::decode(&"A".repeat(9000)).unwrap_err(),
        FasError::MalformedPayload
    );
    assert_eq!(
        FasPayloadDecoder::decode(&STANDARD.encode([0xff, 0xfe, 0xfd])).unwrap_err(),
        FasError::MalformedPayload
    );
}

#[test]
fn decode_requires_every_mandatory_field() {
    for field in [
        "clientip",
        "clientmac",
        "hid",
        "gatewayaddress",
        "authdir",
        "gatewayname",
    ] {
        let error = FasPayloadDecoder::decode(&encode(&without(field))).unwrap_err();

        assert_eq!(
            error,
            FasError::InvalidField(match field {
                "clientip" => "clientip",
                "clientmac" => "clientmac",
                "hid" => "hid",
                "gatewayaddress" => "gatewayaddress",
                "authdir" => "authdir",
                _ => "gatewayname",
            })
        );
    }
}

#[test]
fn decode_rejects_values_that_could_be_used_for_injection() {
    let cases = [
        ("clientmac", "AA:BB:CC:DD:EE", "clientmac"),
        ("clientmac", "zz:bb:cc:dd:ee:ff", "clientmac"),
        ("clientmac", "aa:bb:cc:dd:ee:ff:00", "clientmac"),
        ("clientip", "not-an-ip", "clientip"),
        ("hid", "abc/../def", "hid"),
        ("authdir", "..%2Fadmin", "authdir"),
        ("authdir", "a/b", "authdir"),
        ("gatewayaddress", "evil.example/x:80", "gatewayaddress"),
        ("gatewayaddress", "evil.example:80", "gatewayaddress"),
        ("gatewayaddress", "10.0.0.1", "gatewayaddress"),
    ];
    for (field, value, expected) in cases {
        let error = FasPayloadDecoder::decode(&encode(&replacing(field, value))).unwrap_err();

        assert_eq!(error, FasError::InvalidField(expected), "{field}={value}");
    }
}

#[test]
fn decode_rejects_overlong_fields() {
    let long_hid = "a".repeat(129);
    let long_name = "g".repeat(129);

    assert!(FasPayloadDecoder::decode(&encode(&replacing("hid", &long_hid))).is_err());
    assert!(FasPayloadDecoder::decode(&encode(&replacing("gatewayname", &long_name))).is_err());
}

#[test]
fn decode_drops_an_origin_url_that_is_not_http_or_https() {
    let payload =
        FasPayloadDecoder::decode(&encode(&replacing("originurl", "javascript%3Aalert(1)")))
            .unwrap();

    assert!(payload.originurl.is_none());
    let long = format!("http%3A%2F%2Fexample.com%2F{}", "a".repeat(3000));
    let payload = FasPayloadDecoder::decode(&encode(&replacing("originurl", &long))).unwrap();
    assert!(payload.originurl.is_none());
}

#[test]
fn return_token_is_the_sha256_of_the_hid_followed_by_the_faskey() {
    assert_eq!(
        FasToken::return_token("abc", "secret"),
        "a42178b773273f5c9f24387fbea546af537d08b8c06b23631e44878b9ce47f49"
    );
    assert_ne!(
        FasToken::return_token("abc", "secret"),
        FasToken::return_token("secret", "abc")
    );
}
