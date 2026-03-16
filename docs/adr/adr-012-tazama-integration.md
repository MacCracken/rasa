# ADR-012: Tazama Integration via File-Based MCP Tool Exchange

- **Status:** Accepted
- **Date:** 2026-03-16

## Context

Rasa (image editor) and Tazama (video editor) are complementary tools in the
Anomaly creative suite. Users frequently need to extract a single frame from a
video timeline, perform pixel-level edits (retouching, compositing, color
grading), and insert the result back into the video project.

We need a mechanism for the two applications to exchange frame data and
metadata so an AI agent (or a human using MCP tooling) can orchestrate the
round-trip workflow.

## Decision

We adopt **file-based exchange via MCP tools**:

1. **Tazama** exposes a `tazama_extract_frame` tool that decodes a single video
   frame and writes it to disk as a PNG file.
2. **Rasa** exposes a `rasa_import_video_frame` tool that opens the PNG and
   records source metadata (clip ID, frame number) so the origin is not lost.
3. **Rasa** exposes a `rasa_export_for_video` tool that composites the document
   and writes a PNG suitable for re-insertion into the video timeline, echoing
   back the source metadata.
4. On the Tazama side, the existing `tazama_add_clip` tool can insert the
   exported PNG as a `ClipKind::Image` clip at the original frame position.

The data flow is:

```
Tazama (extract_frame) --> PNG on disk --> Rasa (import_video_frame)
         edit in Rasa ...
Rasa (export_for_video) --> PNG on disk --> Tazama (add_clip as Image)
```

## Rationale

- **Simplicity:** PNG files are a universal interchange format. No custom IPC,
  shared memory, or network protocol is required.
- **Process isolation:** Rasa and Tazama run as separate processes (or even on
  separate machines). File-based exchange works across any deployment topology.
- **Debuggability:** Intermediate PNGs can be inspected with any image viewer.
- **MCP compatibility:** MCP tools already communicate via JSON-RPC; file paths
  are natural parameters. An AI agent can orchestrate the full workflow with
  sequential tool calls.
- **Lossless:** PNG is lossless, preserving full pixel fidelity for the
  round-trip.

## Consequences

- Disk I/O is required for every frame exchange. For single-frame workflows
  this is negligible; batch frame editing would benefit from a streaming
  approach in the future.
- The two applications must have access to a shared filesystem (or the agent
  must transfer the file).
- Source metadata (clip ID, frame number) is carried in the MCP tool
  parameters, not embedded in the PNG file itself. If the metadata is lost,
  the user must manually specify where to re-insert the frame.
