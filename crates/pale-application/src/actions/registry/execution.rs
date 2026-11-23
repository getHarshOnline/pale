//! Closed handler dispatch and successful result/receipt production.

use pale_core::records::{
    ActionError, ActionErrorCode, ActionExecution, ActionInput, ActionOutput, ActionReceipt,
    ActionReceiptDigest, ActionResult, ActionResultDigest,
};

use super::seal::{ActionReceiptPreimage, ActionResultPreimage};
use crate::actions::{
    compliance, config_invariant, cost, evidence_selection, loop_status, path_inventory, ranking,
    review_convergence, rubric, service_definition, skill_health, ACTION_RECORD_SCHEMA_VERSION,
    RECEIPT_RECORD, RESULT_RECORD,
};

use super::{NativeActionDefinition, PreparedNativeAction};
use crate::actions::error::{action_error, request_error};

macro_rules! native_action_definitions {
    ($(($executor:ident, $module:ident, $variant:ident, $id:ident, $description:literal)),+ $(,)?) => {
        $(
            fn $executor(input: &ActionInput) -> Result<ActionOutput, ActionError> {
                match input {
                    ActionInput::$variant(input) =>
                        $module::execute(input).map(ActionOutput::$variant),
                    _ => Err(action_error(
                        Some($id),
                        ActionErrorCode::InputMismatch,
                        "request payload kind does not match the native action definition",
                    )),
                }
            }
        )+

        /// Sole application-owned pairing of action metadata and executable handlers.
        pub(super) const NATIVE_ACTION_DEFINITIONS: &[NativeActionDefinition] = &[
            $(NativeActionDefinition {
                id: $id,
                description: $description,
                input_kind: pale_core::records::ActionInputKind::$variant,
                output_kind: pale_core::records::ActionOutputKind::$variant,
                execute: $executor,
            }),+
        ];
    };
}

use pale_core::records::{
    COMPLIANCE_TRACE_GRADE_ACTION_ID, CONFIG_INVARIANT_SCAN_ACTION_ID, COST_AGGREGATE_ACTION_ID,
    EVIDENCE_SELECTION_ACTION_ID, LOOP_STATUS_REDUCE_ACTION_ID, PATH_INVENTORY_ACTION_ID,
    REVIEW_CONVERGENCE_REDUCE_ACTION_ID, RUBRIC_EVALUATION_ACTION_ID,
    SERVICE_DEFINITION_NORMALIZE_ACTION_ID, SKILL_HEALTH_AGGREGATE_ACTION_ID,
    WEIGHTED_RANKING_ACTION_ID,
};

native_action_definitions!(
    (
        execute_compliance,
        compliance,
        ComplianceTraceGrade,
        COMPLIANCE_TRACE_GRADE_ACTION_ID,
        "Grade caller-classified trace events against bounded temporal rules."
    ),
    (
        execute_config_invariant,
        config_invariant,
        ConfigInvariantScan,
        CONFIG_INVARIANT_SCAN_ACTION_ID,
        "Evaluate bounded lexical invariant leads over caller-supplied configuration text."
    ),
    (
        execute_cost,
        cost,
        CostAggregate,
        COST_AGGREGATE_ACTION_ID,
        "Select latest cumulative session snapshots and aggregate integer micro-USD costs."
    ),
    (
        execute_loop_status,
        loop_status,
        LoopStatusReduce,
        LOOP_STATUS_REDUCE_ACTION_ID,
        "Reduce normalized loop events into deterministic fail-closed intervention signals."
    ),
    (
        execute_review_convergence,
        review_convergence,
        ReviewConvergenceReduce,
        REVIEW_CONVERGENCE_REDUCE_ACTION_ID,
        "Deduplicate supplied review findings and converge verifier evidence deterministically."
    ),
    (
        execute_rubric,
        rubric,
        RubricEvaluation,
        RUBRIC_EVALUATION_ACTION_ID,
        "Normalize supplied integer rubric results and rank failures deterministically."
    ),
    (
        execute_service_definition,
        service_definition,
        ServiceDefinitionNormalize,
        SERVICE_DEFINITION_NORMALIZE_ACTION_ID,
        "Validate and order provider-neutral service definitions without host access."
    ),
    (
        execute_skill_health,
        skill_health,
        SkillHealthAggregate,
        SKILL_HEALTH_AGGREGATE_ACTION_ID,
        "Aggregate supplied skill observations over caller-defined time windows."
    ),
    (
        execute_ranking,
        ranking,
        WeightedRanking,
        WEIGHTED_RANKING_ACTION_ID,
        "Rank structured candidates with checked fixed-point arithmetic."
    ),
    (
        execute_path_inventory,
        path_inventory,
        PathInventory,
        PATH_INVENTORY_ACTION_ID,
        "Normalize and inventory caller-supplied POSIX paths without filesystem access."
    ),
    (
        execute_evidence_selection,
        evidence_selection,
        EvidenceSelection,
        EVIDENCE_SELECTION_ACTION_ID,
        "Select caller-declared rules from caller-supplied evidence in explicit precedence order."
    ),
);

impl PreparedNativeAction<'_> {
    /// Execute the retained request and seal its result and receipt.
    pub(in crate::actions) fn execute_request(self) -> Result<ActionExecution, ActionError> {
        let Self { action, request } = self;
        let request_digest = request.request_digest.clone();
        let output = (action.execute)(&request.input).map_err(|mut error| {
            error.request_digest = Some(request_digest.clone());
            error
        })?;
        if output.kind() != action.contract.output_kind {
            return Err(request_error(
                &request.action_id,
                &request_digest,
                ActionErrorCode::OutputMismatch,
                "native handler output does not match its sealed contract",
            ));
        }

        let result_preimage = ActionResultPreimage::for_request(&request, &output);
        let result_digest =
            ActionResultDigest::hash_canonical(&result_preimage).map_err(|error| {
                request_error(
                    &request.action_id,
                    &request_digest,
                    ActionErrorCode::CanonicalizationFailed,
                    error.to_string(),
                )
            })?;
        let result = ActionResult {
            schema: RESULT_RECORD.to_string(),
            schema_version: ACTION_RECORD_SCHEMA_VERSION,
            action_id: request.action_id.clone(),
            action_version: request.action_version,
            contract_digest: request.contract_digest.clone(),
            request_digest: request_digest.clone(),
            output,
            result_digest: result_digest.clone(),
        };

        let receipt_preimage = ActionReceiptPreimage::for_request(&request, &result_digest);
        let receipt_digest =
            ActionReceiptDigest::hash_canonical(&receipt_preimage).map_err(|error| {
                request_error(
                    &request.action_id,
                    &request_digest,
                    ActionErrorCode::CanonicalizationFailed,
                    error.to_string(),
                )
            })?;
        let receipt = ActionReceipt {
            schema: RECEIPT_RECORD.to_string(),
            schema_version: ACTION_RECORD_SCHEMA_VERSION,
            action_id: request.action_id.clone(),
            action_version: request.action_version,
            contract_digest: request.contract_digest.clone(),
            request_digest: request_digest.clone(),
            result_digest,
            receipt_digest,
        };
        let schema_identity = pale_contracts::record_schema_identity("action-execution.v1")
            .map_err(|error| {
                request_error(
                    &request.action_id,
                    &request_digest,
                    ActionErrorCode::InternalInvariant,
                    format!("action execution schema identity is invalid: {error}"),
                )
            })?;
        Ok(ActionExecution {
            schema: schema_identity.schema,
            schema_sha256: schema_identity.schema_sha256,
            schema_version: ACTION_RECORD_SCHEMA_VERSION,
            request,
            result,
            receipt,
        })
    }
}
