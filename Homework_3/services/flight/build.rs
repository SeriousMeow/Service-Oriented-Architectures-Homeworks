use std::env;

fn main() {
    println!("cargo:rerun-if-changed=../../proto/flight/v1/flight.proto");

    let protoc_path =
        protoc_bin_vendored::protoc_bin_path().expect("failed to get vendored protoc path");
    // Build script runs single-threaded here; setting PROTOC only affects this process.
    unsafe {
        env::set_var("PROTOC", protoc_path);
    }

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(&["../../proto/flight/v1/flight.proto"], &["../../proto"])
        .expect("failed to compile flight proto");
}
