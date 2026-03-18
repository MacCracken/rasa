use crate::tool::{Tool, ToolRegistry};

macro_rules! builtin_tool {
    ($struct_name:ident, $name:expr, $shortcut:expr, $icon:expr) => {
        pub struct $struct_name;
        impl Tool for $struct_name {
            fn name(&self) -> &str {
                $name
            }
            fn shortcut(&self) -> &str {
                $shortcut
            }
            fn icon_label(&self) -> &str {
                $icon
            }
        }
    };
}

builtin_tool!(BrushTool, "Brush", "B", "B");
builtin_tool!(EraserTool, "Eraser", "E", "E");
builtin_tool!(MoveTool, "Move", "M", "M");
builtin_tool!(SelectionTool, "Selection", "S", "S");
builtin_tool!(EyedropperTool, "Eyedropper", "I", "I");
builtin_tool!(FillTool, "Fill", "F", "F");
builtin_tool!(GradientTool, "Gradient", "G", "G");
builtin_tool!(CropTool, "Crop", "C", "C");
builtin_tool!(TransformTool, "Transform", "T", "T");
builtin_tool!(TextTool, "Text", "X", "X");

/// Register all 10 built-in tools in standard order.
pub fn register_builtins(registry: &mut ToolRegistry) {
    registry.register(Box::new(BrushTool));
    registry.register(Box::new(EraserTool));
    registry.register(Box::new(MoveTool));
    registry.register(Box::new(SelectionTool));
    registry.register(Box::new(EyedropperTool));
    registry.register(Box::new(FillTool));
    registry.register(Box::new(GradientTool));
    registry.register(Box::new(CropTool));
    registry.register(Box::new(TransformTool));
    registry.register(Box::new(TextTool));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_builtins_populates_registry() {
        let mut reg = ToolRegistry::new();
        register_builtins(&mut reg);
        assert_eq!(reg.len(), 10);
    }

    #[test]
    fn builtin_names_match_tools() {
        let mut reg = ToolRegistry::new();
        register_builtins(&mut reg);
        let expected = vec![
            "Brush",
            "Eraser",
            "Move",
            "Selection",
            "Eyedropper",
            "Fill",
            "Gradient",
            "Crop",
            "Transform",
            "Text",
        ];
        assert_eq!(reg.list_names(), expected);
    }

    #[test]
    fn builtin_shortcuts_non_empty() {
        let mut reg = ToolRegistry::new();
        register_builtins(&mut reg);
        for tool in reg.list_tools() {
            assert!(
                !tool.shortcut().is_empty(),
                "{} has empty shortcut",
                tool.name()
            );
            assert!(
                !tool.icon_label().is_empty(),
                "{} has empty icon",
                tool.name()
            );
        }
    }

    #[test]
    fn lookup_builtin_by_name() {
        let mut reg = ToolRegistry::new();
        register_builtins(&mut reg);
        let t = reg.tool_by_name("Brush").unwrap();
        assert_eq!(t.shortcut(), "B");
    }
}
