//! YaoXiang Package Manager
//!
//! Provides package management functionality including project initialization,
//! dependency management, and lock file generation.

pub mod commands;
pub mod dependency;
pub mod error;
pub mod lock;
pub mod manifest;
pub mod source;
pub mod template;
pub mod vendor;

pub use error::{PackageError, PackageResult};
pub use manifest::PackageManifest;
pub use lock::LockFile;
pub use dependency::DependencySpec;
