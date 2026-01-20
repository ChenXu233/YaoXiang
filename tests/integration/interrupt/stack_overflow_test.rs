//! Stack overflow interrupt tests

use std::sync::Arc;
use std::thread;

use yaoxiang::runtime::interrupt::{Interrupt, InterruptState};

/// Test stack overflow interrupt operations.
#[test]
fn test_stack_overflow_operations() {
    let state = InterruptState::new();

    // Initially no interrupt
    assert!(state.check_and_clear().is_none());

    // Set stack overflow
    state.set_stack_overflow();

    // Retrieve and verify
    let intr = state.check_and_clear().expect("Should have interrupt");
    match intr {
        Interrupt::StackOverflow => {
            // Expected
        }
        _ => panic!("Expected StackOverflow interrupt"),
    }

    // Now empty
    assert!(state.check_and_clear().is_none());
}

/// Test stack overflow display format.
#[test]
fn test_stack_overflow_display() {
    let state = InterruptState::new();
    state.set_stack_overflow();

    let intr = state.check_and_clear().unwrap();
    let display = format!("{}", intr);

    assert!(display.contains("stack overflow"));
}

/// Test stack overflow thread safety: concurrent set operations.
#[test]
fn test_stack_overflow_concurrent_set() {
    let state = Arc::new(InterruptState::new());
    let barrier = Arc::new(std::sync::Barrier::new(4));

    // Multiple threads setting stack overflow concurrently
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let state = state.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                state.set_stack_overflow();
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Should have exactly one stack overflow (last set wins)
    let intr = state.check_and_clear().expect("Should have one interrupt");
    match intr {
        Interrupt::StackOverflow => {}
        _ => panic!("Expected StackOverflow"),
    }
}

/// Test that stack overflow can be overwritten by other interrupt types.
#[test]
fn test_stack_overflow_overwrite() {
    let state = InterruptState::new();

    // Set stack overflow
    state.set_stack_overflow();
    assert!(state.has_interrupt());

    // Set timeout (should overwrite since we use atomic store)
    state.set_timeout(std::time::Duration::from_millis(100));

    // Should only have timeout now
    let intr = state.check_and_clear().unwrap();
    match intr {
        Interrupt::Timeout(_) => {}
        _ => panic!("Expected Timeout after overwrite"),
    }
}

/// Test stack overflow overwrites previous interrupt.
#[test]
fn test_stack_overflow_overwrites_previous() {
    let state = InterruptState::new();

    // Set timeout first
    state.set_timeout(std::time::Duration::from_millis(100));
    // Then set stack overflow
    state.set_stack_overflow();

    // Should have stack overflow (last set wins)
    let intr = state.check_and_clear().unwrap();
    match intr {
        Interrupt::StackOverflow => {}
        _ => panic!("Expected StackOverflow after overwrite"),
    }
}
