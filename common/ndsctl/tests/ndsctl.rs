#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use ru5ty_gate_ndsctl::{Ndsctl, NdsctlError};
use tokio::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::const_new(());

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ru5ty-gate-ndsctl-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn script(dir: &std::path::Path, body: &str) -> String {
    let path = dir.join("ndsctl");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.to_str().unwrap().to_owned()
}

#[tokio::test]
async fn deauth_runs_the_program_with_the_deauth_subcommand_and_mac() {
    let _serial = SERIAL.lock().await;
    let dir = scratch_dir("ok");
    let log = dir.join("calls.log");
    let program = script(&dir, &format!("echo \"$@\" >> {}", log.display()));

    Ndsctl::with_program(program)
        .deauth("aa:bb:cc:dd:ee:ff")
        .await
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(&log).unwrap().trim(),
        "deauth aa:bb:cc:dd:ee:ff"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn deauth_reports_a_failing_exit_status() {
    let _serial = SERIAL.lock().await;
    let dir = scratch_dir("fail");
    let program = script(&dir, "echo nope >&2\nexit 3");

    let error = Ndsctl::with_program(program)
        .deauth("aa:bb:cc:dd:ee:ff")
        .await
        .unwrap_err();

    assert!(matches!(error, NdsctlError::Failed { code: Some(3), .. }));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn deauth_refuses_anything_that_is_not_a_mac_address() {
    let _serial = SERIAL.lock().await;
    let dir = scratch_dir("invalid");
    let program = script(&dir, "exit 0");

    for bad in [
        "",
        "aa:bb",
        "; rm -rf /",
        "aa:bb:cc:dd:ee:gg",
        "aa:bb:cc:dd:ee:ff:00",
    ] {
        let error = Ndsctl::with_program(program.clone())
            .deauth(bad)
            .await
            .unwrap_err();
        assert!(matches!(error, NdsctlError::InvalidMac));
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn disabled_runner_does_nothing() {
    let runner = Ndsctl::disabled();

    assert!(!runner.is_enabled());
    assert!(runner.deauth("not even a mac").await.is_ok());
}

#[tokio::test]
async fn deauth_reports_a_missing_program() {
    let error = Ndsctl::with_program("/nonexistent/ndsctl")
        .deauth("aa:bb:cc:dd:ee:ff")
        .await
        .unwrap_err();

    assert!(matches!(error, NdsctlError::Spawn(_)));
}
