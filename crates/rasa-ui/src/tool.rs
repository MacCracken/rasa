/// A pluggable tool that can appear in the tool palette.
///
/// Built-in tools (brush, eraser, etc.) implement this trait, and
/// third-party plugins can register additional tools at runtime.
pub trait Tool: Send + Sync {
    /// Unique name for display and lookup.
    fn name(&self) -> &str;

    /// Keyboard shortcut key (e.g. "B" for brush).
    fn shortcut(&self) -> &str;

    /// Single-character label shown on the palette button.
    fn icon_label(&self) -> &str;
}

/// Runtime registry of available [`Tool`] implementations.
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
    active_idx: usize,
}

impl ToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            active_idx: 0,
        }
    }

    /// Register a new tool.
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    /// Look up a tool by name.
    pub fn tool_by_name(&self, name: &str) -> Option<&dyn Tool> {
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .map(|t| t.as_ref())
    }

    /// List all registered tools.
    pub fn list_tools(&self) -> Vec<&dyn Tool> {
        self.tools.iter().map(|t| t.as_ref()).collect()
    }

    /// List the names of all registered tools.
    pub fn list_names(&self) -> Vec<&str> {
        self.tools.iter().map(|t| t.name()).collect()
    }

    /// Get the currently active tool.
    pub fn active(&self) -> Option<&dyn Tool> {
        self.tools.get(self.active_idx).map(|t| t.as_ref())
    }

    /// Set the active tool by name. Returns `true` if found.
    pub fn set_active_by_name(&mut self, name: &str) -> bool {
        if let Some(idx) = self.tools.iter().position(|t| t.name() == name) {
            self.active_idx = idx;
            true
        } else {
            false
        }
    }

    /// Set the active tool by index.
    ///
    /// # Panics
    /// Panics if `idx` is out of bounds.
    pub fn set_active_by_index(&mut self, idx: usize) {
        assert!(idx < self.tools.len(), "tool index out of bounds");
        self.active_idx = idx;
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Whether the registry contains no tools.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubTool {
        label: String,
    }

    impl Tool for StubTool {
        fn name(&self) -> &str {
            &self.label
        }
        fn shortcut(&self) -> &str {
            "Z"
        }
        fn icon_label(&self) -> &str {
            "Z"
        }
    }

    #[test]
    fn registry_new_empty() {
        let reg = ToolRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert!(reg.active().is_none());
    }

    #[test]
    fn registry_register_and_lookup() {
        let mut reg = ToolRegistry::new();
        reg.register(Box::new(StubTool {
            label: "Wand".into(),
        }));
        assert_eq!(reg.len(), 1);
        assert!(reg.tool_by_name("Wand").is_some());
        assert!(reg.tool_by_name("Missing").is_none());
    }

    #[test]
    fn registry_active_tool() {
        let mut reg = ToolRegistry::new();
        reg.register(Box::new(StubTool {
            label: "Alpha".into(),
        }));
        reg.register(Box::new(StubTool {
            label: "Beta".into(),
        }));
        assert_eq!(reg.active().unwrap().name(), "Alpha");
        reg.set_active_by_index(1);
        assert_eq!(reg.active().unwrap().name(), "Beta");
    }

    #[test]
    fn registry_set_active_by_name() {
        let mut reg = ToolRegistry::new();
        reg.register(Box::new(StubTool {
            label: "Alpha".into(),
        }));
        reg.register(Box::new(StubTool {
            label: "Beta".into(),
        }));
        assert!(reg.set_active_by_name("Beta"));
        assert_eq!(reg.active().unwrap().name(), "Beta");
        assert!(!reg.set_active_by_name("Gamma"));
    }

    #[test]
    fn registry_list_names() {
        let mut reg = ToolRegistry::new();
        reg.register(Box::new(StubTool { label: "A".into() }));
        reg.register(Box::new(StubTool { label: "B".into() }));
        assert_eq!(reg.list_names(), vec!["A", "B"]);
    }

    #[test]
    fn registry_default_trait() {
        let reg = ToolRegistry::default();
        assert!(reg.is_empty());
    }
}
