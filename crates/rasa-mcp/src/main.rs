// Rasa MCP Server — stdio-based MCP 2.0 protocol
//
// Exposes 5 tools for Claude integration:
//   1. rasa_open_image   — Open or create an image document
//   2. rasa_edit_layer   — Add, modify, or transform layers
//   3. rasa_apply_filter — Apply filters, adjustments, or AI effects
//   4. rasa_get_document — Get document state (layers, dimensions, history)
//   5. rasa_export       — Export to image file (PNG, JPEG, WebP, TIFF)

fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::level_filters::LevelFilter::INFO.into()),
        )
        .init();

    tracing::info!("rasa-mcp server starting (stdio transport)");
    rasa_mcp::server::run_stdio();
}
