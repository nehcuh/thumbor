use std::fs;
use std::path::Path;

fn main() {
    let target_path = Path::new("src/pb");

    if !target_path.exists() {
        fs::create_dir_all(target_path).expect("target path {target_path} failed to create");
    }

    prost_build::Config::new()
        .out_dir(target_path)
        .compile_protos(&["abi.proto"], &["."])
        .expect("proto3 should be compiled");
}
