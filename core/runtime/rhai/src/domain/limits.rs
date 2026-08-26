/// Encapsulates execution and memory ceilings to prevent denial of service and infinite loops (PRD-002:78-79, C-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionLimits {
    max_operations: u64,
    max_call_stack_depth: usize,
    max_expr_depth: usize,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionLimits {
    /// Creates a default set of execution limits (100k operations, 64 stack depth, 64 expr depth).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_operations: 100_000,
            max_call_stack_depth: 64,
            max_expr_depth: 64,
        }
    }

    /// Sets the maximum number of evaluation steps/operations allowed before termination.
    #[must_use]
    pub const fn with_max_operations(mut self, max_operations: u64) -> Self {
        self.max_operations = max_operations;
        self
    }

    /// Sets the maximum call stack depth.
    #[must_use]
    pub const fn with_max_call_stack_depth(mut self, max_call_stack_depth: usize) -> Self {
        self.max_call_stack_depth = max_call_stack_depth;
        self
    }

    /// Sets the maximum expression nesting depth.
    #[must_use]
    pub const fn with_max_expr_depth(mut self, max_expr_depth: usize) -> Self {
        self.max_expr_depth = max_expr_depth;
        self
    }

    /// Returns the maximum allowed operations.
    #[must_use]
    pub const fn max_operations(&self) -> u64 {
        self.max_operations
    }

    /// Applies these limits to a Rhai `Engine` instance.
    pub fn apply_to(&self, engine: &mut rhai::Engine) {
        engine.set_max_operations(self.max_operations);
        engine.set_max_call_levels(self.max_call_stack_depth);
        engine.set_max_expr_depths(self.max_expr_depth, self.max_expr_depth);
    }
}
