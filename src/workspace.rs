/// Sets the `CARGO_WORKSPACE_MEMBERS` environment variable to include a comma-separated list of
/// names of all crates in the current cargo workspace.
///
/// This function should be typically called within build scripts, so that the environment variable
/// is available to the corresponding crate at compile time.
///
/// # Panics
///
/// Panics if running the `cargo metadata` command fails.
#[allow(clippy::expect_used)]
pub fn set_cargo_workspace_members_env() {
    use std::io::Write;

    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .expect("Failed to obtain cargo metadata");

    let workspace_members = metadata
        .workspace_packages()
        .iter()
        .map(|package| package.name.as_str())
        .collect::<Vec<_>>()
        .join(",");

    writeln!(
        &mut std::io::stdout(),
        "cargo:rustc-env=CARGO_WORKSPACE_MEMBERS={workspace_members}"
    )
    .expect("Failed to set `CARGO_WORKSPACE_MEMBERS` environment variable");
}
