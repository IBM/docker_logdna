fn main() {
    // TODO: does this even do anything?
    println!("cargo:rerun-if-changed=src/entry.proto");
    prost_build::compile_protos(&["src/entry.proto"], &["src/"])
        .expect("Failed to compile protobuf");
}
