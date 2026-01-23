//! VM interrupt types and handler
//!
//! Defines external interrupts (Timeout, Breakpoint, StackOverflow, MemoryViolation)
//! that can be injected by the scheduler or external systems.

use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::time::Duration;
use std::fmt;

/// Unique identifier for breakpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BreakpointId(u64);

impl BreakpointId {
    /// Create a new breakpoint ID.
    #[inline]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the inner value.
    #[inline]
    pub fn inner(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for BreakpointId {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "breakpoint-{}", self.0)
    }
}

/// Memory access type for violation reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    Read = 0,
    Write = 1,
    Execute = 2,
}

impl AccessType {
    /// Convert from raw value.
    #[inline]
    pub fn from_raw(value: u8) -> Self {
        match value {
            0 => AccessType::Read,
            1 => AccessType::Write,
            _ => AccessType::Execute,
        }
    }

    /// Get as raw u8 value.
    #[inline]
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

impl fmt::Display for AccessType {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            AccessType::Read => write!(f, "read"),
            AccessType::Write => write!(f, "write"),
            AccessType::Execute => write!(f, "execute"),
        }
    }
}

/// External interrupt types that can be injected into the VM.
///
/// These represent exceptional conditions that occur outside normal execution:
/// - **Timeout**: Execution exceeded the allowed time budget
/// - **Breakpoint**: Debug breakpoint hit
/// - **StackOverflow**: Call stack exceeded maximum depth
/// - **MemoryViolation**: Invalid memory access (read/write/execute on protected address)
#[derive(Debug, Clone)]
pub enum Interrupt {
    /// Execution timeout
    Timeout(Duration),
    /// Debug breakpoint
    Breakpoint(BreakpointId),
    /// Stack overflow
    StackOverflow,
    /// Memory access violation
    MemoryViolation {
        /// The memory address that was accessed
        address: usize,
        /// The type of access that was attempted
        access: AccessType,
    },
}

impl fmt::Display for Interrupt {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Interrupt::Timeout(duration) => {
                write!(f, "timeout after {}ms", duration.as_millis())
            }
            Interrupt::Breakpoint(id) => {
                write!(f, "breakpoint: {}", id)
            }
            Interrupt::StackOverflow => {
                write!(f, "stack overflow")
            }
            Interrupt::MemoryViolation { address, access } => {
                let access_str = match access {
                    AccessType::Read => "read",
                    AccessType::Write => "write",
                    AccessType::Execute => "execute",
                };
                write!(
                    f,
                    "memory violation: {} at address {:#018x}",
                    access_str, address
                )
            }
        }
    }
}

/// Thread-safe interrupt state storage.
///
/// This is the core mechanism for injecting interrupts into the VM.
/// The scheduler or external systems can set interrupts, and the VM
/// checks and clears them at DAG node boundaries.
#[derive(Debug)]
pub struct InterruptState {
    /// Interrupt type (0 = None, 1 = Timeout, 2 = Breakpoint, 3 = StackOverflow, 4 = MemoryViolation)
    interrupt_type: AtomicU8,
    /// First argument (duration_ms for Timeout, address for MemoryViolation, breakpoint_id high bits)
    arg0: AtomicU64,
    /// Second argument (breakpoint_id low bits, access type for MemoryViolation)
    arg1: AtomicU64,
}

impl Default for InterruptState {
    fn default() -> Self {
        Self::new()
    }
}

impl InterruptState {
    /// Create a new empty interrupt state.
    #[inline]
    pub fn new() -> Self {
        Self {
            interrupt_type: AtomicU8::new(0),
            arg0: AtomicU64::new(0),
            arg1: AtomicU64::new(0),
        }
    }

    /// Set a Timeout interrupt.
    #[inline]
    pub fn set_timeout(
        &self,
        duration: Duration,
    ) {
        self.interrupt_type.store(1, Ordering::SeqCst);
        self.arg0
            .store(duration.as_nanos() as u64, Ordering::SeqCst);
    }

    /// Set a Breakpoint interrupt.
    #[inline]
    pub fn set_breakpoint(
        &self,
        id: BreakpointId,
    ) {
        self.interrupt_type.store(2, Ordering::SeqCst);
        self.arg0.store(id.inner(), Ordering::SeqCst);
    }

    /// Set a StackOverflow interrupt.
    #[inline]
    pub fn set_stack_overflow(&self) {
        self.interrupt_type.store(3, Ordering::SeqCst);
    }

    /// Set a MemoryViolation interrupt.
    #[inline]
    pub fn set_memory_violation(
        &self,
        address: usize,
        access: AccessType,
    ) {
        self.interrupt_type.store(4, Ordering::SeqCst);
        self.arg0.store(address as u64, Ordering::SeqCst);
        self.arg1.store(access.as_u8() as u64, Ordering::SeqCst);
    }

    /// Check and clear any pending interrupt.
    ///
    /// Returns `Some(Interrupt)` if an interrupt was pending and has been cleared,
    /// returns `None` if no interrupt was pending.
    ///
    /// This is the primary method the scheduler uses to check for interrupts
    /// at DAG node boundaries.
    #[inline]
    pub fn check_and_clear(&self) -> Option<Interrupt> {
        let interrupt_type = self.interrupt_type.load(Ordering::SeqCst);
        if interrupt_type == 0 {
            return None;
        }

        // Clear the interrupt atomically
        self.interrupt_type.store(0, Ordering::SeqCst);

        Some(match interrupt_type {
            1 => {
                let duration_ns = self.arg0.load(Ordering::SeqCst);
                Interrupt::Timeout(Duration::from_nanos(duration_ns))
            }
            2 => {
                let id = self.arg0.load(Ordering::SeqCst);
                Interrupt::Breakpoint(BreakpointId::new(id))
            }
            3 => Interrupt::StackOverflow,
            4 => {
                let address = self.arg0.load(Ordering::SeqCst) as usize;
                let access_raw = self.arg1.load(Ordering::SeqCst) as u8;
                Interrupt::MemoryViolation {
                    address,
                    access: AccessType::from_raw(access_raw),
                }
            }
            _ => unreachable!("Invalid interrupt type: {}", interrupt_type),
        })
    }

    /// Check if there is a pending interrupt without clearing it.
    #[inline]
    pub fn has_interrupt(&self) -> bool {
        self.interrupt_type.load(Ordering::SeqCst) != 0
    }

    /// Clear any pending interrupt without returning it.
    #[inline]
    pub fn clear(&self) {
        self.interrupt_type.store(0, Ordering::SeqCst);
    }
}

/// Type alias for the interrupt handler (Arc for shared ownership across threads).
pub type InterruptHandler = std::sync::Arc<InterruptState>;
