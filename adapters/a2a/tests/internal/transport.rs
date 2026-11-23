//! Evidence-accounting laws, not proof of live HTTP or cancellation.

use super::*;

#[test]
fn evidence_charges_json_and_sse_coordinates_across_items() {
    let evidence = A2aExchangeEvidence {
        facts: A2aExchangeFacts::DirectMessage,
        bytes: b"{}".to_vec(),
        sse: Some(("α".to_owned(), "update".to_owned())),
    };
    let length = 2 + "α".len() + "update".len();
    let mut used = 0;
    let maximum = NonZeroUsize::new(length * 2).unwrap();
    charge(&evidence, &mut used, maximum).unwrap();
    assert_eq!(used, length);
    charge(&evidence, &mut used, maximum).unwrap();
    assert_eq!(used, length * 2);
    assert_eq!(
        charge(&evidence, &mut used, maximum),
        Err(A2aExchangeError::LimitExceeded)
    );
    assert_eq!(used, length * 2);
}

#[test]
fn projected_task_ids_and_remote_codes_are_not_free_evidence() {
    let task = pale_core::records::A2aRemoteTaskIdV1::parse("task-native".to_owned()).unwrap();
    let context =
        pale_core::records::A2aRemoteContextIdV1::parse("context-native".to_owned()).unwrap();
    let expected = task.as_str().len() + context.as_str().len() + 2;
    let evidence = A2aExchangeEvidence {
        facts: A2aExchangeFacts::Artifact { task, context },
        bytes: b"{}".to_vec(),
        sse: None,
    };
    let mut used = 0;
    charge(&evidence, &mut used, NonZeroUsize::new(expected).unwrap()).unwrap();
    assert_eq!(used, expected);
    let error = A2aExchangeEvidence {
        facts: A2aExchangeFacts::RemoteError { code: -32000 },
        bytes: b"{}".to_vec(),
        sse: None,
    };
    assert_eq!(
        charge(&error, &mut 0, NonZeroUsize::new(9).unwrap()),
        Err(A2aExchangeError::LimitExceeded)
    );
}
