//! Localized bytes emitted by the application's real organization fixture owner.

pub(crate) fn documents() -> Vec<(String, Vec<u8>)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/organization");
    [
        "pale-workspace.toml",
        "organization.toml",
        "responsibility.toml",
        "resolution.lock.toml",
        "universe.lock.toml",
    ]
    .into_iter()
    .map(|label| {
        (
            label.to_owned(),
            std::fs::read(root.join(label))
                .expect("run the documented owning fixture export and byte qualification first"),
        )
    })
    .collect()
}
