use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    println!("cargo:rustc-env=RU5TY_GATE_BUILD_UNIX={now}");
}
