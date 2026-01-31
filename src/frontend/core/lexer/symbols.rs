//! Symbol table management
//! Unified symbol management for RFC-004, RFC-010, and RFC-011 support

/// Symbol table entry
#[derive(Debug, Clone)]
pub struct SymbolEntry {
    pub name: String,
    pub kind: SymbolKind,
    pub arity: Option<usize>,
}

/// Kind of symbol
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    // Basic symbols
    Variable,
    Function,
    Type,

    // RFC-010: Generic symbols
    GenericFunction,
    GenericType,
    TypeClass,
    Trait,

    // RFC-011: Advanced type system symbols
    ConstGeneric,
    HigherKindedType,
    TypeFamily,

    // RFC-004: Binding symbols
    Binding,
    PositionBinding,
}

/// Symbol table for managing identifiers
pub struct SymbolTable {
    symbols: Vec<SymbolEntry>,
}

impl SymbolTable {
    /// Create new empty symbol table
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
        }
    }

    /// Insert a symbol
    pub fn insert(
        &mut self,
        name: String,
        kind: SymbolKind,
    ) {
        self.symbols.push(SymbolEntry {
            name,
            kind,
            arity: None,
        });
    }

    /// Insert a symbol with arity
    pub fn insert_with_arity(
        &mut self,
        name: String,
        kind: SymbolKind,
        arity: usize,
    ) {
        self.symbols.push(SymbolEntry {
            name,
            kind,
            arity: Some(arity),
        });
    }

    /// Lookup a symbol by name
    pub fn lookup(
        &self,
        name: &str,
    ) -> Option<&SymbolEntry> {
        self.symbols.iter().rev().find(|s| s.name == name)
    }

    /// Check if symbol exists
    pub fn contains(
        &self,
        name: &str,
    ) -> bool {
        self.lookup(name).is_some()
    }

    /// Get all symbols of a specific kind
    pub fn get_by_kind(
        &self,
        kind: &SymbolKind,
    ) -> Vec<&SymbolEntry> {
        self.symbols.iter().filter(|s| &s.kind == kind).collect()
    }

    /// Clear the symbol table
    pub fn clear(&mut self) {
        self.symbols.clear();
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

/// RFC-004: Binding position validator
pub struct BindingValidator {
    max_positions: usize,
}

impl BindingValidator {
    /// Create new validator
    pub fn new(max_positions: usize) -> Self {
        Self { max_positions }
    }

    /// Validate binding positions
    /// Returns Ok if positions are valid, Err with error message otherwise
    pub fn validate_positions(
        &self,
        positions: &[i32],
    ) -> Result<(), String> {
        for &pos in positions {
            if pos < 0 {
                return Err(format!("Negative position index: {}", pos));
            }
            if pos as usize >= self.max_positions {
                return Err(format!(
                    "Position index {} exceeds maximum allowed positions {}",
                    pos, self.max_positions
                ));
            }
        }
        Ok(())
    }

    /// Validate binding syntax
    /// Supports RFC-004 binding syntax: function[0, 1, 2]
    pub fn validate_binding_syntax(
        &self,
        binding: &str,
    ) -> Result<(), String> {
        // Check for valid binding syntax pattern
        if !binding.contains('[') || !binding.contains(']') {
            return Err("Invalid binding syntax: missing brackets".to_string());
        }

        // Extract position list
        let positions_str = binding
            .split('[')
            .nth(1)
            .ok_or("Invalid binding syntax")?
            .trim_end_matches(']');

        // Parse positions
        let positions: Result<Vec<i32>, _> =
            positions_str.split(',').map(|s| s.trim().parse()).collect();

        let positions = positions.map_err(|_| "Invalid position value")?;

        // Validate positions
        self.validate_positions(&positions)?;

        Ok(())
    }
}

/// RFC-010: Generic parameter validator
pub struct GenericValidator {
    max_type_params: usize,
    max_const_params: usize,
}

impl GenericValidator {
    /// Create new validator
    pub fn new(
        max_type_params: usize,
        max_const_params: usize,
    ) -> Self {
        Self {
            max_type_params,
            max_const_params,
        }
    }

    /// Validate generic parameter list
    /// Supports RFC-010 syntax: [T], [T: Clone], [T, U], [T, N: Int]
    pub fn validate_generic_params(
        &self,
        params: &[String],
    ) -> Result<(), String> {
        if params.len() > self.max_type_params {
            return Err(format!(
                "Too many type parameters: {}, maximum is {}",
                params.len(),
                self.max_type_params
            ));
        }

        for param in params {
            if param.trim().is_empty() {
                return Err("Empty generic parameter".to_string());
            }
        }

        Ok(())
    }

    /// Validate generic constraint
    /// Supports RFC-010 syntax: T: Clone + Add
    pub fn validate_constraint(
        &self,
        constraint: &str,
    ) -> Result<(), String> {
        // Basic constraint validation
        if !constraint.contains(':') {
            return Err("Invalid constraint syntax: missing ':'".to_string());
        }

        // Check for valid trait names (basic validation)
        let parts: Vec<&str> = constraint.split(':').collect();
        if parts.len() != 2 {
            return Err("Invalid constraint syntax".to_string());
        }

        let param_name = parts[0].trim();
        let traits = parts[1].trim();

        if param_name.is_empty() {
            return Err("Empty parameter name in constraint".to_string());
        }

        // Validate trait names
        for trait_name in traits.split('+') {
            let trait_name = trait_name.trim();
            if trait_name.is_empty() {
                return Err("Empty trait name in constraint".to_string());
            }
            // Basic identifier validation
            if !trait_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Err(format!("Invalid trait name: {}", trait_name));
            }
        }

        Ok(())
    }
}

/// RFC-011: Type system validator
pub struct TypeSystemValidator {
    max_type_depth: usize,
    max_constraint_count: usize,
}

impl TypeSystemValidator {
    /// Create new validator
    pub fn new(
        max_type_depth: usize,
        max_constraint_count: usize,
    ) -> Self {
        Self {
            max_type_depth,
            max_constraint_count,
        }
    }

    /// Validate type expression complexity
    pub fn validate_type_complexity(
        &self,
        type_str: &str,
    ) -> Result<(), String> {
        let depth = self.calculate_nesting_depth(type_str);
        if depth > self.max_type_depth {
            return Err(format!(
                "Type nesting too deep: {}, maximum is {}",
                depth, self.max_type_depth
            ));
        }
        Ok(())
    }

    /// Calculate nesting depth of a type expression
    fn calculate_nesting_depth(
        &self,
        type_str: &str,
    ) -> usize {
        let mut max_depth = 0;
        let mut current_depth: usize = 0;

        for ch in type_str.chars() {
            match ch {
                '<' | '{' | '(' => {
                    current_depth += 1;
                    max_depth = max_depth.max(current_depth);
                }
                '>' | '}' | ')' => {
                    current_depth = current_depth.saturating_sub(1);
                }
                _ => {}
            }
        }

        max_depth
    }

    /// Validate constraint count
    pub fn validate_constraint_count(
        &self,
        constraints: &[String],
    ) -> Result<(), String> {
        if constraints.len() > self.max_constraint_count {
            return Err(format!(
                "Too many constraints: {}, maximum is {}",
                constraints.len(),
                self.max_constraint_count
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_validator() {
        let validator = BindingValidator::new(10);

        // Valid binding
        assert!(validator.validate_binding_syntax("func[0, 1, 2]").is_ok());

        // Invalid binding - missing brackets
        assert!(validator.validate_binding_syntax("func").is_err());

        // Invalid binding - negative position
        assert!(validator.validate_binding_syntax("func[-1, 0]").is_err());

        // Invalid binding - position too large
        assert!(validator.validate_binding_syntax("func[0, 15]").is_err());
    }

    #[test]
    fn test_generic_validator() {
        let validator = GenericValidator::new(10, 5);

        // Valid generic params
        assert!(validator
            .validate_generic_params(&["T".to_string()])
            .is_ok());
        assert!(validator
            .validate_generic_params(&["T".to_string(), "U".to_string()])
            .is_ok());

        // Valid constraints
        assert!(validator.validate_constraint("T: Clone").is_ok());
        assert!(validator.validate_constraint("T: Clone + Add").is_ok());

        // Invalid constraints
        assert!(validator.validate_constraint("Clone").is_err());
    }

    #[test]
    fn test_type_system_validator() {
        let validator = TypeSystemValidator::new(10, 5);

        // Valid type
        assert!(validator.validate_type_complexity("Vec<T>").is_ok());

        // Invalid type - too deep (11 levels, exceeds max of 10)
        let deep_type = "Vec<Vec<Vec<Vec<Vec<Vec<Vec<Vec<Vec<Vec<Vec<T>>>>>>>>>>>>";
        assert!(validator.validate_type_complexity(deep_type).is_err());

        // Valid constraints
        assert!(validator
            .validate_constraint_count(&["Clone".to_string()])
            .is_ok());

        // Invalid constraints - too many
        let many_constraints = vec!["Clone".to_string(); 6];
        assert!(validator
            .validate_constraint_count(&many_constraints)
            .is_err());
    }
}
