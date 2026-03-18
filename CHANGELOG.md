# Changelog

All notable changes to Rasa will be documented in this file.

## [2026.3.18] — 2026-03-18

### Added

**Plugin System** (rasa-engine, rasa-ui, rasa-ai)
- `Filter` trait + `FilterRegistry` — pluggable filter interface with runtime registration
- 4 built-in filter wrappers: Invert, Grayscale, Gaussian Blur, Sharpen
- `Tool` trait + `ToolRegistry` — pluggable tool interface with active tool tracking
- 10 built-in tool impls matching existing `ActiveTool` enum
- `Plugin` trait + `PluginContext` + `PluginManager` — third-party registration system
- Provider-aware AI pipeline: `AiPipeline` now holds `ProviderRegistry`, routes `StyleTransfer`/`ColorGrade` through registered providers
- `AiPipeline::with_registry()` constructor for custom provider sets
- Dynamic Filter menu in UI — renders from `FilterRegistry` instead of hardcoded list
- `PluginError` variant added to `RasaError`
- `[plugins]` section in `rasa.example.toml`

**Gradient Vector Fills** (rasa-core, rasa-engine)
- `FillStyle::LinearGradient` — start/end points with start/end colors
- `FillStyle::RadialGradient` — center/radius with center/edge colors
- `sample_fill()` dispatcher for all three fill styles in vector renderer

**Color Space Rendering** (rasa-engine)
- Display P3: proper sRGB→P3 gamut matrix (Bradford-adapted) with sRGB transfer function
- CMYK preview: naive CMYK roundtrip simulation for on-screen soft-proofing

**File Dialogs** (rasa-ui)
- `rfd::FileDialog` integration replacing stub functions
- Open dialog: Images (9 formats), Rasa Project, All Files filters
- Save dialog: PNG, JPEG, WebP, TIFF, BMP, Rasa Project filters

**Text Rendering** (rasa-engine)
- `render_text_layer()` now discovers and uses system fonts (Linux/macOS/Windows)
- Falls back to transparent buffer only when no system font is found

**Configuration** (rasa-ai, rasa-ui)
- `RASA_SYNAPSE_URL` environment variable for AI server endpoint (default: `http://localhost:8090`)
- `RASA_AI_TIMEOUT` environment variable for HTTP timeout in seconds (default: 300)

**Build Infrastructure**
- `bump-version.sh` updated to sync VERSION, Cargo.toml, Cargo.lock, .agnos-agent.json, and roadmap.md
- Makefile `version-bump` target now delegates to the bump script
- P0 roadmap section: code refactor, review/audit, security, performance sweep items

### Improved

- **Vector rendering performance**: fill and stroke now iterate only the path bounding box instead of the full canvas
- **Selection::to_mask optimization**: Rect and Ellipse use direct bounding-box iteration instead of per-pixel `contains()` across entire image
- **AI workflow memory**: `ai_select()` no longer clones the input buffer (borrows instead)
- **Error handling**: replaced 6 `.unwrap()` calls on `.mime_str()` in AI client with `.expect()` including reason

### Test Summary

**600 tests passing** across 7 crates (up from 566). 89% coverage on testable crates.

---

## [2026.3.16] — 2026-03-16

### Added

**PSD Import/Export** (rasa-storage)
- Multi-layer PSD import with layer name, opacity, visibility, and blend mode mapping (12 direct + 16 grouped PSD blend modes)
- Positioned layer data placement within document-size buffers
- Flat (composited) PSD export with valid file header, 4-channel RGBA uncompressed image data
- Blend mode mapping between PSD and rasa (bidirectional)

**RAW Format Support** (rasa-storage)
- Camera RAW import: CR2, NEF, ARW, DNG, RAF, ORF, RW2 via `imagepipe`
- Full demosaic, white balance, and tone-mapping pipeline with 16-bit precision
- Import-only (RAW is a capture format, not an output format)

**ICC Profile Management** (rasa-core, rasa-storage)
- `IccProfile` struct with header parsing (RGB/CMYK/Gray/Lab color space detection)
- Built-in sRGB IEC61966-2.1 v2 profile via `IccProfile::srgb_v2()`
- `ProfileColorSpace` enum for profile type identification
- Document-level `color_space` and `icc_profile` fields
- ICC-aware export via `export_buffer_with_config()` — embeds profiles in PNG, JPEG, TIFF
- `ExportConfig` struct wrapping settings + optional ICC profile
- Profile transform between RGB profiles via `lcms2` (apply_profile_transform)
- Error variants: `InvalidIccProfile`, `ColorConversionFailed`

**CMYK Color Mode** (rasa-core, rasa-storage)
- `CmykColor` struct with c/m/y/k components (0.0-1.0)
- `Cmyk` variant added to `ColorSpace` enum
- Naive RGB↔CMYK conversion (`rgb_to_cmyk_naive`, `cmyk_to_rgb_naive`)
- ICC-based CMYK conversion via lcms2 (`buffer_to_cmyk_icc`, `cmyk_to_buffer_icc`)
- CMYK TIFF export (`ExportSettings::TiffCmyk`) with proper IFD structure, resolution tags, InkSet

**Batch Processing** (rasa-storage, rasa-mcp)
- `BatchJob` engine: import files, apply filter chains, export with format conversion
- 6 batch filters: Invert, Grayscale, BrightnessContrast, HueSaturation, GaussianBlur, Sharpen
- `rasa_batch_export` MCP tool (tool #6) for agent-driven batch operations
- Batch size capped at 1000 files, filter validation with error reporting

**Text Engine** (rasa-core, rasa-engine, rasa-ui)
- Extended `TextLayer` with `color`, `alignment` (Left/Center/Right), `line_height` fields
- `TextAlign` enum for horizontal text alignment
- Text rendering module using `ab_glyph` for glyph rasterization with kerning and multiline
- Compositor integration: explicit `LayerKind::Text` match arm with on-demand rendering
- Text UI tool (shortcut "X") — 10th tool in palette

**Vector Tools** (rasa-core, rasa-engine)
- `VectorData`, `VectorPath`, `PathSegment` (MoveTo/LineTo/QuadTo/CubicTo) types
- `FillStyle` (Solid), `StrokeStyle` (color, width, cap, join), `LineCap`, `LineJoin` enums
- Shape constructors: `VectorPath::rect()`, `ellipse()`, `line()`
- Vector rendering via `kurbo` — winding-number fill, distance-based stroke rasterization
- `LayerKind::Vector` upgraded from unit stub to `Vector(VectorData)` with compositor integration

**AI Features — Provider Abstraction** (rasa-ai)
- `InferenceProvider` trait for provider-agnostic AI operations (text-to-image, style transfer, color grading)
- `SynapseProvider` wrapping existing SynapseClient for local inference
- `ProviderRegistry` for runtime provider storage and selection
- Style transfer: `ModelKind::StyleTransfer`, client method, pipeline request, workflow operation
- AI color grading: `ModelKind::ColorGrading`, client method, pipeline request, workflow operation
- New client methods: `style_transfer()`, `color_grade()` on SynapseClient

**Performance** (rasa-engine)
- Rayon parallel compositing: row-level parallelism in `composite_layer`
- Parallel filters: `gaussian_blur`, `sharpen`, `invert`, `grayscale`, all adjustments parallelized via `par_chunks_mut`/`par_iter_mut`

**Tazama Integration** (rasa-mcp)
- `rasa_import_video_frame` MCP tool — import frame PNG with source clip/frame metadata for round-trip
- `rasa_export_for_video` MCP tool — export composited document as PNG with source metadata
- File-based exchange protocol: Tazama extracts frame → Rasa edits → Rasa exports → Tazama inserts as Image clip

**Documentation**
- ADR-008: Text Engine (ab_glyph, on-demand rendering)
- ADR-009: Parallel Compositing with Rayon
- ADR-010: Vector Tools (kurbo, rasterize-on-demand)
- ADR-011: AI Provider Abstraction (InferenceProvider trait)
- ADR-012: Tazama Integration (file-based MCP exchange)
- Guide: Text Layers
- Guide: Performance
- Guide: Vector Layers
- Guide: AI Features
- Guide: Tazama Integration

### Fixed

- PSD export channel order: corrected from A,R,G,B to R,G,B,A per PSD spec
- Integer overflow in PSD pixel index calculation (u32 → usize arithmetic)
- RAW import out-of-bounds panic on truncated decoded data
- RAW dimension cast overflow (usize → u32::try_from with error)
- `ExportSettings::for_format(Raw)` changed from panic to Result
- `to_image_format()` replaced `unreachable!()` with proper Result errors
- `buffer_to_image()` replaced `expect()` with safe fallback
- `merge_down()` replaced `.unwrap()` with `let Some(..) else { continue }`
- `rgba_bytes_to_buffer()` safe on short input (no OOB panic)
- CMYK TIFF: added XResolution, YResolution, ResolutionUnit tags for spec compliance
- MCP export now passes document ICC profile through to `export_buffer_with_config`
- PSD layer dimensions use `u32::from()` (safe widening from u16)
- Batch size capped at 1000 files (prevent resource exhaustion)
- Invalid batch filters now return errors instead of being silently dropped
- JPEG quality clamped to 1-100 before cast (prevents u8 truncation)
- Blur/sharpen radius clamped to 1-500 in MCP batch tool
- `RasaError::Other` replaced with specific error variants (`Io`, `CorruptFile`, `UnsupportedFormat`) at 8 sites

### Test Summary

**566 tests passing** across 7 crates (up from 410). 89% coverage on testable crates.

---

## [2026.3.13] — 2026-03-13

### Added

**rasa-core** — Document model and core types
- Document, Layer, Color (`#[repr(C)]`), Geometry, Transform, Selection, PixelBuffer types
- 12 blend modes with Porter-Duff alpha compositing
- Undo/redo command history (VecDeque-based, O(1) eviction)
- Error type hierarchy: 13 domain-specific variants (layer, selection, transform, storage, AI, history)
- Selection combine operations (add, subtract, intersect via mask arithmetic)
- Merge down, layer grouping/ungrouping with full undo/redo
- Dimension validation (1..65536 clamping), PixelBuffer allocation cap (256 MP)
- Last-layer removal guard (`CannotRemoveLastLayer`)
- 154 unit tests + 32 serde round-trip integration tests

**rasa-engine** — Rendering and editing tools
- CPU compositing pipeline with recursive group rendering and adjustment layers
- Document renderer with sRGB/linear/Display P3 color space conversion
- 8 filters: brightness/contrast, hue/saturation, curves, levels, gaussian blur, sharpen, invert, grayscale
- Tile-based rendering (256x256) with dirty-region cache
- Brush engine: round/square tips, hardness, pressure sensitivity, spacing
- Eraser, flood fill, selection fill, linear gradient, crop, affine transform (bilinear interpolation)
- Eyedropper / color picker
- Optimized hot paths: slice-based pixel access (no per-pixel bounds checks)
- 71 tests

**rasa-gpu** — GPU acceleration
- wgpu device initialization (Vulkan/Metal) with graceful CPU fallback
- 9 WGSL compute shaders: composite (Normal/Multiply/Screen), invert, grayscale, brightness/contrast, blur H/V, brush dab
- Compute pipeline: shader compilation, bind groups, dispatch, GPU readback
- `RenderBackend` trait abstracting CPU/GPU with `select_backend()`
- Performance benchmark framework (CPU baseline + GPU comparison, MP/s metrics)
- 20 tests

**rasa-storage** — File I/O
- PNG, JPEG, WebP, TIFF, BMP, GIF import/export with sRGB/linear conversion
- JPEG quality wired to encoder (1-100)
- Native `.rasa` project format: RASA magic, JSON header (size-validated), binary pixel data
- Buffered I/O (BufReader/BufWriter) for large documents
- Recent files catalog (SQLite via rusqlite)
- Format detection, alpha support queries, export settings
- 41 tests

**rasa-ai** — AI inference
- hoosh/Synapse HTTP API client (inpaint, upscale, segment, generate, remove-bg)
- Model management: ModelId, ModelInfo, ModelKind, preset models
- Pre/post-processing pipeline (PixelBuffer to/from PNG)
- Document integration: apply AI results as layers, within selections, with feathered blending
- Progress tracking and cancellation
- 36 tests

**rasa-mcp** — MCP server and AGNOS integration
- MCP 2.0 server: stdio transport, JSON-RPC 2.0 (initialize, tools/list, tools/call)
- 5 tools: rasa_open_image, rasa_edit_layer, rasa_apply_filter, rasa_get_document, rasa_export
- 5 agnoshi voice intents: rasa.open, rasa.filter, rasa.layer, rasa.export, rasa.ai
- `.agnos-agent.json` bundle
- Session state with Mutex poison recovery
- Input validation: dimension caps, file existence checks, parent directory validation
- JSON-RPC spec compliance: `skip_serializing_if` for response fields, notification handling
- 47 tests

**rasa-ui** — Desktop GUI
- egui/eframe application (Wayland-compatible via winit)
- Menu bar (File/Edit/View/Layer/Filter), status bar
- Canvas viewport: pan, zoom, pixel grid, checkerboard transparency
- 9-tool palette: Brush, Eraser, Move, Selection, Eyedropper, Fill, Gradient, Crop, Transform
- Layer panel: visibility, opacity slider, blend mode, click-to-select
- Properties panel: tool-specific settings, color picker with hex display
- History panel with undo/redo
- Keyboard shortcuts: B/E/M/S/I/F/G/C/T, Ctrl+Z/Shift+Z, +/-
- 9 tests

**Infrastructure**
- 7-crate workspace with Cargo.toml workspace dependencies
- CI pipeline: build, test, lint (clippy), audit
- Makefile with 20 targets
- Date-based versioning (YYYY.M.DD)

### Test Summary

**410 tests passing** across 7 crates. 89% coverage on testable crates (core, engine, storage, mcp).
