# Muharrir Integration Roadmap

Items that rasa currently implements custom but could be replaced by future
muharrir shared primitives. These are candidates for upstream development in
muharrir and subsequent adoption in rasa.

## Completed (v0.23.3)

- [x] **command** — `Command` trait implemented on rasa `Command` enum; `CommandHistory` and `CompoundCommand` re-exported for external consumers
- [x] **dirty** — `DirtyState` added to `Document` for unsaved-change tracking; status bar shows `(modified)`, export marks clean, quit warns on unsaved
- [x] **inspector** — `PropertySheet` used for structured layer property inspection in collapsible Inspector panel
- [x] **notification** — `Toasts` + `NotificationLog` replace ad-hoc status messages; toast overlay with severity-colored badges
- [x] **selection** — `PanelStates` used for panel visibility management (View menu toggles)
- [x] **recent** — `RecentFiles` for file history persistence; File > Open Recent submenu
- [x] **prefs** — `PrefsStore` + `config_dir` for preferences persistence (`~/.config/rasa/`)
- [x] **expr** — `eval_f64` wired into brush size text input for expression evaluation (e.g. "10+5")
- [x] **hw** — `HardwareProfile` + `QualityTier` integrated into `select_backend()` — probes hardware before GPU init, falls back to CPU on Low tier

## Available but not yet integrated

- [ ] **hierarchy** — `build_hierarchy` / `flatten` available for layer tree display; not yet used because the current flat `Vec<Layer>` iteration is sufficient. Will integrate when drag-and-drop reordering or nested group display is added.
- [ ] **history** (audit) — `muharrir::history::History` provides tamper-evident audit logging via `libro::AuditChain`. Not yet integrated alongside undo/redo. Blocked on reconciling the two history models (undo stack vs audit chain).
- [ ] **selection** (item) — `muharrir::selection::Selection<T>` available for multi-item selection (e.g. selecting multiple layers). Currently only single active_layer is tracked. Will integrate when multi-layer selection is added.

## Roadmap — pending muharrir upstream

### PixelBuffer / Image Primitives
- rasa-core `PixelBuffer` (RGBA f32 linear, get/set, dimensions)
- Could be a shared `muharrir::pixel` or `muharrir::image` module
- Would enable shared compositing primitives across AGNOS apps

### Color / Blend Mode Primitives
- rasa-core `Color` (linear RGBA f32), `BlendMode` (12 modes), `ColorSpace`
- rasa-core `blend()` function with all blend mode implementations
- Currently tightly coupled to `ranga` — could share via muharrir

### Filter System
- rasa-engine `Filter` trait + `FilterRegistry`
- Dynamic filter registration, lookup by name, built-in filters
- Candidate for `muharrir::filter` module

### Tool System
- rasa-ui `Tool` trait + `ToolRegistry`
- Dynamic tool registration with shortcuts and icons
- Candidate for `muharrir::tool` module

### Plugin System
- rasa-ui `Plugin` trait + `PluginManager` + `PluginContext`
- Plugin lifecycle, context injection for filters/tools/providers
- Candidate for `muharrir::plugin` module

### Transform / Geometry Primitives
- rasa-core `Transform` (2D affine), `Point`, `Size`, `Rect`
- Currently wraps `ranga::transform::Affine` + `kurbo`
- Could unify geometry types across AGNOS apps

### Selection (Spatial)
- rasa-core `Selection` enum (None, Rect, Ellipse, Freeform, Mask)
- Spatial selection with mask operations (union, subtract, intersect)
- Different from muharrir's `Selection<T>` (item selection) — could be `muharrir::spatial_selection`

### CommandHistory push-without-apply
- Current `CommandHistory::execute` always calls `apply` on the target
- rasa uses an apply-then-record pattern (apply inline, then push command)
- A `push_recorded(cmd)` method would enable rasa to fully adopt `CommandHistory`
- Alternatively, an `execute_pre_applied` variant

### Personality / Theming
- muharrir `personality` feature (via `bhava`) for app theming
- Not yet evaluated for rasa's egui-based UI
