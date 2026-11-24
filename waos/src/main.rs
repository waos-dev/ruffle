//! Ruffle as a WaOS program: it plays one movie in a window of WaOS.
//!
//! A WaOS program is a WASI component, so the player ([`program`]) is built for WASI alone. On
//! any other target this crate is a binary that only says so, which lets the workspace's checks on
//! the host pass over it.

#[cfg(target_os = "wasi")]
mod program;

#[cfg(target_os = "wasi")]
#[wasdk::main]
async fn main() -> Result<(), String> {
    program::run().await
}

#[cfg(not(target_os = "wasi"))]
fn main() -> std::process::ExitCode {
    eprintln!("ruffle_waos is a WaOS program: build it for wasm32-wasip3, as WAOS.md says");
    std::process::ExitCode::FAILURE
}
