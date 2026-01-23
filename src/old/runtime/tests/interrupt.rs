//! Interrupt module unit tests

use std::time::Duration;

use crate::runtime::interrupt::{AccessType, BreakpointId, Interrupt, InterruptState};

#[test]
fn test_interrupt_state_timeout() {
    let state = InterruptState::new();
    assert!(state.check_and_clear().is_none());

    state.set_timeout(Duration::from_millis(100));
    let intr = state.check_and_clear().unwrap();
    match intr {
        Interrupt::Timeout(d) => assert_eq!(d.as_millis(), 100),
        _ => panic!("Expected Timeout"),
    }

    assert!(state.check_and_clear().is_none());
}

#[test]
fn test_interrupt_state_breakpoint() {
    let state = InterruptState::new();
    state.set_breakpoint(BreakpointId::new(42));
    let intr = state.check_and_clear().unwrap();
    match intr {
        Interrupt::Breakpoint(id) => assert_eq!(id.inner(), 42),
        _ => panic!("Expected Breakpoint"),
    }
}

#[test]
fn test_interrupt_state_stack_overflow() {
    let state = InterruptState::new();
    state.set_stack_overflow();
    let intr = state.check_and_clear().unwrap();
    match intr {
        Interrupt::StackOverflow => {}
        _ => panic!("Expected StackOverflow"),
    }
}

#[test]
fn test_interrupt_state_memory_violation() {
    let state = InterruptState::new();
    state.set_memory_violation(0xDEADBEEF, AccessType::Write);
    let intr = state.check_and_clear().unwrap();
    match intr {
        Interrupt::MemoryViolation { address, access } => {
            assert_eq!(address, 0xDEADBEEF);
            assert_eq!(access, AccessType::Write);
        }
        _ => panic!("Expected MemoryViolation"),
    }
}

#[test]
fn test_interrupt_state_clear() {
    let state = InterruptState::new();
    state.set_timeout(Duration::from_secs(1));
    assert!(state.has_interrupt());

    state.clear();
    assert!(!state.has_interrupt());
}

#[test]
fn test_breakpoint_id_format() {
    let id = BreakpointId::new(123);
    assert_eq!(format!("{}", id), "breakpoint-123");
}
