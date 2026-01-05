//! VM call frames

use super::executor::Value;

/// Call frame
#[derive(Debug, Clone)]
pub struct Frame {
    /// Function name
    pub name: String,
    /// Return address
    pub return_addr: usize,
    /// Saved frame pointer
    pub saved_fp: usize,
    /// Local variables
    pub locals: Vec<Value>,
}

impl Frame {
    /// Create a new frame
    pub fn new(
        name: String,
        return_addr: usize,
        saved_fp: usize,
        locals: Vec<Value>,
    ) -> Self {
        Self {
            name,
            return_addr,
            saved_fp,
            locals,
        }
    }
}
