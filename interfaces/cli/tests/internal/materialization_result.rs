//! CLI completeness classification for application-owned materialization results.

use pale_sdk::LocalMaterializationRunOutcome;

use super::result_completion;
use crate::terminal::ResultCompletion;

#[test]
fn result_completeness_is_orthogonal_to_settlement_success() {
    assert_eq!(
        result_completion(LocalMaterializationRunOutcome::Succeeded),
        ResultCompletion::Complete
    );
    assert_eq!(
        result_completion(LocalMaterializationRunOutcome::FailedNoEffect),
        ResultCompletion::Complete,
        "a conclusive no-effect failure is complete even though it is unsuccessful"
    );
    assert_eq!(
        result_completion(LocalMaterializationRunOutcome::Partial),
        ResultCompletion::Partial
    );
}
