use ru5ty_gate_http::ServerInfo;

#[test]
fn environment_reflects_the_production_flag() {
    assert_eq!(
        ServerInfo::new("admin-api", "1.2.3", 4001, false).environment(),
        "development"
    );
    assert_eq!(
        ServerInfo::new("admin-api", "1.2.3", 4001, true).environment(),
        "production"
    );
}

#[test]
fn local_url_uses_the_configured_port() {
    let info = ServerInfo::new("customer-api", "1.2.3", 4002, false);

    assert_eq!(info.local_url(), "http://localhost:4002");
}

#[test]
fn docs_url_is_absent_when_the_service_has_no_docs() {
    let info = ServerInfo::new("schedule-api", "1.2.3", 4003, false);

    assert_eq!(info.docs_url(), None);
}

#[test]
fn docs_url_is_built_from_the_port_and_docs_path_in_development() {
    let info = ServerInfo::new("admin-api", "1.2.3", 4001, false).with_docs("/docs");

    assert_eq!(
        info.docs_url().as_deref(),
        Some("http://localhost:4001/docs")
    );
}

#[test]
fn docs_url_is_never_exposed_in_production() {
    let info = ServerInfo::new("admin-api", "1.2.3", 4001, true).with_docs("/docs");

    assert_eq!(info.docs_url(), None);
}
