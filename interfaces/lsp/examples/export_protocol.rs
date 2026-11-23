//! Write private editor schemas to standard output for development tooling.

use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = io::stdout().lock();
    serde_json::to_writer_pretty(&mut output, &pale_lsp::private_protocol_schemas())?;
    writeln!(output)?;
    Ok(())
}
