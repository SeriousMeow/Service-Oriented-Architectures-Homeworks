use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let schema_dir = Path::new(&manifest_dir).join("../../schemas/avro");

    let mut paths: Vec<String> = fs::read_dir(&schema_dir)
        .expect("read schema dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("avsc"))
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    paths.sort();

    let out_avrogen = Path::new(&env::var("OUT_DIR").expect("OUT_DIR")).join("avrogen");

    let mut builder = avrogen::Avrogen::new().output_folder(out_avrogen);
    for p in &paths {
        builder = builder.add_source(p);
    }
    builder.set_verbosity_off().execute().expect("avrogen");

    println!("cargo:rerun-if-changed={}", schema_dir.display());
}
