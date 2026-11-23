//! Admitted Package graph value tests.

use super::AdmittedMarkdown;
use pale_core::records::{PackageIdV1, PackageResourceIdV1, PackageResourceRefV1};

impl AdmittedMarkdown {
    /// Shift every heading from the already admitted syntax projection.
    fn shift_headings(&self, levels: usize) -> String {
        self.projection.shift_headings(self.text(), levels)
    }
}

fn owner() -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("test-package").unwrap(),
        resource_id: PackageResourceIdV1::parse("test-body").unwrap(),
    }
}

#[test]
fn one_admitted_projection_serves_every_content_consumer() {
    let admitted =
        AdmittedMarkdown::from_text(owner(), "# Title\n\n## Procedure\n\nExact bytes.\n");

    assert_eq!(admitted.title(), Some("Title"));
    assert_eq!(
        admitted.body_recipe().materialize().as_str(),
        "## Procedure\n\nExact bytes."
    );
    assert_eq!(
        admitted.shifted_body_recipe(1).materialize().as_str(),
        "### Procedure\n\nExact bytes."
    );
    assert_eq!(
        admitted.shift_headings(2),
        "### Title\n\n#### Procedure\n\nExact bytes.\n"
    );
    assert_eq!(admitted.text(), "# Title\n\n## Procedure\n\nExact bytes.\n");
}
