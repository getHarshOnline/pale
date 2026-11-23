//! Existing compiler fixture shared by command, terminal and protocol tests.

/// Copy the eight existing visibility-widening documents without another corpus.
pub(super) fn documents() -> Vec<(&'static str, String)> {
    macro_rules! document {
        ($label:literal) => {
            (
                $label,
                include_str!(concat!(
                    "../../../lsp/tests/fixtures/authoring/visibility-widening/",
                    $label,
                ))
                .to_owned(),
            )
        };
    }
    vec![
        document!("pale-workspace.toml"),
        document!("module.toml"),
        document!("consumer-module.toml"),
        document!("alpha.toml"),
        document!("beta.toml"),
        document!("consumer.toml"),
        document!("resolution.lock.toml"),
        document!("universe.lock.toml"),
    ]
}
