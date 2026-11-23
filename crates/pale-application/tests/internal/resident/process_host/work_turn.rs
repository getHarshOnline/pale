use std::{cell::RefCell, io, rc::Rc, time::Instant};

use pale_ports::{
    LocalIpcCarrierError, LocalIpcCarrierOperation, LocalIpcFramePort, LocalIpcPeerCredentials,
};

use super::*;
use crate::resident::process_host::{
    frame_service::ResidentFrameProgram,
    program::{ResidentProgramRetainedPhaseTurn, ResidentProgramTurnError},
    resident_frame_ingress::{ResidentFrameIngressError, ResidentProgramFrameOutcome},
};

#[derive(Default)]
struct WorkCounts {
    frames: usize,
    retained_phases: usize,
    recoveries: usize,
    received: Vec<String>,
    retained_phase: bool,
    advance_retained_phase: bool,
}

struct CountingProgram {
    counts: Rc<RefCell<WorkCounts>>,
}

impl ResidentFrameProgram for CountingProgram {
    fn serve_frame(
        &mut self,
        carrier: &mut dyn LocalIpcFramePort,
        carrier_deadline: Instant,
        _: Instant,
    ) -> Result<ResidentProgramFrameOutcome, ResidentFrameIngressError> {
        let bytes = carrier.receive_frame(carrier_deadline)?;
        let label =
            String::from_utf8(bytes.into_vec()).map_err(|_| ResidentFrameIngressError::Frame)?;
        let mut counts = self.counts.borrow_mut();
        counts.frames += 1;
        counts.received.push(label);
        Ok(ResidentProgramFrameOutcome::Operation)
    }

    fn has_retained_phase(&self) -> bool {
        self.counts.borrow().retained_phase
    }

    fn continue_one_retained_phase(&mut self) -> ResidentProgramRetainedPhaseTurn {
        let mut counts = self.counts.borrow_mut();
        if !counts.retained_phase {
            return ResidentProgramRetainedPhaseTurn::Idle;
        }
        counts.retained_phases += 1;
        if counts.advance_retained_phase {
            counts.retained_phase = false;
            ResidentProgramRetainedPhaseTurn::Advanced
        } else {
            ResidentProgramRetainedPhaseTurn::Deferred
        }
    }

    fn recover_one_bounded_source(&mut self) -> Result<(), ResidentProgramTurnError> {
        self.counts.borrow_mut().recoveries += 1;
        Ok(())
    }
}

struct ReadyCarrier {
    inbound: Option<Box<[u8]>>,
    reads: usize,
}

impl ReadyCarrier {
    fn new(label: &str) -> Self {
        Self {
            inbound: Some(label.as_bytes().to_vec().into_boxed_slice()),
            reads: 0,
        }
    }
}

impl LocalIpcFramePort for ReadyCarrier {
    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        Ok(LocalIpcPeerCredentials::new(501, 20, Some(8_081)))
    }

    fn send_frame(&mut self, _: &[u8], _: Instant) -> Result<(), LocalIpcCarrierError> {
        Ok(())
    }

    fn receive_frame(&mut self, _: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        self.reads += 1;
        self.inbound.take().ok_or_else(|| {
            LocalIpcCarrierError::io(
                LocalIpcCarrierOperation::ReceiveFrame,
                io::Error::other("test frame was already consumed"),
            )
        })
    }
}

fn service(counts: Rc<RefCell<WorkCounts>>) -> ResidentFrameService<'static> {
    ResidentFrameService::from_program(CountingProgram { counts })
}

fn frame(carrier: &mut ReadyCarrier) -> ResidentReadyFrame<'_> {
    let deadline = Instant::now();
    ResidentReadyFrame::new(carrier, deadline, deadline)
}

#[test]
fn one_turn_selects_only_one_ready_source_and_rotates_fairly() {
    let counts = Rc::new(RefCell::new(WorkCounts {
        retained_phase: true,
        ..WorkCounts::default()
    }));
    let mut service = service(Rc::clone(&counts));
    let mut coordinator = ResidentWorkTurnCoordinator::new();
    let mut first = ReadyCarrier::new("first-frame");
    let mut second = ReadyCarrier::new("second-frame");

    assert_eq!(
        coordinator
            .run_one(
                &mut service,
                ResidentWorkTurnInput::idle()
                    .with_frame(frame(&mut first))
                    .with_recovery(),
            )
            .unwrap(),
        ResidentWorkTurnOutcome::Frame(ResidentFrameOutcome::Operation)
    );
    assert_eq!(
        (
            counts.borrow().frames,
            counts.borrow().retained_phases,
            counts.borrow().recoveries
        ),
        (1, 0, 0)
    );

    assert_eq!(
        coordinator
            .run_one(
                &mut service,
                ResidentWorkTurnInput::idle()
                    .with_frame(frame(&mut second))
                    .with_recovery(),
            )
            .unwrap(),
        ResidentWorkTurnOutcome::RetainedPhaseDeferred
    );
    assert_eq!(second.reads, 0);
    assert_eq!(
        (
            counts.borrow().frames,
            counts.borrow().retained_phases,
            counts.borrow().recoveries
        ),
        (1, 1, 0)
    );

    assert_eq!(
        coordinator
            .run_one(
                &mut service,
                ResidentWorkTurnInput::idle()
                    .with_frame(frame(&mut second))
                    .with_recovery(),
            )
            .unwrap(),
        ResidentWorkTurnOutcome::RecoveryAdvanced
    );
    assert_eq!(second.reads, 0);
    assert_eq!(
        (
            counts.borrow().frames,
            counts.borrow().retained_phases,
            counts.borrow().recoveries
        ),
        (1, 1, 1)
    );

    assert_eq!(
        coordinator
            .run_one(
                &mut service,
                ResidentWorkTurnInput::idle().with_frame(frame(&mut second)),
            )
            .unwrap(),
        ResidentWorkTurnOutcome::Frame(ResidentFrameOutcome::Operation)
    );
    assert_eq!(second.reads, 1);
    assert_eq!(
        counts.borrow().received,
        vec!["first-frame".to_owned(), "second-frame".to_owned()]
    );
}

#[test]
fn interrupted_start_does_not_permanently_starve_cancel_or_advance_frames() {
    let counts = Rc::new(RefCell::new(WorkCounts {
        retained_phase: true,
        ..WorkCounts::default()
    }));
    let mut service = service(Rc::clone(&counts));
    let mut coordinator = ResidentWorkTurnCoordinator::new();
    let mut warmup = ReadyCarrier::new("warmup");
    let mut cancel = ReadyCarrier::new("cancel");
    let mut advance = ReadyCarrier::new("advance");

    coordinator
        .run_one(
            &mut service,
            ResidentWorkTurnInput::idle().with_frame(frame(&mut warmup)),
        )
        .unwrap();
    assert_eq!(coordinator.next_source(), ResidentWorkSource::RetainedPhase);

    assert_eq!(
        coordinator
            .run_one(
                &mut service,
                ResidentWorkTurnInput::idle().with_frame(frame(&mut cancel)),
            )
            .unwrap(),
        ResidentWorkTurnOutcome::RetainedPhaseDeferred
    );
    assert_eq!(cancel.reads, 0);
    assert_eq!(
        coordinator
            .run_one(
                &mut service,
                ResidentWorkTurnInput::idle().with_frame(frame(&mut cancel)),
            )
            .unwrap(),
        ResidentWorkTurnOutcome::Frame(ResidentFrameOutcome::Operation)
    );

    assert_eq!(
        coordinator
            .run_one(
                &mut service,
                ResidentWorkTurnInput::idle().with_frame(frame(&mut advance)),
            )
            .unwrap(),
        ResidentWorkTurnOutcome::RetainedPhaseDeferred
    );
    assert_eq!(advance.reads, 0);
    assert_eq!(
        coordinator
            .run_one(
                &mut service,
                ResidentWorkTurnInput::idle().with_frame(frame(&mut advance)),
            )
            .unwrap(),
        ResidentWorkTurnOutcome::Frame(ResidentFrameOutcome::Operation)
    );

    assert_eq!(
        counts.borrow().received,
        vec![
            "warmup".to_owned(),
            "cancel".to_owned(),
            "advance".to_owned()
        ]
    );
    assert_eq!(counts.borrow().retained_phases, 2);
}

#[test]
fn completed_retained_phase_leaves_later_sources_eligible() {
    let counts = Rc::new(RefCell::new(WorkCounts {
        retained_phase: true,
        advance_retained_phase: true,
        ..WorkCounts::default()
    }));
    let mut service = service(Rc::clone(&counts));
    let mut coordinator = ResidentWorkTurnCoordinator {
        next_source: ResidentWorkSource::RetainedPhase,
    };

    assert_eq!(
        coordinator
            .run_one(&mut service, ResidentWorkTurnInput::idle())
            .unwrap(),
        ResidentWorkTurnOutcome::RetainedPhaseAdvanced
    );
    assert!(!counts.borrow().retained_phase);
    assert_eq!(coordinator.next_source(), ResidentWorkSource::Recovery);
}

#[test]
fn idle_turn_has_no_effect_and_does_not_move_the_fairness_cursor() {
    let counts = Rc::new(RefCell::new(WorkCounts::default()));
    let mut service = service(Rc::clone(&counts));
    let mut coordinator = ResidentWorkTurnCoordinator::new();

    assert_eq!(
        coordinator
            .run_one(&mut service, ResidentWorkTurnInput::idle())
            .unwrap(),
        ResidentWorkTurnOutcome::Idle
    );
    let counts = counts.borrow();
    assert_eq!(
        (counts.frames, counts.retained_phases, counts.recoveries),
        (0, 0, 0)
    );
    assert!(counts.received.is_empty());
    assert_eq!(coordinator.next_source(), ResidentWorkSource::Frame);
}
