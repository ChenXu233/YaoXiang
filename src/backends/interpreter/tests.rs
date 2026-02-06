//! Runtime Value Weak Tests

use crate::backends::common::RuntimeValue;
use std::sync::Arc;

/// Test Weak creation from Arc
#[test]
fn test_weak_from_arc() {
    let value = RuntimeValue::Int(42);
    let arc_value: Arc<RuntimeValue> = Arc::new(value);
    let arc_rt = RuntimeValue::Arc(arc_value);
    let weak = RuntimeValue::from_arc_into_weak(arc_rt);

    match weak {
        RuntimeValue::Weak(_) => {
            // Success - Weak was created
        }
        _ => panic!("Expected Weak variant"),
    }
}

/// Test Weak upgrade returns Some when Arc is alive
#[test]
fn test_weak_upgrade_some() {
    let value = RuntimeValue::Int(42);
    let arc_inner: Arc<RuntimeValue> = Arc::new(value);
    let arc_rt = RuntimeValue::Arc(arc_inner.clone());
    let weak = RuntimeValue::from_arc_into_weak(arc_rt);

    // Upgrade should return Some since Arc is still alive
    let upgraded = weak.upgrade();
    assert!(
        upgraded.is_some(),
        "Upgrade returned None, Arc may have been dropped"
    );

    if let Some(arc2) = upgraded {
        match arc2 {
            RuntimeValue::Arc(inner) => {
                // Verify the value is correct
                match &*inner {
                    RuntimeValue::Int(42) => {}
                    _ => panic!("Expected Int(42) inside Arc"),
                }
            }
            _ => panic!("Expected Arc variant after upgrade"),
        }
    }
}

/// Test Weak upgrade returns None after Arc is dropped
#[test]
fn test_weak_upgrade_none() {
    // Create Weak while Arc is alive
    let weak = {
        let value = RuntimeValue::Int(42);
        let arc_inner: Arc<RuntimeValue> = Arc::new(value);
        let arc_rt = RuntimeValue::Arc(arc_inner);
        RuntimeValue::from_arc_into_weak(arc_rt)
        // Arc is dropped here when it goes out of scope
    };

    // Upgrade should return None since Arc was dropped
    let upgraded = weak.upgrade();
    assert!(
        upgraded.is_none(),
        "Upgrade should return None after Arc is dropped"
    );
}
