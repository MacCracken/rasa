use rasa_ai::registry::ProviderRegistry;
use rasa_engine::filter::FilterRegistry;

use crate::tool::ToolRegistry;

/// Context passed to plugins during registration, providing mutable access
/// to all registries so plugins can add filters, tools, and AI providers.
pub struct PluginContext<'a> {
    pub filters: &'a mut FilterRegistry,
    pub tools: &'a mut ToolRegistry,
    pub providers: &'a mut ProviderRegistry,
}

/// A plugin that can register filters, tools, and AI providers.
pub trait Plugin: Send + Sync {
    /// Plugin name for display and logging.
    fn name(&self) -> &str;

    /// Plugin version string.
    fn version(&self) -> &str;

    /// Register all capabilities into the provided context.
    fn register(&self, ctx: &mut PluginContext<'_>);
}

/// Manages loaded plugins and coordinates their registration.
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    /// Create an empty plugin manager.
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Add a plugin.
    pub fn add(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    /// Initialize all plugins, calling their `register` methods.
    pub fn init_all(&self, ctx: &mut PluginContext<'_>) {
        for plugin in &self.plugins {
            plugin.register(ctx);
        }
    }

    /// List the names of all loaded plugins.
    pub fn list(&self) -> Vec<&str> {
        self.plugins.iter().map(|p| p.name()).collect()
    }

    /// Number of loaded plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Whether no plugins are loaded.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rasa_core::pixel::PixelBuffer;
    use rasa_engine::filter::Filter;

    struct TestFilter;
    impl Filter for TestFilter {
        fn name(&self) -> &str {
            "TestGlow"
        }
        fn apply(&self, _buf: &mut PixelBuffer) {}
    }

    struct TestPlugin;
    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            "test-plugin"
        }
        fn version(&self) -> &str {
            "0.1.0"
        }
        fn register(&self, ctx: &mut PluginContext<'_>) {
            ctx.filters.register(Box::new(TestFilter));
        }
    }

    #[test]
    fn plugin_manager_empty() {
        let mgr = PluginManager::new();
        assert!(mgr.is_empty());
        assert_eq!(mgr.len(), 0);
    }

    #[test]
    fn plugin_manager_add_and_list() {
        let mut mgr = PluginManager::new();
        mgr.add(Box::new(TestPlugin));
        assert_eq!(mgr.len(), 1);
        assert_eq!(mgr.list(), vec!["test-plugin"]);
    }

    #[test]
    fn plugin_manager_init_all_registers() {
        let mut mgr = PluginManager::new();
        mgr.add(Box::new(TestPlugin));

        let mut filters = FilterRegistry::new();
        let mut tools = ToolRegistry::new();
        let mut providers = ProviderRegistry::new();

        let mut ctx = PluginContext {
            filters: &mut filters,
            tools: &mut tools,
            providers: &mut providers,
        };
        mgr.init_all(&mut ctx);

        assert_eq!(filters.len(), 1);
        assert!(filters.filter_by_name("TestGlow").is_some());
    }

    #[test]
    fn plugin_manager_default_trait() {
        let mgr = PluginManager::default();
        assert!(mgr.is_empty());
    }
}
