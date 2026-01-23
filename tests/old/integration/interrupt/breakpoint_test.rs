//! Breakpoint interrupt tests

use std::sync::Arc;
use std::thread;

use yaoxiang::runtime::interrupt::{BreakpointId, Interrupt, InterruptState};

/// Test breakpoint interrupt operations.
#[test]
fn test_breakpoint_operations() {
    let state = InterruptState::new();

    // Initially no interrupt
    assert!(state.check_and_clear().is_none());

    // Set breakpoint
    state.set_breakpoint(BreakpointId::new(42));

    // Retrieve and verify
    let intr = state.check_and_clear().expect("Should have interrupt");
    match intr {
        Interrupt::Breakpoint(id) => {
            assert_eq!(id.inner(), 42);
        }
        _ => panic!("Expected Breakpoint interrupt"),
    }

    // Now empty
    assert!(state.check_and_clear().is_none());
}

/// Test breakpoint with various IDs.
#[test]
fn test_breakpoint_ids() {
    let state = InterruptState::new();

    // Test various breakpoint IDs
    for id in [0, 1, 100, 1000, u64::MAX] {
        state.set_breakpoint(BreakpointId::new(id));
        let intr = state.check_and_clear().unwrap();
        match intr {
            Interrupt::Breakpoint(bpid) => assert_eq!(bpid.inner(), id),
            _ => panic!("Expected Breakpoint"),
        }
    }
}

/// Test breakpoint display format.
#[test]
fn test_breakpoint_display() {
    let state = InterruptState::new();
    state.set_breakpoint(BreakpointId::new(123));

    let intr = state.check_and_clear().unwrap();
    let display = format!("{}", intr);

    assert!(display.contains("breakpoint-123"));
}

/// Test breakpoint thread safety: concurrent set operations.
#[test]
fn test_breakpoint_concurrent_set() {
    let state = Arc::new(InterruptState::new());
    let barrier = Arc::new(std::sync::Barrier::new(4));

    // Multiple threads setting breakpoints concurrently
    let handles: Vec<_> = (0..4)
        .map(|i| {
            let state = state.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                state.set_breakpoint(BreakpointId::new(i));
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Should have exactly one breakpoint (last set wins)
    let intr = state.check_and_clear().expect("Should have one interrupt");
    match intr {
        Interrupt::Breakpoint(id) => {
            // ID should be 0, 1, 2, or 3 (last set wins)
            assert!(id.inner() < 4);
        }
        _ => panic!("Expected Breakpoint"),
    }
}

/// Test that breakpoints can be overwritten.
#[test]
fn test_breakpoint_overwrite() {
    let state = InterruptState::new();

    state.set_breakpoint(BreakpointId::new(1));
    state.set_breakpoint(BreakpointId::new(2));
    state.set_breakpoint(BreakpointId::new(3));

    let intr = state.check_and_clear().unwrap();
    match intr {
        Interrupt::Breakpoint(id) => assert_eq!(id.inner(), 3),
        _ => panic!("Expected Breakpoint"),
    }
}
