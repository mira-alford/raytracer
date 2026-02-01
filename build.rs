use std::{
    fs::File,
    io::{BufWriter, Read, Write},
    os::unix::process,
    path::{Path, PathBuf},
};

fn build_slang(file: &str) {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    eprintln!("{}", std::env::var("PWD").unwrap());
    eprintln!("{}", manifest_dir);
    let out = std::process::Command::new("ls").output().unwrap();
    eprintln!("{:?}", out);
    let shader = Path::new(&manifest_dir)
        .join("shaders")
        .join(format!("{file}.slang"));
    assert!(shader.exists(), "Shader not found: {}", shader.display());

    let output_spv = out_dir.join(format!("{file}.spv"));
    let output_json = out_dir.join(format!("{file}.json"));

    let output = std::process::Command::new("slangc")
        .args(["-profile", "glsl_460"])
        .arg(&shader)
        .arg("-o")
        .arg(&output_spv)
        .arg("-reflection-json")
        .arg(&output_json)
        .output()
        .expect("Failed to run slangc");

    if !output.status.success() {
        eprintln!(
            "slangc failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        panic!("Shader compilation failed.");
    }
}

fn main() {
    println!("cargo:rerun-if-changed=shaders");

    build_slang("render");
    build_slang("sample");
    build_slang("ray_extend");
    // build_slang("logic");
    // build_slang("new_ray");
    // build_slang("extension");
    // build_slang("lambertian");
    // build_slang("metallic");
    // build_slang("dielectric");
    // build_slang("emissive");
    // build_slang("shadow");
    // build_slang("magic");
}
