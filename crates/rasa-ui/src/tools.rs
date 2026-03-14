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
    fn nine_tools_total() {
        // Verify we have all 9 tools
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
        ]
        .len();
        assert_eq!(count, 9);
    }
}
