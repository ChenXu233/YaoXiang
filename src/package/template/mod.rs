//! Project template generators

mod gitignore;
mod main_yx;

#[cfg(test)]
mod tests;

pub use main_yx::{generate_main_yx, generate_lib_yx};
pub use gitignore::generate_gitignore;
