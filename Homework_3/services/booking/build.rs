use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../proto/flight/v1/flight.proto");
    println!("cargo:rerun-if-changed=../../openapi/booking.yaml");

    let protoc_path =
        protoc_bin_vendored::protoc_bin_path().expect("failed to get vendored protoc path");
    // Build script runs single-threaded here; setting PROTOC only affects this process.
    unsafe {
        env::set_var("PROTOC", protoc_path);
    }

    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&["../../proto/flight/v1/flight.proto"], &["../../proto"])
        .expect("failed to compile flight proto for booking client");

    let status = match Command::new("oas3-gen")
        .args([
            "generate",
            "server-mod",
            "-i",
            "../../openapi/booking.yaml",
            "-o",
            "src/api",
        ])
        .status()
    {
        Ok(status) => status,
        Err(error) => {
            println!(
                "cargo:warning=failed to run oas3-gen ({error}); using checked-in generated API files"
            );
            return;
        }
    };

    if !status.success() {
        panic!("oas3-gen failed with status: {status}");
    }
}
