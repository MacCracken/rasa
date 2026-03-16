use crate::provider::InferenceProvider;

/// Runtime registry of available [`InferenceProvider`] implementations.
///
/// The registry owns all registered providers and allows callers to
/// query them by name or fall back to a default.
pub struct ProviderRegistry {
    providers: Vec<Box<dyn InferenceProvider>>,
    default_idx: usize,
}

impl ProviderRegistry {
    /// Create an empty registry with no providers.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            default_idx: 0,
        }
    }

    /// Register a new provider. The first provider registered automatically
    /// becomes the default.
    pub fn register(&mut self, provider: Box<dyn InferenceProvider>) {
        self.providers.push(provider);
    }

    /// Set the default provider by index.
    ///
    /// # Panics
    /// Panics if `idx` is out of bounds.
    pub fn set_default(&mut self, idx: usize) {
        assert!(idx < self.providers.len(), "provider index out of bounds");
        self.default_idx = idx;
    }

    /// Return the default provider, or `None` if the registry is empty.
    pub fn default_provider(&self) -> Option<&dyn InferenceProvider> {
        self.providers.get(self.default_idx).map(|p| p.as_ref())
    }

    /// Look up a provider by its [`InferenceProvider::name`].
    pub fn provider_by_name(&self, name: &str) -> Option<&dyn InferenceProvider> {
        self.providers
            .iter()
            .find(|p| p.name() == name)
            .map(|p| p.as_ref())
    }

    /// List the names of all registered providers.
    pub fn list_providers(&self) -> Vec<&str> {
        self.providers.iter().map(|p| p.name()).collect()
    }

    /// Number of registered providers.
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Whether the registry contains no providers.
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rasa_core::error::RasaError;

    /// Minimal stub provider for unit tests.
    struct StubProvider {
        label: String,
    }

    impl StubProvider {
        fn new(label: &str) -> Self {
            Self {
                label: label.to_string(),
            }
        }
    }

    impl InferenceProvider for StubProvider {
        fn name(&self) -> &str {
            &self.label
        }

        fn is_available(
            &self,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>> {
            Box::pin(async { true })
        }

        fn text_to_image(
            &self,
            _prompt: &str,
            _negative_prompt: &str,
            _width: u32,
            _height: u32,
            _params: &crate::provider::GenerationParams,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Vec<u8>, RasaError>> + Send + '_>,
        > {
            Box::pin(async { Ok(vec![]) })
        }

        fn style_transfer(
            &self,
            _image_png: &[u8],
            _style: &str,
            _strength: f32,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Vec<u8>, RasaError>> + Send + '_>,
        > {
            Box::pin(async { Ok(vec![]) })
        }

        fn color_grade(
            &self,
            _image_png: &[u8],
            _preset: &str,
            _intensity: f32,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Vec<u8>, RasaError>> + Send + '_>,
        > {
            Box::pin(async { Ok(vec![]) })
        }
    }

    #[test]
    fn registry_new_empty() {
        let reg = ProviderRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert!(reg.default_provider().is_none());
    }

    #[test]
    fn registry_register_provider() {
        let mut reg = ProviderRegistry::new();
        reg.register(Box::new(StubProvider::new("Alpha")));
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());
    }

    #[test]
    fn registry_list_providers() {
        let mut reg = ProviderRegistry::new();
        reg.register(Box::new(StubProvider::new("Alpha")));
        reg.register(Box::new(StubProvider::new("Beta")));
        let names = reg.list_providers();
        assert_eq!(names, vec!["Alpha", "Beta"]);
    }

    #[test]
    fn registry_provider_by_name() {
        let mut reg = ProviderRegistry::new();
        reg.register(Box::new(StubProvider::new("Alpha")));
        reg.register(Box::new(StubProvider::new("Beta")));
        assert!(reg.provider_by_name("Alpha").is_some());
        assert_eq!(reg.provider_by_name("Alpha").unwrap().name(), "Alpha");
        assert!(reg.provider_by_name("Gamma").is_none());
    }

    #[test]
    fn registry_default_provider() {
        let mut reg = ProviderRegistry::new();
        reg.register(Box::new(StubProvider::new("First")));
        reg.register(Box::new(StubProvider::new("Second")));
        // First registered is the default
        assert_eq!(reg.default_provider().unwrap().name(), "First");
        // Change default
        reg.set_default(1);
        assert_eq!(reg.default_provider().unwrap().name(), "Second");
    }

    #[test]
    fn registry_default_trait() {
        let reg = ProviderRegistry::default();
        assert!(reg.is_empty());
    }
}
