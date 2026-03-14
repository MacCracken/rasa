use egui;
use rasa_core::Document;

/// Canvas viewport state — pan, zoom, grid.
#[derive(Debug, Clone)]
pub struct CanvasState {
    pub zoom: f32,
    pub pan: egui::Vec2,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
        }
    }
}

/// Render the canvas viewport with the document.
pub fn canvas_viewport(
    ui: &mut egui::Ui,
    doc: &Document,
    state: &mut CanvasState,
    show_grid: bool,
    show_rulers: bool,
) {
    let available = ui.available_size();

    // Rulers
    if show_rulers {
        ui.horizontal(|ui| {
            ui.label(format!(
                "  {}x{} @ {:.0}%",
                doc.size.width,
                doc.size.height,
                state.zoom * 100.0
            ));
        });
    }

    let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());
    let rect = response.rect;

    // Handle pan (middle-click drag or space+drag)
    if response.dragged_by(egui::PointerButton::Middle) {
        state.pan += response.drag_delta();
    }

    // Handle zoom (scroll wheel)
    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll.abs() > 0.1 {
        let factor = if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 };
        state.zoom = (state.zoom * factor).clamp(0.1, 32.0);
    }

    // Canvas background (checkerboard for transparency)
    let doc_w = doc.size.width as f32 * state.zoom;
    let doc_h = doc.size.height as f32 * state.zoom;
    let canvas_origin = egui::Pos2::new(
        rect.center().x - doc_w / 2.0 + state.pan.x,
        rect.center().y - doc_h / 2.0 + state.pan.y,
    );
    let canvas_rect = egui::Rect::from_min_size(canvas_origin, egui::Vec2::new(doc_w, doc_h));

    // Dark workspace background
    painter.rect_filled(rect, 0.0, egui::Color32::from_gray(40));

    // Checkerboard pattern for canvas area
    draw_checkerboard(&painter, canvas_rect, state.zoom);

    // Document border
    painter.rect_stroke(
        canvas_rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
        egui::StrokeKind::Outside,
    );

    // Pixel grid (only at high zoom)
    if show_grid && state.zoom >= 4.0 {
        draw_pixel_grid(
            &painter,
            canvas_rect,
            doc.size.width,
            doc.size.height,
            state.zoom,
        );
    }
}

fn draw_checkerboard(painter: &egui::Painter, rect: egui::Rect, zoom: f32) {
    let check_size = (8.0 * zoom).max(8.0);
    let light = egui::Color32::from_gray(200);
    let dark = egui::Color32::from_gray(160);

    // Fill with light first
    painter.rect_filled(rect, 0.0, light);

    // Draw dark checks
    let cols = (rect.width() / check_size).ceil() as i32;
    let rows = (rect.height() / check_size).ceil() as i32;
    for row in 0..rows {
        for col in 0..cols {
            if (row + col) % 2 == 1 {
                let x = rect.min.x + col as f32 * check_size;
                let y = rect.min.y + row as f32 * check_size;
                let check =
                    egui::Rect::from_min_size(egui::Pos2::new(x, y), egui::Vec2::splat(check_size))
                        .intersect(rect);
                painter.rect_filled(check, 0.0, dark);
            }
        }
    }
}

fn draw_pixel_grid(painter: &egui::Painter, rect: egui::Rect, width: u32, height: u32, zoom: f32) {
    let stroke = egui::Stroke::new(
        0.5,
        egui::Color32::from_rgba_premultiplied(100, 100, 100, 60),
    );

    // Vertical lines
    for x in 0..=width {
        let px = rect.min.x + x as f32 * zoom;
        if px >= rect.min.x && px <= rect.max.x {
            painter.line_segment(
                [
                    egui::Pos2::new(px, rect.min.y),
                    egui::Pos2::new(px, rect.max.y),
                ],
                stroke,
            );
        }
    }

    // Horizontal lines
    for y in 0..=height {
        let py = rect.min.y + y as f32 * zoom;
        if py >= rect.min.y && py <= rect.max.y {
            painter.line_segment(
                [
                    egui::Pos2::new(rect.min.x, py),
                    egui::Pos2::new(rect.max.x, py),
                ],
                stroke,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_state_default() {
        let state = CanvasState::default();
        assert_eq!(state.zoom, 1.0);
        assert_eq!(state.pan, egui::Vec2::ZERO);
    }

    #[test]
    fn zoom_clamp_range() {
        let mut state = CanvasState::default();
        state.zoom = 100.0;
        state.zoom = state.zoom.clamp(0.1, 32.0);
        assert_eq!(state.zoom, 32.0);
        state.zoom = 0.001;
        state.zoom = state.zoom.clamp(0.1, 32.0);
        assert_eq!(state.zoom, 0.1);
    }
}
