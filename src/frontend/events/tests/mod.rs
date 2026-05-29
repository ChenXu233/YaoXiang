//! Event system tests

use std::any::Any;
use crate::frontend::events::*;

#[test]
fn test_event_type_variants() {
    assert!(matches!(EventType::Base, EventType::Base));
    assert!(matches!(EventType::Phase, EventType::Phase));
    assert!(matches!(EventType::Progress, EventType::Progress));
    assert!(matches!(EventType::Diagnostic, EventType::Diagnostic));
    assert!(matches!(EventType::All, EventType::All));
}

#[test]
fn test_event_metadata_default() {
    let m = EventMetadata::default();
    assert_eq!(m.sequence, 0);
    assert!(m.source_file.is_none());
}

#[test]
fn test_event_metadata_with_source() {
    let m = EventMetadata::with_source("test.yx");
    assert_eq!(m.source_file, Some("test.yx".to_string()));
}

#[test]
fn test_compilation_phase_display() {
    assert_eq!(CompilationPhase::Lexing.to_string(), "lexing");
    assert_eq!(CompilationPhase::Parsing.to_string(), "parsing");
    assert_eq!(CompilationPhase::TypeChecking.to_string(), "type checking");
    assert_eq!(CompilationPhase::IRGeneration.to_string(), "IR generation");
    assert_eq!(CompilationPhase::Full.to_string(), "full compilation");
}

#[test]
fn test_phase_start_event() {
    let event = PhaseStart::new(CompilationPhase::Parsing);
    assert_eq!(event.phase(), CompilationPhase::Parsing);
    assert_eq!(event.event_type(), EventType::Phase);
    assert_eq!(event.name(), "PhaseStart");
}

#[test]
fn test_phase_complete_event() {
    let event = PhaseComplete::new(CompilationPhase::TypeChecking, 42);
    assert_eq!(event.phase(), CompilationPhase::TypeChecking);
    assert_eq!(event.duration_ms(), 42);
    assert_eq!(event.event_type(), EventType::Phase);
}

#[test]
fn test_error_occurred_event() {
    use crate::frontend::events::base::ErrorLevel;
    let event = ErrorOccurred::new("test", "E0001", ErrorLevel::Error);
    assert_eq!(event.event_type(), EventType::Base);
}

#[test]
fn test_null_emitter() {
    let mut emitter = NullEmitter;
    emitter.emit(PhaseStart::new(CompilationPhase::Full));
    emitter.emit_with(
        PhaseStart::new(CompilationPhase::Parsing),
        EventMetadata::default(),
    );
    assert_eq!(emitter.next_sequence(), 0);
}

#[test]
fn test_event_bus_new() {
    let mut bus = EventBus::new();
    bus.emit(PhaseStart::new(CompilationPhase::Parsing));
}

#[test]
fn test_event_bus_subscribe() {
    struct TestSubscriber;
    impl EventSubscriber for TestSubscriber {
        fn event_types(&self) -> Vec<EventType> {
            vec![EventType::Phase]
        }
        fn on_event(
            &self,
            _event: &dyn Any,
            _meta: &EventMetadata,
        ) {
        }
    }
    let mut bus = EventBus::new();
    bus.subscribe(TestSubscriber);
    bus.emit(PhaseStart::new(CompilationPhase::Lexing));
}

#[test]
fn test_console_logger() {
    let logger = ConsoleLogger;
    assert_eq!(logger.event_types(), vec![EventType::All]);
}
