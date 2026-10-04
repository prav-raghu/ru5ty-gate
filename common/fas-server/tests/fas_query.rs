use ru5ty_gate_fas_server::FasQuery;

fn query(authaction: Option<&str>) -> FasQuery {
    FasQuery {
        clientip: "10.1.0.5".to_owned(),
        clientmac: "AA:BB:CC:DD:EE:FF".to_owned(),
        gatewayname: "gl-mt6000".to_owned(),
        gatewayaddress: "10.1.0.1".to_owned(),
        gatewayport: "2050".to_owned(),
        gatewaymac: None,
        originurl: "http://example.com/page?x=1".to_owned(),
        clientif: None,
        hid: "abc123hash".to_owned(),
        authaction: authaction.map(str::to_owned),
    }
}

#[test]
fn auth_redirect_url_is_built_from_the_gateway_when_no_authaction_is_supplied() {
    let url = query(None).auth_redirect_url();

    assert!(url.starts_with("http://10.1.0.1:2050/opennds_auth/?"));
    assert!(url.contains("tok=abc123hash"));
    assert!(url.contains("redir=http%3A%2F%2Fexample.com%2Fpage%3Fx%3D1"));
}

#[test]
fn auth_redirect_url_prefers_authaction_when_present() {
    let url = query(Some("http://10.1.0.1:2050/opennds_auth/?extra=1")).auth_redirect_url();

    assert!(url.starts_with("http://10.1.0.1:2050/opennds_auth/?extra=1&tok="));
}
