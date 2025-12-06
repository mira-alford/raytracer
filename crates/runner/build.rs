use std::{
    fs::File,
    io::{BufWriter, Read, Write},
};

fn build_slang(file: &str) {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let output_spv = out_dir.join(format!("{}.spv", file));
    let output_json = out_dir.join(format!("{}.json", file));
    let output_rs = out_dir.join(format!("{}_reflection.rs", file));
    let output = std::process::Command::new("slangc")
        .args(["-profile", "sm_4_0"])
        .args([
            &format!("{}/../../shaders/{}.slang", manifest_dir, file),
            "-o",
        ])
        .arg(&output_spv)
        .arg("-reflection-json")
        .arg(&output_json)
        .output()
        .expect("Failed to run slangc");

    if !(output.status.success()) {
        eprintln!("slangc failed:");
        if !output.stderr.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }
        if !output.stdout.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&output.stdout));
        }
        panic!("Shader compilation failed.");
    }
}

fn main() {
    println!("cargo:rerun-if-changed=shaders");

    build_slang("render");
    build_slang("logic");
    build_slang("new_ray");
    build_slang("extension");

    // Old WESL stuff:
    wesl::Wesl::new("src/shaders")
        .build_artifact(&"package::extension".parse().unwrap(), "extension");
    wesl::Wesl::new("src/shaders").build_artifact(&"package::new_ray".parse().unwrap(), "new_ray");
    wesl::Wesl::new("src/shaders").build_artifact(&"package::logic".parse().unwrap(), "logic");
    wesl::Wesl::new("src/shaders").build_artifact(&"package::shader".parse().unwrap(), "shader");
}
