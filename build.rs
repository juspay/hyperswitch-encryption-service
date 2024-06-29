mod cargo_workspace {
    include!("src/workspace.rs");
}

fn main() {
    cargo_workspace::set_cargo_workspace_members_env();
}
