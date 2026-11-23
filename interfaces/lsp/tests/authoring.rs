//! Public-surface regression for the configuration-only process leaf.

#[test]
fn public_surface_is_the_stdio_leaf_and_its_typed_failure() {
    let _: fn() -> Result<(), pale_lsp::ServerError> = pale_lsp::run_stdio;
}
