//! Project template generators

mod gitignore;
mod main_yx;

pub use main_yx::generate_main_yx;
pub use gitignore::generate_gitignore;
