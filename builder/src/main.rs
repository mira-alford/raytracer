use spirv_builder::{MetadataPrintout, SpirvBuilder};
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

fn build_shader(path_to_crate: &str, env_var: &str) -> Result<(), Box<dyn Error>> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("Expected Manifest Dir");
    let builder_dir = &Path::new(&manifest_dir);
    let path_to_crate = builder_dir.join(path_to_crate);

    let result = SpirvBuilder::new(path_to_crate, "spirv-unknown-vulkan1.4")
        .print_metadata(MetadataPrintout::Full)
        .build()?;

    println!("{:?}", result.codegen_entry_point_strings());
    println!("{:?}", result.module);

    if let Ok(out_dir) = std::env::var("OUT_DIR") {
        println!(
            "cargo:rustc-env={}_PATH={}",
            env_var,
            result.module.unwrap_single().display()
        );
        let dest_path = Path::new(&out_dir).join("entry_point.rs");
        fs::create_dir_all(&out_dir).unwrap();
        fs::write(&dest_path, result.codegen_entry_point_strings()).unwrap();
        // fs::write(&dest_path, result.entry_points.join(",")).unwrap();
        println!(
            "cargo:rustc-env={}_ENTRYPOINTS={}",
            env_var,
            dest_path.display()
        );
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let full_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(file!());
    println!("cargo:rerun-if-changed={}", full_path.display());
    build_shader("../crates/shaders", "SHADERS")?;
    Ok(())
}
