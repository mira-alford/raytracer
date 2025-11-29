fn main() {
    // Old WESL stuff:
    wesl::Wesl::new("src/shaders")
        .build_artifact(&"package::extension".parse().unwrap(), "extension");
    wesl::Wesl::new("src/shaders").build_artifact(&"package::new_ray".parse().unwrap(), "new_ray");
    wesl::Wesl::new("src/shaders").build_artifact(&"package::logic".parse().unwrap(), "logic");
    wesl::Wesl::new("src/shaders").build_artifact(&"package::shader".parse().unwrap(), "shader");

    // New RustGPU stuff:
    let spirv_cargo =
        std::env::var("SPIRV_CARGO").expect("Expected SPIRV_CARGO env var (provided by flake)");
    let spirv_path =
        std::env::var("SPIRV_PATH").expect("Expected SPIRV_PATH env var (provided by flake)");

    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{spirv_path}:{old_path}");

    let status = std::process::Command::new(spirv_cargo)
        .args(["run", "--manifest-path", "../../builder/Cargo.toml"])
        .env("PATH", new_path)
        .status()
        .expect("Failed to run spirv toolchain cargo for shaders.");

    if !status.success() {
        panic!("Spirv toolchain build failed.");
    }
}
