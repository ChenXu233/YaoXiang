//! Timeout interrupt tests

use std::sync::Arc;
use std::time::Duration;
use std::thread;

use yaoxiang::runtime::interrupt::{Interrupt, InterruptState};

/// Test that timeout interrupt can be set and retrieved.
#[test]
fn test_timeout_interrupt_set_and_get() {
    let state = InterruptState::new();

    // Initially no interrupt
    assert!(state.check_and_clear().is_none());

    // Set timeout
    state.set_timeout(Duration::from_millis(500));

    // Retrieve and verify
    let intr = state.check_and_clear().expect("Should have interrupt");
    match intr {
        Interrupt::Timeout(duration) => {
            assert_eq!(duration.as_millis(), 500);
        }
        _ => panic!("Expected Timeout interrupt"),
    }

    // Now empty
    assert!(state.check_and_clear().is_none());
}

/// Test timeout with different durations.
#[test]
fn test_timeout_durations() {
    let state = InterruptState::new();

    // Test various durations
    for ms in [1u64, 10, 100, 1000, 5000] {
        state.set_timeout(Duration::from_millis(ms));
        let intr = state.check_and_clear().unwrap();
        match intr {
            Interrupt::Timeout(d) => assert_eq!(d.as_millis() as u64, ms),
            _ => panic!("Expected Timeout"),
        }
    }
}

/// Test thread safety: concurrent set and check operations.
#[test]
fn test_timeout_concurrent_set_check() {
    let state = Arc::new(InterruptState::new());
    let barrier = Arc::new(std::sync::Barrier::new(4));

    // Spawn multiple threads to concurrently set timeouts
    let handles: Vec<_> = (0..4)
        .map(|i| {
            let state = state.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                // Each thread sets a unique timeout
                state.set_timeout(Duration::from_millis((i + 1) as u64 * 1000));
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Should have exactly one interrupt (last set wins)
    let intr = state.check_and_clear().expect("Should have one interrupt");
    match intr {
        Interrupt::Timeout(duration) => {
            // Duration will be one of 1000, 2000, 3000, or 4000 (last set wins)
            assert!(duration.as_secs() >= 1 && duration.as_secs() <= 4);
        }
        _ => panic!("Expected Timeout"),
    }

    // Now empty
    assert!(state.check_and_clear().is_none());
}

/// Test that last timeout overwrites previous ones.
#[test]
fn test_timeout_overwrite() {
    let state = InterruptState::new();

    // Set multiple timeouts
    state.set_timeout(Duration::from_millis(100));
    state.set_timeout(Duration::from_millis(200));
    state.set_timeout(Duration::from_millis(300));

    // Should only have the last one
    let intr = state.check_and_clear().unwrap();
    match intr {
        Interrupt::Timeout(d) => assert_eq!(d.as_millis(), 300),
        _ => panic!("Expected Timeout"),
    }
}

/// Test timeout display format.
#[test]
fn test_timeout_display() {
    let state = InterruptState::new();
    state.set_timeout(Duration::from_millis(1234));

    let intr = state.check_and_clear().unwrap();
    let display = format!("{}", intr);

    assert!(display.contains("1234ms") || display.contains("1.234s"));
}
