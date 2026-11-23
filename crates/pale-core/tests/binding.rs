//! Private-binding completeness and storage-boundary tests.

use pale_core::records::{
    DurableEvidenceBinding, MaterializationLineageBinding, RouteRecordLimit, SessionModel,
    SessionStateBinding, SessionTtlSeconds, SurfaceExecutableBinding,
};

#[test]
fn surface_executable_binding_is_either_absent_or_complete() {
    let digest = "a".repeat(64);
    let json = format!(
        r#"{{"exec":"/opt/pale/pale","digest":"{digest}","version":"0.0.1","protocol":"pale.agent-surface-interface.v1"}}"#
    );
    let binding: SurfaceExecutableBinding =
        serde_json::from_str(&json).expect("complete surface executable binding");
    assert_eq!(serde_json::to_string(&binding).unwrap(), json);

    let partial = format!(r#"{{"exec":"/opt/pale/pale","digest":"{digest}","version":"0.0.1"}}"#);
    assert!(serde_json::from_str::<SurfaceExecutableBinding>(&partial).is_err());
}

#[test]
fn durable_evidence_and_session_continuity_are_not_interchangeable() {
    let evidence = r#"{"root":"/var/lib/pale/evidence","retention_seconds":86400}"#;
    let binding: DurableEvidenceBinding =
        serde_json::from_str(evidence).expect("nonzero evidence retention");
    assert_eq!(binding.retention_seconds.get(), 86_400);
    assert_eq!(serde_json::to_string(&binding).unwrap(), evidence);

    let zero = r#"{"root":"/var/lib/pale/evidence","retention_seconds":0}"#;
    assert!(serde_json::from_str::<DurableEvidenceBinding>(zero).is_err());

    let session_vocabulary = r#"{"root":"/var/lib/pale/evidence","ttl_seconds":86400}"#;
    assert!(serde_json::from_str::<DurableEvidenceBinding>(session_vocabulary).is_err());
    assert!(serde_json::from_str::<SessionStateBinding>(evidence).is_err());

    let session: SessionStateBinding =
        serde_json::from_str(session_vocabulary).expect("nonzero session lifetime");
    assert_eq!(session.ttl_seconds.get(), 86_400);
    assert_eq!(serde_json::to_string(&session).unwrap(), session_vocabulary);
    assert_eq!(SessionTtlSeconds::new(86_400), Some(session.ttl_seconds));
    assert_eq!(SessionTtlSeconds::new(0), None);

    let zero_ttl = r#"{"root":"/var/lib/pale/session","ttl_seconds":0}"#;
    assert!(serde_json::from_str::<SessionStateBinding>(zero_ttl).is_err());
}

#[test]
fn route_record_limit_matches_the_wire_ceiling() {
    let maximum = format!(
        r#"{{"root":"/var/lib/pale/lineage","max_route_records_per_collection":{}}}"#,
        RouteRecordLimit::MAX
    );
    let binding: MaterializationLineageBinding =
        serde_json::from_str(&maximum).expect("maximum route-record limit");
    assert_eq!(
        binding.max_route_records_per_collection.get(),
        RouteRecordLimit::MAX
    );
    assert_eq!(serde_json::to_string(&binding).unwrap(), maximum);

    for invalid in [0, RouteRecordLimit::MAX + 1] {
        let json = format!(
            r#"{{"root":"/var/lib/pale/lineage","max_route_records_per_collection":{invalid}}}"#
        );
        assert!(serde_json::from_str::<MaterializationLineageBinding>(&json).is_err());
    }
}

#[test]
fn absent_session_runtime_is_omitted_and_round_trips() {
    let session: SessionModel = serde_json::from_str(r#"{"activation_roots":["alpha"]}"#)
        .expect("portable session without a machine runtime");
    assert!(session.runtime.is_none());

    let encoded = serde_json::to_value(&session).expect("serialize portable session");
    assert!(
        encoded.get("runtime").is_none(),
        "absence must be encoded by omitting runtime, not by writing null"
    );
    assert_eq!(
        serde_json::from_value::<SessionModel>(encoded).expect("round-trip portable session"),
        session
    );
}
