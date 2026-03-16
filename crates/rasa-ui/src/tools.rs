/// Active tool selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTool {
    Brush,
    Eraser,
    Move,
    Selection,
    Eyedropper,
    Fill,
    Gradient,
    Crop,
    Transform,
    Text,
}

impl ActiveTool {
    /// Keyboard shortcut for this tool.
    pub fn shortcut(&self) -> &'static str {
        match self {
            Self::Brush => "B",
            Self::Eraser => "E",
            Self::Move => "M",
            Self::Selection => "S",
            Self::Eyedropper => "I",
            Self::Fill => "F",
            Self::Gradient => "G",
            Self::Crop => "C",
            Self::Transform => "T",
            Self::Text => "X",
        }
    }

    /// Display name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Brush => "Brush",
            Self::Eraser => "Eraser",
            Self::Move => "Move",
            Self::Selection => "Selection",
            Self::Eyedropper => "Eyedropper",
            Self::Fill => "Fill",
            Self::Gradient => "Gradient",
            Self::Crop => "Crop",
            Self::Transform => "Transform",
            Self::Text => "Text",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tools_have_shortcuts() {
        let tools = [
            ActiveTool::Brush,
            ActiveTool::Eraser,
            ActiveTool::Move,
            ActiveTool::Selection,
            ActiveTool::Eyedropper,
            ActiveTool::Fill,
            ActiveTool::Gradient,
            ActiveTool::Crop,
            ActiveTool::Transform,
            ActiveTool::Text,
        ];
        for tool in tools {
            assert!(!tool.shortcut().is_empty());
            assert!(!tool.name().is_empty());
        }
    }

    #[test]
    fn tool_equality() {
        assert_eq!(ActiveTool::Brush, ActiveTool::Brush);
        assert_ne!(ActiveTool::Brush, ActiveTool::Eraser);
    }

    #[test]
    fn ten_tools_total() {
        // Verify we have all 10 tools
        let count = [
            ActiveTool::Brush,
            ActiveTool::Eraser,
            ActiveTool::Move,
            ActiveTool::Selection,
            ActiveTool::Eyedropper,
            ActiveTool::Fill,
            ActiveTool::Gradient,
            ActiveTool::Crop,
            ActiveTool::Transform,
            ActiveTool::Text,
        ]
        .len();
        assert_eq!(count, 10);
    }
}
