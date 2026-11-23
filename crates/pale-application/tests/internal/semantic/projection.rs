//! Verifies deterministic semantic projection precedence without changing qualification.

use pale_core::records::{
    ModeDefaultSource, PackageIdV1, PackageResourceIdV1, PackageResourceRefV1,
};

use super::preferred_mode_default;

fn resource(package: &str, id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse(package).expect("valid test Package id"),
        resource_id: PackageResourceIdV1::parse(id).expect("valid test resource id"),
    }
}

#[test]
fn project_mode_default_outranks_user_fallback_without_dequalification() {
    let project = resource("project-package", "focused");
    let user = resource("user-package", "balanced");

    assert_eq!(
        preferred_mode_default(Some(&project), Some(&user)),
        Some((&project, ModeDefaultSource::Project))
    );
    assert_eq!(
        preferred_mode_default(None, Some(&user)),
        Some((&user, ModeDefaultSource::User))
    );
}
