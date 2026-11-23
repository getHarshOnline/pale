//! Semantic-lineage validation and closed physical transition projection.

use pale_core::records::{
    ActionEnvelopeV1, CommonRequestEnvelopeV1, EffectAdmissionV1, EffectConflictClaimStateV1,
    EffectConflictClaimV1, EffectConflictTerminalReasonV1, EffectDispatchIntentV1, EffectReceiptV1,
    ReconciliationCaseOriginV1, ReconciliationCaseV1, ReconciliationCustodyStateV1,
    ReconciliationCustodyV1, ReconciliationRequestV1, ReconciliationResultV1,
};
use pale_ports::{
    AntiRollbackAnchor, AuthorityStoreCoordinate, ControlCheckpointHead,
    ControlDomainAntiRollbackAnchor, ControlRecordCoordinate, EffectCaseActionAdmissionKind,
    EffectCaseCheckpointTransition, EffectCaseIndexAssertion, EffectCaseIndexMutation,
    EffectCaseIndexPredecessor, EffectCaseReconciliationAdvanceKind, EffectCaseTransitionKind,
    EffectCaseTransitionMember, PreparedAntiRollbackAnchorTransition,
};

use super::record::{
    project_action, project_admission, project_case, project_checkpoint_record, project_claim,
    project_custody, project_disposition, project_intent, project_receipt,
    project_reconciliation_request, project_request, project_result, require, verify_base,
    verify_claim, verify_claim_resource, verify_custody_resource, verify_disposition,
    verify_disposition_admission, EffectCaseKeys,
};
use super::{
    AdmittedEffectCaseCheckpointTransition, AdmittedEffectCaseRecord,
    EffectCaseCheckpointConstruction, EffectCaseDisposition, EffectCaseProjectionError,
    EffectCaseTransitionInput,
};

pub(super) struct ProjectedTransition {
    pub(super) kind: EffectCaseTransitionKind,
    pub(super) members: Vec<EffectCaseTransitionMember>,
    pub(super) mutations: Vec<EffectCaseIndexMutation>,
    pub(super) assertions: Vec<EffectCaseIndexAssertion>,
    pub(super) dispatch_intent: Option<ControlRecordCoordinate>,
}

pub(super) fn project_transition(
    input: EffectCaseTransitionInput<'_>,
) -> Result<ProjectedTransition, EffectCaseProjectionError> {
    match input {
        EffectCaseTransitionInput::AdmitNew {
            request,
            action,
            claim,
            admission,
            prior_claim,
            prior_custody,
        } => {
            verify_base(request.record, action.record)?;
            verify_claim(claim.record, request.record, action.record)?;
            admission.record.verify()?;
            require(
                claim.record.claim_state() == &EffectConflictClaimStateV1::Active
                    && admission.record.request_id() == request.record.request_id()
                    && admission.record.canonical_request_sha256()
                        == request.record.canonical_request_sha256()
                    && admission.record.action_id() == action.record.action_id()
                    && admission.record.action_body_sha256()
                        == &action.record.canonical_action_body_sha256()?
                    && admission.record.conflict_claim_ref() == claim.record_ref
                    && admission.record.conflict_claim_sha256()
                        == claim.record.canonical_effect_conflict_claim_sha256(),
            )?;
            if let Some(custody) = prior_custody {
                verify_custody_resource(custody.record, action.record)?;
                require(custody.record.custody_state() == &ReconciliationCustodyStateV1::Closed)?;
            }
            if let Some(prior) = prior_claim {
                verify_claim_resource(prior.record, request.record, action.record)?;
            }
            require(matches!(
                (
                    prior_claim.map(|prior| prior.record.claim_state()),
                    prior_custody
                ),
                (None, None)
                    | (Some(EffectConflictClaimStateV1::Released), None)
                    | (Some(EffectConflictClaimStateV1::Transferred), Some(_))
            ))?;
            let keys = EffectCaseKeys::derive(request.record, action.record)?;
            let request_record = project_request(request)?;
            let action_record = project_action(request.record, action)?;
            let claim_record = project_claim(claim)?;
            let admission_record = project_admission(admission)?;
            let prior_claim = prior_claim.map(project_claim).transpose()?;
            let prior_custody = prior_custody.map(project_custody).transpose()?;
            Ok(ProjectedTransition {
                kind: EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New),
                members: vec![
                    EffectCaseTransitionMember::CommonRequest(request_record.clone()),
                    EffectCaseTransitionMember::Action(action_record),
                    EffectCaseTransitionMember::ConflictClaim(claim_record.clone()),
                    EffectCaseTransitionMember::Admission(admission_record.clone()),
                ],
                mutations: vec![
                    EffectCaseIndexMutation::RequestIdentity {
                        key: keys.request,
                        expected: EffectCaseIndexPredecessor::Absent,
                        successor: request_record.coordinate().clone(),
                    },
                    EffectCaseIndexMutation::LogicalAction {
                        key: keys.logical.clone(),
                        expected: EffectCaseIndexPredecessor::Absent,
                        successor: admission_record.coordinate().clone(),
                    },
                    EffectCaseIndexMutation::ActionDisposition {
                        key: keys.logical.clone(),
                        expected: EffectCaseIndexPredecessor::Absent,
                        successor: admission_record.coordinate().clone(),
                    },
                    EffectCaseIndexMutation::ActiveClaim {
                        key: keys.conflict.clone(),
                        expected: prior_claim
                            .map(EffectCaseIndexPredecessor::Exact)
                            .unwrap_or(EffectCaseIndexPredecessor::Absent),
                        successor: claim_record.coordinate().clone(),
                    },
                ],
                assertions: vec![EffectCaseIndexAssertion::ActiveCustody {
                    key: keys.conflict,
                    expected: prior_custody
                        .map(EffectCaseIndexPredecessor::Exact)
                        .unwrap_or(EffectCaseIndexPredecessor::Absent),
                }],
                dispatch_intent: None,
            })
        }
        EffectCaseTransitionInput::AliasExisting {
            request,
            action,
            admission,
            disposition,
        } => {
            verify_base(request.record, action.record)?;
            admission.record.verify()?;
            verify_disposition(disposition, action.record)?;
            verify_disposition_admission(disposition, admission)?;
            require(
                admission.record.action_id() == action.record.action_id()
                    && admission.record.action_body_sha256()
                        == &action.record.canonical_action_body_sha256()?,
            )?;
            let keys = EffectCaseKeys::derive(request.record, action.record)?;
            let request_record = project_request(request)?;
            let admission_record = project_admission(admission)?;
            let disposition_record = project_disposition(disposition)?;
            Ok(ProjectedTransition {
                kind: EffectCaseTransitionKind::AdmitAction(
                    EffectCaseActionAdmissionKind::AliasExisting,
                ),
                members: vec![EffectCaseTransitionMember::CommonRequest(
                    request_record.clone(),
                )],
                mutations: vec![EffectCaseIndexMutation::RequestIdentity {
                    key: keys.request,
                    expected: EffectCaseIndexPredecessor::Absent,
                    successor: request_record.coordinate().clone(),
                }],
                assertions: vec![
                    EffectCaseIndexAssertion::LogicalAction {
                        key: keys.logical.clone(),
                        expected: EffectCaseIndexPredecessor::Exact(admission_record),
                    },
                    EffectCaseIndexAssertion::ActionDisposition {
                        key: keys.logical,
                        expected: EffectCaseIndexPredecessor::Exact(disposition_record),
                    },
                ],
                dispatch_intent: None,
            })
        }
        EffectCaseTransitionInput::CommitDispatchIntent {
            request,
            action,
            admission,
            active_claim,
            intent,
        } => {
            verify_base(request.record, action.record)?;
            verify_claim(active_claim.record, request.record, action.record)?;
            admission.record.verify()?;
            intent.record.verify()?;
            let action_digest = action.record.canonical_action_body_sha256()?;
            require(
                active_claim.record.claim_state() == &EffectConflictClaimStateV1::Active
                    && admission.record.request_id() == request.record.request_id()
                    && admission.record.canonical_request_sha256()
                        == request.record.canonical_request_sha256()
                    && admission.record.action_id() == action.record.action_id()
                    && admission.record.action_body_sha256() == &action_digest
                    && admission.record.conflict_claim_ref() == active_claim.record_ref
                    && admission.record.conflict_claim_sha256()
                        == active_claim.record.canonical_effect_conflict_claim_sha256()
                    && intent.record.request_ref() == request.record_ref
                    && intent.record.canonical_request_sha256()
                        == request.record.canonical_request_sha256()
                    && intent.record.action_id() == action.record.action_id()
                    && intent.record.action_body_sha256() == &action_digest
                    && intent.record.effect_admission_ref() == admission.record_ref
                    && intent.record.effect_admission_sha256()
                        == admission.record.canonical_effect_admission_sha256()
                    && intent.record.conflict_claim_ref() == active_claim.record_ref
                    && intent.record.conflict_claim_sha256()
                        == active_claim.record.canonical_effect_conflict_claim_sha256(),
            )?;
            let keys = EffectCaseKeys::derive(request.record, action.record)?;
            let admission_record = project_admission(admission)?;
            let claim_record = project_claim(active_claim)?;
            let intent_record = project_intent(intent)?;
            Ok(ProjectedTransition {
                kind: EffectCaseTransitionKind::CommitDispatchIntent,
                members: vec![EffectCaseTransitionMember::DispatchIntent(
                    intent_record.clone(),
                )],
                mutations: vec![
                    EffectCaseIndexMutation::ActionIntent {
                        key: keys.logical.clone(),
                        expected: EffectCaseIndexPredecessor::Absent,
                        successor: intent_record.coordinate().clone(),
                    },
                    EffectCaseIndexMutation::ActionDisposition {
                        key: keys.logical.clone(),
                        expected: EffectCaseIndexPredecessor::Exact(admission_record),
                        successor: intent_record.coordinate().clone(),
                    },
                ],
                assertions: vec![EffectCaseIndexAssertion::ActiveClaim {
                    key: keys.conflict,
                    expected: EffectCaseIndexPredecessor::Exact(claim_record),
                }],
                dispatch_intent: Some(intent_record.coordinate().clone()),
            })
        }
        EffectCaseTransitionInput::CancelBeforeDispatch {
            request,
            action,
            admission,
            active_claim,
            released_claim,
        } => project_terminal_claim(request, action, admission, active_claim, released_claim),
        EffectCaseTransitionInput::SettleWithoutReconciliation {
            request,
            action,
            intent,
            active_claim,
            released_claim,
            receipt,
        } => project_settlement(
            request,
            action,
            intent,
            active_claim,
            released_claim,
            receipt,
        ),
        EffectCaseTransitionInput::OpenReconciliation {
            request,
            action,
            intent,
            active_claim,
            transferred_claim,
            receipt,
            case,
            custody,
            prior_custody,
        } => project_open(
            request,
            action,
            intent,
            active_claim,
            transferred_claim,
            receipt,
            case,
            custody,
            prior_custody,
        ),
        EffectCaseTransitionInput::AdvanceReconciliation {
            effect_request,
            action,
            request,
            reconciliation_request,
            case,
            predecessor,
            custody,
            result,
            closed_custody,
        } => project_advance(
            effect_request,
            action,
            request,
            reconciliation_request,
            case,
            predecessor,
            custody,
            result,
            closed_custody,
        ),
        EffectCaseTransitionInput::TransferCustody {
            request,
            action,
            disposition,
            custody,
            successor,
        } => project_transfer(request, action, disposition, custody, successor),
        EffectCaseTransitionInput::ReopenReconciliation {
            request,
            action,
            active_claim,
            predecessor_case,
            predecessor_result,
            closed_custody,
            case,
            custody,
        } => project_reopen(
            request,
            action,
            active_claim,
            predecessor_case,
            predecessor_result,
            closed_custody,
            case,
            custody,
        ),
    }
}

fn project_terminal_claim(
    request: AdmittedEffectCaseRecord<'_, CommonRequestEnvelopeV1>,
    action: AdmittedEffectCaseRecord<'_, ActionEnvelopeV1>,
    admission: AdmittedEffectCaseRecord<'_, EffectAdmissionV1>,
    active_claim: AdmittedEffectCaseRecord<'_, EffectConflictClaimV1>,
    released_claim: AdmittedEffectCaseRecord<'_, EffectConflictClaimV1>,
) -> Result<ProjectedTransition, EffectCaseProjectionError> {
    verify_base(request.record, action.record)?;
    verify_claim(active_claim.record, request.record, action.record)?;
    verify_claim(released_claim.record, request.record, action.record)?;
    admission.record.verify()?;
    let action_digest = action.record.canonical_action_body_sha256()?;
    require(
        active_claim.record.claim_state() == &EffectConflictClaimStateV1::Active
            && released_claim.record.claim_state() == &EffectConflictClaimStateV1::Released
            && released_claim.record.terminal_reason()
                == &Some(EffectConflictTerminalReasonV1::CancelledBeforeDispatch)
            && released_claim.record.previous_claim_ref().as_ref() == Some(active_claim.record_ref)
            && released_claim.record.previous_claim_sha256().as_ref()
                == Some(active_claim.record.canonical_effect_conflict_claim_sha256())
            && admission.record.request_id() == request.record.request_id()
            && admission.record.canonical_request_sha256()
                == request.record.canonical_request_sha256()
            && admission.record.action_id() == action.record.action_id()
            && admission.record.action_body_sha256() == &action_digest
            && admission.record.conflict_claim_ref() == active_claim.record_ref
            && admission.record.conflict_claim_sha256()
                == active_claim.record.canonical_effect_conflict_claim_sha256(),
    )?;
    let keys = EffectCaseKeys::derive(request.record, action.record)?;
    let admission_record = project_admission(admission)?;
    let active_record = project_claim(active_claim)?;
    let released_record = project_claim(released_claim)?;
    Ok(ProjectedTransition {
        kind: EffectCaseTransitionKind::CancelBeforeDispatch,
        members: vec![EffectCaseTransitionMember::ConflictClaim(
            released_record.clone(),
        )],
        mutations: vec![
            EffectCaseIndexMutation::ActiveClaim {
                key: keys.conflict.clone(),
                expected: EffectCaseIndexPredecessor::Exact(active_record),
                successor: released_record.coordinate().clone(),
            },
            EffectCaseIndexMutation::ActionDisposition {
                key: keys.logical.clone(),
                expected: EffectCaseIndexPredecessor::Exact(admission_record),
                successor: released_record.coordinate().clone(),
            },
        ],
        assertions: vec![EffectCaseIndexAssertion::ActionIntent {
            key: keys.logical,
            expected: EffectCaseIndexPredecessor::Absent,
        }],
        dispatch_intent: None,
    })
}

fn project_settlement(
    request: AdmittedEffectCaseRecord<'_, CommonRequestEnvelopeV1>,
    action: AdmittedEffectCaseRecord<'_, ActionEnvelopeV1>,
    intent: AdmittedEffectCaseRecord<'_, EffectDispatchIntentV1>,
    active_claim: AdmittedEffectCaseRecord<'_, EffectConflictClaimV1>,
    released_claim: AdmittedEffectCaseRecord<'_, EffectConflictClaimV1>,
    receipt: AdmittedEffectCaseRecord<'_, EffectReceiptV1>,
) -> Result<ProjectedTransition, EffectCaseProjectionError> {
    verify_base(request.record, action.record)?;
    verify_claim(active_claim.record, request.record, action.record)?;
    verify_claim(released_claim.record, request.record, action.record)?;
    verify_disposition(EffectCaseDisposition::DispatchIntent(intent), action.record)?;
    verify_disposition(EffectCaseDisposition::Receipt(receipt), action.record)?;
    require(
        active_claim.record.claim_state() == &EffectConflictClaimStateV1::Active
            && released_claim.record.claim_state() == &EffectConflictClaimStateV1::Released
            && released_claim.record.previous_claim_ref().as_ref() == Some(active_claim.record_ref)
            && released_claim.record.previous_claim_sha256().as_ref()
                == Some(active_claim.record.canonical_effect_conflict_claim_sha256())
            && released_claim.record.effect_receipt_ref().as_ref() == Some(receipt.record_ref)
            && released_claim.record.effect_receipt_sha256().as_ref()
                == Some(receipt.record.canonical_receipt_sha256())
            && receipt.record.effect_dispatch_intent_ref() == intent.record_ref
            && receipt.record.effect_dispatch_intent_sha256()
                == intent.record.canonical_dispatch_intent_sha256()
            && receipt.record.conflict_claim_ref() == active_claim.record_ref
            && receipt.record.conflict_claim_sha256()
                == active_claim.record.canonical_effect_conflict_claim_sha256()
            && !*receipt.record.reconciliation_required(),
    )?;
    let keys = EffectCaseKeys::derive(request.record, action.record)?;
    let intent_record = project_intent(intent)?;
    let active_record = project_claim(active_claim)?;
    let released_record = project_claim(released_claim)?;
    let receipt_record = project_receipt(receipt)?;
    Ok(ProjectedTransition {
        kind: EffectCaseTransitionKind::SettleWithoutReconciliation,
        members: vec![
            EffectCaseTransitionMember::ConflictClaim(released_record.clone()),
            EffectCaseTransitionMember::Receipt(receipt_record.clone()),
        ],
        mutations: vec![
            EffectCaseIndexMutation::ActiveClaim {
                key: keys.conflict.clone(),
                expected: EffectCaseIndexPredecessor::Exact(active_record),
                successor: released_record.coordinate().clone(),
            },
            EffectCaseIndexMutation::ActionDisposition {
                key: keys.logical.clone(),
                expected: EffectCaseIndexPredecessor::Exact(intent_record.clone()),
                successor: receipt_record.coordinate().clone(),
            },
        ],
        assertions: vec![EffectCaseIndexAssertion::ActionIntent {
            key: keys.logical,
            expected: EffectCaseIndexPredecessor::Exact(intent_record),
        }],
        dispatch_intent: None,
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "the arguments are the closed reconciliation-opening witness set"
)]
fn project_open(
    request: AdmittedEffectCaseRecord<'_, CommonRequestEnvelopeV1>,
    action: AdmittedEffectCaseRecord<'_, ActionEnvelopeV1>,
    intent: AdmittedEffectCaseRecord<'_, EffectDispatchIntentV1>,
    active_claim: AdmittedEffectCaseRecord<'_, EffectConflictClaimV1>,
    transferred_claim: AdmittedEffectCaseRecord<'_, EffectConflictClaimV1>,
    receipt: AdmittedEffectCaseRecord<'_, EffectReceiptV1>,
    case: AdmittedEffectCaseRecord<'_, ReconciliationCaseV1>,
    custody_entry: AdmittedEffectCaseRecord<'_, ReconciliationCustodyV1>,
    prior_custody: Option<AdmittedEffectCaseRecord<'_, ReconciliationCustodyV1>>,
) -> Result<ProjectedTransition, EffectCaseProjectionError> {
    verify_base(request.record, action.record)?;
    verify_claim(active_claim.record, request.record, action.record)?;
    verify_claim(transferred_claim.record, request.record, action.record)?;
    verify_disposition(EffectCaseDisposition::DispatchIntent(intent), action.record)?;
    verify_disposition(EffectCaseDisposition::Receipt(receipt), action.record)?;
    verify_disposition(
        EffectCaseDisposition::ReconciliationCase(case),
        action.record,
    )?;
    verify_custody_resource(custody_entry.record, action.record)?;
    if let Some(prior) = prior_custody {
        verify_custody_resource(prior.record, action.record)?;
        require(prior.record.custody_state() == &ReconciliationCustodyStateV1::Closed)?;
    }
    require(
        active_claim.record.claim_state() == &EffectConflictClaimStateV1::Active
            && transferred_claim.record.claim_state() == &EffectConflictClaimStateV1::Transferred
            && transferred_claim.record.previous_claim_ref().as_ref()
                == Some(active_claim.record_ref)
            && transferred_claim.record.previous_claim_sha256().as_ref()
                == Some(active_claim.record.canonical_effect_conflict_claim_sha256())
            && transferred_claim.record.terminal_reason()
                == &Some(EffectConflictTerminalReasonV1::ReconciliationCustody)
            && receipt.record.effect_dispatch_intent_ref() == intent.record_ref
            && receipt.record.effect_dispatch_intent_sha256()
                == intent.record.canonical_dispatch_intent_sha256()
            && receipt.record.conflict_claim_ref() == active_claim.record_ref
            && receipt.record.conflict_claim_sha256()
                == active_claim.record.canonical_effect_conflict_claim_sha256()
            && *receipt.record.reconciliation_required()
            && case.record.case_origin() == &ReconciliationCaseOriginV1::Initial
            && case.record.action_id() == action.record.action_id()
            && case.record.effect_admission_ref() == intent.record.effect_admission_ref()
            && case.record.effect_admission_sha256() == intent.record.effect_admission_sha256()
            && custody_entry.record.case_ref() == case.record_ref
            && custody_entry.record.case_sha256() == case.record.canonical_case_sha256()
            && custody_entry.record.custody_state() == &ReconciliationCustodyStateV1::Active
            && transferred_claim
                .record
                .reconciliation_custody_ref()
                .as_ref()
                == Some(custody_entry.record_ref)
            && transferred_claim
                .record
                .reconciliation_custody_sha256()
                .as_ref()
                == Some(custody_entry.record.canonical_custody_sha256()),
    )?;
    let keys = EffectCaseKeys::derive(request.record, action.record)?;
    let intent_record = project_intent(intent)?;
    let active_record = project_claim(active_claim)?;
    let transferred_record = project_claim(transferred_claim)?;
    let receipt_record = project_receipt(receipt)?;
    let case_record = project_case(case)?;
    let custody_record = project_custody(custody_entry)?;
    let prior_custody_record = prior_custody.map(project_custody).transpose()?;
    let case_key = EffectCaseKeys::case(
        request.record,
        action.record,
        case.record.case_id().as_str(),
    )?;
    Ok(ProjectedTransition {
        kind: EffectCaseTransitionKind::OpenReconciliation,
        members: vec![
            EffectCaseTransitionMember::ConflictClaim(transferred_record.clone()),
            EffectCaseTransitionMember::Receipt(receipt_record),
            EffectCaseTransitionMember::ReconciliationCase(case_record.clone()),
            EffectCaseTransitionMember::ReconciliationCustody(custody_record.clone()),
        ],
        mutations: vec![
            EffectCaseIndexMutation::ActiveClaim {
                key: keys.conflict.clone(),
                expected: EffectCaseIndexPredecessor::Exact(active_record),
                successor: transferred_record.coordinate().clone(),
            },
            EffectCaseIndexMutation::ActiveCustody {
                key: keys.conflict,
                expected: prior_custody_record
                    .map(EffectCaseIndexPredecessor::Exact)
                    .unwrap_or(EffectCaseIndexPredecessor::Absent),
                successor: custody_record.coordinate().clone(),
            },
            EffectCaseIndexMutation::CaseResult {
                key: case_key,
                expected: EffectCaseIndexPredecessor::Absent,
                successor: case_record.coordinate().clone(),
            },
            EffectCaseIndexMutation::ActionDisposition {
                key: keys.logical.clone(),
                expected: EffectCaseIndexPredecessor::Exact(intent_record.clone()),
                successor: case_record.coordinate().clone(),
            },
        ],
        assertions: vec![EffectCaseIndexAssertion::ActionIntent {
            key: keys.logical,
            expected: EffectCaseIndexPredecessor::Exact(intent_record),
        }],
        dispatch_intent: None,
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "the arguments are the closed reconciliation-advance witness set"
)]
fn project_advance(
    effect_request: AdmittedEffectCaseRecord<'_, CommonRequestEnvelopeV1>,
    action: AdmittedEffectCaseRecord<'_, ActionEnvelopeV1>,
    request: AdmittedEffectCaseRecord<'_, CommonRequestEnvelopeV1>,
    reconciliation_request: AdmittedEffectCaseRecord<'_, ReconciliationRequestV1>,
    case: AdmittedEffectCaseRecord<'_, ReconciliationCaseV1>,
    predecessor: EffectCaseDisposition<'_>,
    custody_entry: AdmittedEffectCaseRecord<'_, ReconciliationCustodyV1>,
    result: AdmittedEffectCaseRecord<'_, ReconciliationResultV1>,
    closed_custody: Option<AdmittedEffectCaseRecord<'_, ReconciliationCustodyV1>>,
) -> Result<ProjectedTransition, EffectCaseProjectionError> {
    verify_base(effect_request.record, action.record)?;
    request
        .record
        .verify_reconciliation_request_body(reconciliation_request.record)?;
    verify_disposition(
        EffectCaseDisposition::ReconciliationCase(case),
        action.record,
    )?;
    verify_disposition(predecessor, action.record)?;
    verify_custody_resource(custody_entry.record, action.record)?;
    result.record.verify()?;
    let predecessor_exact = match predecessor {
        EffectCaseDisposition::ReconciliationCase(predecessor_case) => {
            result.record.predecessor_result_ref().is_none()
                && result.record.predecessor_result_sha256().is_none()
                && predecessor_case.record_ref == case.record_ref
                && predecessor_case.record.canonical_case_sha256()
                    == case.record.canonical_case_sha256()
        }
        EffectCaseDisposition::ReconciliationResult(predecessor_result) => {
            result.record.predecessor_result_ref().as_ref() == Some(predecessor_result.record_ref)
                && result.record.predecessor_result_sha256().as_ref()
                    == Some(predecessor_result.record.canonical_reconciliation_sha256())
                && predecessor_result.record.case_ref() == case.record_ref
                && predecessor_result.record.case_sha256() == case.record.canonical_case_sha256()
        }
        _ => false,
    };
    require(
        predecessor_exact
            && custody_entry.record.custody_state() != &ReconciliationCustodyStateV1::Closed
            && custody_entry.record.case_ref() == case.record_ref
            && reconciliation_request.record.case_ref() == result.record.case_ref()
            && reconciliation_request.record.case_ref() == case.record_ref
            && reconciliation_request.record.case_sha256() == case.record.canonical_case_sha256()
            && reconciliation_request.record.custody_ref() == custody_entry.record_ref
            && result.record.custody_ref() == custody_entry.record_ref
            && result.record.custody_sha256() == custody_entry.record.canonical_custody_sha256()
            && result.record.action_id() == action.record.action_id()
            && result.record.request_binding().request_id() == request.record.request_id()
            && result.record.request_binding().canonical_request_sha256()
                == request.record.canonical_request_sha256(),
    )?;
    let terminal = result.record.closed_at().is_some();
    require(terminal == closed_custody.is_some())?;
    let kind = EffectCaseTransitionKind::AdvanceReconciliation(if terminal {
        EffectCaseReconciliationAdvanceKind::Terminal
    } else {
        EffectCaseReconciliationAdvanceKind::Open
    });
    let keys = EffectCaseKeys::derive(effect_request.record, action.record)?;
    let request_record = project_request(request)?;
    let body_record = project_reconciliation_request(request.record, reconciliation_request)?;
    let predecessor_record = project_disposition(predecessor)?;
    let custody_record = project_custody(custody_entry)?;
    let result_record = project_result(result)?;
    let case_key = EffectCaseKeys::case(
        effect_request.record,
        action.record,
        case.record.case_id().as_str(),
    )?;
    let mut members = vec![
        EffectCaseTransitionMember::CommonRequest(request_record.clone()),
        EffectCaseTransitionMember::ReconciliationRequest(body_record),
        EffectCaseTransitionMember::ReconciliationResult(result_record.clone()),
    ];
    let mut mutations = vec![
        EffectCaseIndexMutation::RequestIdentity {
            key: EffectCaseKeys::request(request.record)?,
            expected: EffectCaseIndexPredecessor::Absent,
            successor: request_record.coordinate().clone(),
        },
        EffectCaseIndexMutation::CaseResult {
            key: case_key,
            expected: EffectCaseIndexPredecessor::Exact(predecessor_record.clone()),
            successor: result_record.coordinate().clone(),
        },
        EffectCaseIndexMutation::ActionDisposition {
            key: keys.logical.clone(),
            expected: EffectCaseIndexPredecessor::Exact(predecessor_record),
            successor: result_record.coordinate().clone(),
        },
    ];
    let assertions = if let Some(closed) = closed_custody {
        verify_custody_resource(closed.record, action.record)?;
        require(
            closed.record.custody_state() == &ReconciliationCustodyStateV1::Closed
                && closed.record.previous_custody_ref().as_ref() == Some(custody_entry.record_ref)
                && closed.record.previous_custody_sha256().as_ref()
                    == Some(custody_entry.record.canonical_custody_sha256())
                && closed.record.terminal_result_ref().as_ref() == Some(result.record_ref)
                && closed.record.terminal_result_sha256().as_ref()
                    == Some(result.record.canonical_reconciliation_sha256())
                && closed.record.case_ref() == case.record_ref
                && closed.record.case_sha256() == case.record.canonical_case_sha256(),
        )?;
        let closed_record = project_custody(closed)?;
        members.push(EffectCaseTransitionMember::ReconciliationCustody(
            closed_record.clone(),
        ));
        mutations.push(EffectCaseIndexMutation::ActiveCustody {
            key: keys.conflict.clone(),
            expected: EffectCaseIndexPredecessor::Exact(custody_record),
            successor: closed_record.coordinate().clone(),
        });
        Vec::new()
    } else {
        vec![EffectCaseIndexAssertion::ActiveCustody {
            key: keys.conflict,
            expected: EffectCaseIndexPredecessor::Exact(custody_record),
        }]
    };
    Ok(ProjectedTransition {
        kind,
        members,
        mutations,
        assertions,
        dispatch_intent: None,
    })
}

fn project_transfer(
    request: AdmittedEffectCaseRecord<'_, CommonRequestEnvelopeV1>,
    action: AdmittedEffectCaseRecord<'_, ActionEnvelopeV1>,
    disposition: EffectCaseDisposition<'_>,
    custody_entry: AdmittedEffectCaseRecord<'_, ReconciliationCustodyV1>,
    successor: AdmittedEffectCaseRecord<'_, ReconciliationCustodyV1>,
) -> Result<ProjectedTransition, EffectCaseProjectionError> {
    verify_base(request.record, action.record)?;
    verify_disposition(disposition, action.record)?;
    verify_custody_resource(custody_entry.record, action.record)?;
    verify_custody_resource(successor.record, action.record)?;
    let disposition_case_exact = match disposition {
        EffectCaseDisposition::ReconciliationCase(case) => {
            case.record_ref == custody_entry.record.case_ref()
                && case.record.canonical_case_sha256() == custody_entry.record.case_sha256()
        }
        EffectCaseDisposition::ReconciliationResult(result) => {
            result.record.case_ref() == custody_entry.record.case_ref()
                && result.record.case_sha256() == custody_entry.record.case_sha256()
        }
        _ => false,
    };
    require(
        disposition_case_exact
            && successor.record.custody_state() == &ReconciliationCustodyStateV1::Transferred
            && successor.record.previous_custody_ref().as_ref() == Some(custody_entry.record_ref)
            && successor.record.previous_custody_sha256().as_ref()
                == Some(custody_entry.record.canonical_custody_sha256())
            && custody_entry.record.custody_state() != &ReconciliationCustodyStateV1::Closed
            && successor.record.custody_id() == custody_entry.record.custody_id()
            && successor.record.case_ref() == custody_entry.record.case_ref()
            && successor.record.case_sha256() == custody_entry.record.case_sha256()
            && successor.record.conflict_resource_key_schema_id()
                == custody_entry.record.conflict_resource_key_schema_id()
            && successor.record.conflict_resource_key_schema_sha256()
                == custody_entry.record.conflict_resource_key_schema_sha256()
            && successor.record.conflict_resource_key()
                == custody_entry.record.conflict_resource_key()
            && custody_entry
                .record
                .custody_generation()
                .checked_successor()
                == Some(*successor.record.custody_generation()),
    )?;
    let keys = EffectCaseKeys::derive(request.record, action.record)?;
    let disposition_record = project_disposition(disposition)?;
    let custody_record = project_custody(custody_entry)?;
    let successor_record = project_custody(successor)?;
    Ok(ProjectedTransition {
        kind: EffectCaseTransitionKind::TransferCustody,
        members: vec![EffectCaseTransitionMember::ReconciliationCustody(
            successor_record.clone(),
        )],
        mutations: vec![EffectCaseIndexMutation::ActiveCustody {
            key: keys.conflict,
            expected: EffectCaseIndexPredecessor::Exact(custody_record),
            successor: successor_record.coordinate().clone(),
        }],
        assertions: vec![EffectCaseIndexAssertion::ActionDisposition {
            key: keys.logical,
            expected: EffectCaseIndexPredecessor::Exact(disposition_record),
        }],
        dispatch_intent: None,
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "the arguments are the closed reconciliation-reopen witness set"
)]
fn project_reopen(
    request: AdmittedEffectCaseRecord<'_, CommonRequestEnvelopeV1>,
    action: AdmittedEffectCaseRecord<'_, ActionEnvelopeV1>,
    active_claim: AdmittedEffectCaseRecord<'_, EffectConflictClaimV1>,
    predecessor_case: AdmittedEffectCaseRecord<'_, ReconciliationCaseV1>,
    predecessor_result: AdmittedEffectCaseRecord<'_, ReconciliationResultV1>,
    closed_custody: AdmittedEffectCaseRecord<'_, ReconciliationCustodyV1>,
    case: AdmittedEffectCaseRecord<'_, ReconciliationCaseV1>,
    custody_entry: AdmittedEffectCaseRecord<'_, ReconciliationCustodyV1>,
) -> Result<ProjectedTransition, EffectCaseProjectionError> {
    verify_base(request.record, action.record)?;
    verify_claim(active_claim.record, request.record, action.record)?;
    verify_disposition(
        EffectCaseDisposition::ReconciliationCase(predecessor_case),
        action.record,
    )?;
    verify_disposition(
        EffectCaseDisposition::ReconciliationResult(predecessor_result),
        action.record,
    )?;
    verify_custody_resource(closed_custody.record, action.record)?;
    case.record.verify()?;
    verify_custody_resource(custody_entry.record, action.record)?;
    require(
        active_claim.record.claim_state() == &EffectConflictClaimStateV1::Transferred
            && predecessor_result.record.closed_at().is_some()
            && predecessor_result.record.case_ref() == predecessor_case.record_ref
            && predecessor_result.record.case_sha256()
                == predecessor_case.record.canonical_case_sha256()
            && closed_custody.record.custody_state() == &ReconciliationCustodyStateV1::Closed
            && closed_custody.record.terminal_result_ref().as_ref()
                == Some(predecessor_result.record_ref)
            && closed_custody.record.terminal_result_sha256().as_ref()
                == Some(predecessor_result.record.canonical_reconciliation_sha256())
            && closed_custody.record.case_ref() == predecessor_case.record_ref
            && closed_custody.record.case_sha256()
                == predecessor_case.record.canonical_case_sha256()
            && case.record.case_origin() == &ReconciliationCaseOriginV1::Reopened
            && case.record.predecessor_case_ref().as_ref() == Some(predecessor_case.record_ref)
            && case.record.predecessor_case_sha256().as_ref()
                == Some(predecessor_case.record.canonical_case_sha256())
            && case.record.predecessor_terminal_result_ref().as_ref()
                == Some(predecessor_result.record_ref)
            && case.record.predecessor_terminal_result_sha256().as_ref()
                == Some(predecessor_result.record.canonical_reconciliation_sha256())
            && custody_entry.record.case_ref() == case.record_ref
            && custody_entry.record.case_sha256() == case.record.canonical_case_sha256()
            && custody_entry.record.custody_state() == &ReconciliationCustodyStateV1::Active,
    )?;
    let keys = EffectCaseKeys::derive(request.record, action.record)?;
    let claim_record = project_claim(active_claim)?;
    let predecessor_record = project_result(predecessor_result)?;
    let closed_record = project_custody(closed_custody)?;
    let case_record = project_case(case)?;
    let custody_record = project_custody(custody_entry)?;
    let case_key = EffectCaseKeys::case(
        request.record,
        action.record,
        case.record.case_id().as_str(),
    )?;
    let predecessor_case_key = EffectCaseKeys::case(
        request.record,
        action.record,
        predecessor_case.record.case_id().as_str(),
    )?;
    Ok(ProjectedTransition {
        kind: EffectCaseTransitionKind::ReopenReconciliation,
        members: vec![
            EffectCaseTransitionMember::ReconciliationCase(case_record.clone()),
            EffectCaseTransitionMember::ReconciliationCustody(custody_record.clone()),
        ],
        mutations: vec![
            EffectCaseIndexMutation::ActiveCustody {
                key: keys.conflict.clone(),
                expected: EffectCaseIndexPredecessor::Exact(closed_record),
                successor: custody_record.coordinate().clone(),
            },
            EffectCaseIndexMutation::CaseResult {
                key: case_key,
                expected: EffectCaseIndexPredecessor::Absent,
                successor: case_record.coordinate().clone(),
            },
            EffectCaseIndexMutation::ActionDisposition {
                key: keys.logical.clone(),
                expected: EffectCaseIndexPredecessor::Exact(predecessor_record.clone()),
                successor: case_record.coordinate().clone(),
            },
        ],
        assertions: vec![
            EffectCaseIndexAssertion::ActiveClaim {
                key: keys.conflict,
                expected: EffectCaseIndexPredecessor::Exact(claim_record),
            },
            EffectCaseIndexAssertion::CaseResult {
                key: predecessor_case_key,
                expected: EffectCaseIndexPredecessor::Exact(predecessor_record),
            },
        ],
        dispatch_intent: None,
    })
}

pub(super) fn project_checkpoint(
    store: &AuthorityStoreCoordinate,
    kind: EffectCaseTransitionKind,
    members: &[EffectCaseTransitionMember],
    mutations: &[EffectCaseIndexMutation],
    assertions: &[EffectCaseIndexAssertion],
    checkpoints: &AdmittedEffectCaseCheckpointTransition<'_>,
) -> Result<
    (
        EffectCaseCheckpointTransition,
        PreparedAntiRollbackAnchorTransition,
    ),
    EffectCaseProjectionError,
> {
    checkpoints
        .successor
        .verify_successor_of(checkpoints.predecessor_ref, checkpoints.predecessor)?;
    require(
        checkpoints.predecessor.control_store_id() == store.store_id()
            && checkpoints.predecessor.control_incarnation_id() == store.incarnation_id()
            && checkpoints.successor.control_store_id() == store.store_id()
            && checkpoints.successor.control_incarnation_id() == store.incarnation_id(),
    )?;
    checkpoints
        .verifier
        .verify(&EffectCaseCheckpointConstruction {
            kind,
            members,
            mutations,
            assertions,
            checkpoints,
        })?;
    let predecessor_record =
        project_checkpoint_record(checkpoints.predecessor_ref, checkpoints.predecessor)?;
    let successor_record =
        project_checkpoint_record(checkpoints.successor_ref, checkpoints.successor)?;
    let predecessor_head = ControlCheckpointHead::new(
        checkpoints.predecessor.checkpoint_domain_id().clone(),
        *checkpoints.predecessor.domain_epoch(),
        *checkpoints.predecessor.domain_sequence(),
        *checkpoints.predecessor.committed_transaction_sequence(),
        predecessor_record.coordinate().clone(),
    );
    let successor_head = ControlCheckpointHead::new(
        checkpoints.successor.checkpoint_domain_id().clone(),
        *checkpoints.successor.domain_epoch(),
        *checkpoints.successor.domain_sequence(),
        *checkpoints.successor.committed_transaction_sequence(),
        successor_record.coordinate().clone(),
    );
    let transition = EffectCaseCheckpointTransition::new(
        predecessor_head.clone(),
        predecessor_record,
        successor_head.clone(),
        successor_record,
    )?;
    let expected_anchor = ControlDomainAntiRollbackAnchor::new(
        store.store_id().clone(),
        store.incarnation_id().clone(),
        predecessor_head.checkpoint_domain_id().clone(),
        predecessor_head.domain_epoch(),
        predecessor_head.domain_sequence(),
        predecessor_head.coordinate().clone(),
    );
    let successor_anchor = ControlDomainAntiRollbackAnchor::new(
        store.store_id().clone(),
        store.incarnation_id().clone(),
        successor_head.checkpoint_domain_id().clone(),
        successor_head.domain_epoch(),
        successor_head.domain_sequence(),
        successor_head.coordinate().clone(),
    );
    let anchor = PreparedAntiRollbackAnchorTransition::successor(
        AntiRollbackAnchor::from(expected_anchor),
        AntiRollbackAnchor::from(successor_anchor),
    )?;
    Ok((transition, anchor))
}
