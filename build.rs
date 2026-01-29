//! Build script for the anyhow crate.
//!
//! Performs compile-time feature detection to enable optimal error handling
//! based on compiler capabilities. This script probes for unstable features
//! and emits appropriate `rustc-cfg` flags for conditional compilation.
//!
//! # Detected Features
//!
//! - **`error_generic_member_access`**: Unstable feature enabling richer
//!   backtrace propagation via `std::error::Error::provide`.
//!
//! - **`std_backtrace`**: Always enabled with MSRV 1.93, as
//!   `std::backtrace::Backtrace` is stable since Rust 1.65.
//!
//! # Environment Variables
//!
//! - `RUSTC_BOOTSTRAP`: Allows stable/beta compilers to use unstable features.
//! - `RUSTC_STAGE`: Detected to avoid issues within rustc's bootstrap.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::iter;
use std::path::Path;
use std::process::{self, Command, Stdio};
use std::str;

/// Detects compiler features and emits `rustc-cfg` flags.
fn main() {
    if cfg!(feature = "std") {
        println!("cargo:rerun-if-changed=src/nightly.rs");

        let error_generic_member_access;
        let consider_rustc_bootstrap;
        if compile_probe(false) {
            // This is a nightly or dev compiler, so it supports unstable
            // features regardless of RUSTC_BOOTSTRAP. No need to rerun build
            // script if RUSTC_BOOTSTRAP is changed.
            error_generic_member_access = true;
            consider_rustc_bootstrap = false;
        } else if let Some(rustc_bootstrap) = env::var_os("RUSTC_BOOTSTRAP") {
            if compile_probe(true) {
                // This is a stable or beta compiler for which the user has set
                // RUSTC_BOOTSTRAP to turn on unstable features. Rerun build
                // script if they change it.
                error_generic_member_access = true;
                consider_rustc_bootstrap = true;
            } else if rustc_bootstrap == "1" {
                // This compiler does not support the generic member access API
                // in the form that anyhow expects. No need to pay attention to
                // RUSTC_BOOTSTRAP.
                error_generic_member_access = false;
                consider_rustc_bootstrap = false;
            } else {
                // This is a stable or beta compiler for which RUSTC_BOOTSTRAP
                // is set to restrict the use of unstable features by this
                // crate.
                error_generic_member_access = false;
                consider_rustc_bootstrap = true;
            }
        } else {
            // Without RUSTC_BOOTSTRAP, this compiler does not support the
            // generic member access API in the form that anyhow expects, but
            // try again if the user turns on unstable features.
            error_generic_member_access = false;
            consider_rustc_bootstrap = true;
        }

        if error_generic_member_access {
            println!("cargo:rustc-cfg=error_generic_member_access");
        }

        if consider_rustc_bootstrap {
            println!("cargo:rerun-if-env-changed=RUSTC_BOOTSTRAP");
        }
    }

    // std::backtrace::Backtrace is always available with MSRV 1.93
    // https://blog.rust-lang.org/2022/11/03/Rust-1.65.0.html#stabilized-apis
    if cfg!(feature = "std") {
        println!("cargo:rustc-cfg=std_backtrace");
    }

    // Declare expected cfgs for cargo check-cfg
    println!("cargo:rustc-check-cfg=cfg(anyhow_build_probe)");
    println!("cargo:rustc-check-cfg=cfg(anyhow_nightly_testing)");
    println!("cargo:rustc-check-cfg=cfg(error_generic_member_access)");
    println!("cargo:rustc-check-cfg=cfg(std_backtrace)");
}

fn compile_probe(rustc_bootstrap: bool) -> bool {
    if env::var_os("RUSTC_STAGE").is_some() {
        // We are running inside rustc bootstrap. This is a highly non-standard
        // environment with issues such as:
        //
        //     https://github.com/rust-lang/cargo/issues/11138
        //     https://github.com/rust-lang/rust/issues/114839
        //
        // Let's just not use nightly features here.
        return false;
    }

    let rustc = cargo_env_var("RUSTC");
    let out_dir = cargo_env_var("OUT_DIR");
    let out_subdir = Path::new(&out_dir).join("probe");
    let probefile = Path::new("src").join("nightly.rs");

    if let Err(err) = fs::create_dir(&out_subdir)
        && err.kind() != ErrorKind::AlreadyExists {
            eprintln!("Failed to create {}: {}", out_subdir.display(), err);
            process::exit(1);
        }

    let rustc_wrapper = env::var_os("RUSTC_WRAPPER").filter(|wrapper| !wrapper.is_empty());
    let rustc_workspace_wrapper =
        env::var_os("RUSTC_WORKSPACE_WRAPPER").filter(|wrapper| !wrapper.is_empty());
    let mut rustc = rustc_wrapper
        .into_iter()
        .chain(rustc_workspace_wrapper)
        .chain(iter::once(rustc));
    let mut cmd = Command::new(rustc.next().unwrap());
    cmd.args(rustc);

    if !rustc_bootstrap {
        cmd.env_remove("RUSTC_BOOTSTRAP");
    }

    cmd.stderr(Stdio::null())
        .arg("--cfg=anyhow_build_probe")
        .arg("--edition=2024")
        .arg("--crate-name=anyhow")
        .arg("--crate-type=lib")
        .arg("--cap-lints=allow")
        .arg("--emit=dep-info,metadata")
        .arg("--out-dir")
        .arg(&out_subdir)
        .arg(probefile);

    if let Some(target) = env::var_os("TARGET") {
        cmd.arg("--target").arg(target);
    }

    // If Cargo wants to set RUSTFLAGS, use that.
    if let Ok(rustflags) = env::var("CARGO_ENCODED_RUSTFLAGS")
        && !rustflags.is_empty() {
            for arg in rustflags.split('\x1f') {
                cmd.arg(arg);
            }
        }

    let success = match cmd.status() {
        Ok(status) => status.success(),
        Err(_) => false,
    };

    // Clean up to avoid leaving nondeterministic absolute paths in the dep-info
    // file in OUT_DIR, which causes nonreproducible builds in build systems
    // that treat the entire OUT_DIR as an artifact.
    if let Err(err) = fs::remove_dir_all(&out_subdir)
        && err.kind() != ErrorKind::NotFound && err.kind() != ErrorKind::DirectoryNotEmpty {
            eprintln!("Failed to clean up {}: {}", out_subdir.display(), err);
            process::exit(1);
        }

    success
}

fn cargo_env_var(key: &str) -> OsString {
    env::var_os(key).unwrap_or_else(|| {
        eprintln!(
            "Environment variable ${} is not set during execution of build script",
            key,
        );
        process::exit(1);
    })
}
