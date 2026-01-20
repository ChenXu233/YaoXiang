//! Memory violation interrupt tests

use std::sync::Arc;
use std::thread;

use yaoxiang::runtime::interrupt::{AccessType, Interrupt, InterruptState};

/// Test memory violation interrupt operations.
#[test]
fn test_memory_violation_operations() {
    let state = InterruptState::new();

    // Initially no interrupt
    assert!(state.check_and_clear().is_none());

    // Set memory violation
    state.set_memory_violation(0xDEADBEEF, AccessType::Write);

    // Retrieve and verify
    let intr = state.check_and_clear().expect("Should have interrupt");
    match intr {
        Interrupt::MemoryViolation { address, access } => {
            assert_eq!(address, 0xDEADBEEF);
            assert_eq!(access, AccessType::Write);
        }
        _ => panic!("Expected MemoryViolation interrupt"),
    }

    // Now empty
    assert!(state.check_and_clear().is_none());
}

/// Test all access types.
#[test]
fn test_all_access_types() {
    let state = InterruptState::new();

    for (addr, access) in [
        (0x1000, AccessType::Read),
        (0x2000, AccessType::Write),
        (0x3000, AccessType::Execute),
    ] {
        state.set_memory_violation(addr, access);
        let intr = state.check_and_clear().unwrap();
        match intr {
            Interrupt::MemoryViolation {
                address,
                access: got_access,
            } => {
                assert_eq!(address, addr);
                assert_eq!(access, got_access);
            }
            _ => panic!("Expected MemoryViolation"),
        }
    }
}

/// Test memory violation display format.
#[test]
fn test_memory_violation_display() {
    let state = InterruptState::new();
    state.set_memory_violation(0xDEAD_BEEF, AccessType::Read);

    let intr = state.check_and_clear().unwrap();
    let display = format!("{}", intr);

    assert!(display.contains("read"));
    // Format is {:#018x} = 0x00000000deadbeef
    assert!(display.contains("0x") && display.contains("deadbeef"));
}

/// Test memory violation thread safety: concurrent set operations.
#[test]
fn test_memory_violation_concurrent_set() {
    let state = Arc::new(InterruptState::new());
    let barrier = Arc::new(std::sync::Barrier::new(4));

    // Multiple threads setting memory violations concurrently
    let handles: Vec<_> = (0..4)
        .map(|i| {
            let state = state.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                state.set_memory_violation(i * 0x1000, AccessType::Write);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Should have exactly one memory violation (last set wins)
    let intr = state.check_and_clear().expect("Should have one interrupt");
    match intr {
        Interrupt::MemoryViolation { address, access } => {
            // Address should be one of 0x1000, 0x2000, 0x3000, or 0x4000 (last set wins)
            assert!(address < 0x5000);
            assert_eq!(access, AccessType::Write);
        }
        _ => panic!("Expected MemoryViolation"),
    }
}

/// Test memory violations can be overwritten.
#[test]
fn test_memory_violation_overwrite() {
    let state = InterruptState::new();

    state.set_memory_violation(0x1000, AccessType::Read);
    state.set_memory_violation(0x2000, AccessType::Write);

    let intr = state.check_and_clear().unwrap();
    match intr {
        Interrupt::MemoryViolation { address, access } => {
            assert_eq!(address, 0x2000);
            assert_eq!(access, AccessType::Write);
        }
        _ => panic!("Expected MemoryViolation"),
    }
}

/// Test special address values.
#[test]
fn test_special_addresses() {
    let state = InterruptState::new();

    for addr in [0u64, 1, usize::MAX as u64 / 2, usize::MAX as u64] {
        state.set_memory_violation(addr as usize, AccessType::Write);
        let intr = state.check_and_clear().unwrap();
        match intr {
            Interrupt::MemoryViolation { address, .. } => assert_eq!(address, addr as usize),
            _ => panic!("Expected MemoryViolation"),
        }
    }
}

/// Test access type conversion.
#[test]
fn test_access_type_conversion() {
    assert_eq!(AccessType::from_raw(0), AccessType::Read);
    assert_eq!(AccessType::from_raw(1), AccessType::Write);
    assert_eq!(AccessType::from_raw(2), AccessType::Execute);
    assert_eq!(AccessType::from_raw(255), AccessType::Execute); // Invalid maps to Execute

    assert_eq!(AccessType::Read.as_u8(), 0);
    assert_eq!(AccessType::Write.as_u8(), 1);
    assert_eq!(AccessType::Execute.as_u8(), 2);
}
