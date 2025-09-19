fn emit_vergen() -> Result<(), Box<dyn std::error::Error>> {
    use vergen_gitcl::*;

    let build = BuildBuilder::default().build_date(true).build()?;
    let cargo = CargoBuilder::default().debug(true).target_triple(true).build()?;
    let rustc = RustcBuilder::default().semver(true).build()?;
    let git = GitclBuilder::default().describe(true, true, None).build()?;

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&rustc)?
        .add_instructions(&git)?
        .emit()?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    emit_vergen()?;
    Ok(())
}
