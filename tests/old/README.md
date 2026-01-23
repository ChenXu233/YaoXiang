# Old Tests Directory

This directory contains tests for the deprecated runtime and VM systems that have been moved to `src/old/` during the architecture refactor.

## Contents

### integration/
Tests for the old runtime interrupt system:
- `interrupt.rs` - Module declarations
- `interrupt/breakpoint_test.rs` - Breakpoint interrupt tests
- `interrupt/memory_violation_test.rs` - Memory violation tests
- `interrupt/stack_overflow_test.rs` - Stack overflow tests
- `interrupt/timeout_test.rs` - Timeout tests

These tests are preserved for reference but are no longer run as part of the test suite.

## Purpose

These tests document the behavior of the old interrupt system that was part of the runtime module. They serve as historical reference and can be used to understand the evolution of the YaoXiang runtime.

## Status

**DEPRECATED** - These tests are not maintained and may not compile with the current codebase. They are kept solely for historical reference.
