use rasa_core::pixel::PixelBuffer;

/// A pluggable image filter that operates on a pixel buffer in-place.
///
/// Built-in filters (invert, grayscale, blur, sharpen) implement this trait,
/// and third-party plugins can register additional filters at runtime.
pub trait Filter: Send + Sync {
    /// Unique name for display and lookup.
    fn name(&self) -> &str;

    /// Apply the filter to `buf` in-place.
    fn apply(&self, buf: &mut PixelBuffer);
}

/// Runtime registry of available [`Filter`] implementations.
pub struct FilterRegistry {
    filters: Vec<Box<dyn Filter>>,
}

impl FilterRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    /// Register a new filter.
    pub fn register(&mut self, filter: Box<dyn Filter>) {
        self.filters.push(filter);
    }

    /// Look up a filter by name.
    pub fn filter_by_name(&self, name: &str) -> Option<&dyn Filter> {
        self.filters
            .iter()
            .find(|f| f.name() == name)
            .map(|f| f.as_ref())
    }

    /// List the names of all registered filters.
    pub fn list_filters(&self) -> Vec<&str> {
        self.filters.iter().map(|f| f.name()).collect()
    }

    /// Number of registered filters.
    pub fn len(&self) -> usize {
        self.filters.len()
    }

    /// Whether the registry contains no filters.
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }
}

impl Default for FilterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubFilter {
        label: String,
    }

    impl Filter for StubFilter {
        fn name(&self) -> &str {
            &self.label
        }
        fn apply(&self, _buf: &mut PixelBuffer) {}
    }

    #[test]
    fn registry_new_empty() {
        let reg = FilterRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn registry_register_and_lookup() {
        let mut reg = FilterRegistry::new();
        reg.register(Box::new(StubFilter {
            label: "Glow".into(),
        }));
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());
        assert!(reg.filter_by_name("Glow").is_some());
        assert!(reg.filter_by_name("Missing").is_none());
    }

    #[test]
    fn registry_list_filters() {
        let mut reg = FilterRegistry::new();
        reg.register(Box::new(StubFilter {
            label: "Alpha".into(),
        }));
        reg.register(Box::new(StubFilter {
            label: "Beta".into(),
        }));
        assert_eq!(reg.list_filters(), vec!["Alpha", "Beta"]);
    }

    #[test]
    fn registry_default_trait() {
        let reg = FilterRegistry::default();
        assert!(reg.is_empty());
    }
}
