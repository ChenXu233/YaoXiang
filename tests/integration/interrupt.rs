//! Interrupt handling tests
//!
//! Tests for VM interrupt types, handler, and scheduler integration.

#[path = "interrupt/breakpoint_test.rs"]
mod breakpoint_test;
#[path = "interrupt/memory_violation_test.rs"]
mod memory_violation_test;
#[path = "interrupt/stack_overflow_test.rs"]
mod stack_overflow_test;
#[path = "interrupt/timeout_test.rs"]
mod timeout_test;
