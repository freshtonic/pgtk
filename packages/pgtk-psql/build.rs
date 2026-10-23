// `build-support/target_version.rs` is a link to the file of the same name in
// `pgtk-sql`, so that the published package holds the file.
#[path = "build-support/target_version.rs"]
mod target_version;

fn main() -> Result<(), recursa_codegen::GenerateError> {
    println!("cargo::rerun-if-changed=build-support/target_version.rs");
    // Check the version features before the grammar is generated, so an
    // incorrect feature set gives this message and not a codegen error.
    target_version::require_one_target_version("pgtk-psql");
    recursa_codegen::generate("src/lib.rs")?;
    Ok(())
}
