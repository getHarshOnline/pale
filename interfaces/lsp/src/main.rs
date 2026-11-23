//! Standard-input/output process leaf for [`pale_lsp`].

fn main() -> Result<(), pale_lsp::ServerError> {
    pale_lsp::run_stdio()
}
