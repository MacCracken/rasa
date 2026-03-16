# Tazama Integration Guide

This guide explains how to use Rasa and Tazama together to extract video
frames, edit them as images, and send them back to the video timeline.

## Overview

Rasa and Tazama communicate through **file-based MCP tool exchange**. The
workflow uses PNG files as the interchange format and MCP tool calls to
orchestrate the process.

```
Tazama                          Rasa
  |                               |
  |-- tazama_extract_frame ------>|
  |   (writes frame.png)         |
  |                               |-- rasa_import_video_frame
  |                               |   (opens frame.png)
  |                               |
  |                               |   ... edit layers, filters ...
  |                               |
  |                               |-- rasa_export_for_video
  |<-- (reads edited.png) --------|   (writes edited.png)
  |                               |
  |-- tazama_add_clip             |
  |   (inserts as Image clip)     |
```

## Step-by-Step Workflow

### 1. Extract a Frame from Tazama

Use the `tazama_extract_frame` tool to decode a single frame from a video clip:

```json
{
  "name": "tazama_extract_frame",
  "arguments": {
    "clip_id": "<clip-uuid>",
    "frame_number": 120,
    "output_path": "/tmp/frame_120.png"
  }
}
```

Returns the file path, width, height, and frame number.

### 2. Import the Frame into Rasa

Use `rasa_import_video_frame` to open the PNG with source tracking:

```json
{
  "name": "rasa_import_video_frame",
  "arguments": {
    "path": "/tmp/frame_120.png",
    "source_clip_id": "<clip-uuid>",
    "frame_number": 120
  }
}
```

Returns a document ID, dimensions, and source metadata. The source metadata
enables the round-trip back to Tazama.

### 3. Edit in Rasa

Use standard Rasa tools (`rasa_edit_layer`, `rasa_apply_filter`, etc.) to
modify the frame. All layer operations, filters, and adjustments work as
usual.

### 4. Export for Video

Use `rasa_export_for_video` to composite and export the edited frame:

```json
{
  "name": "rasa_export_for_video",
  "arguments": {
    "document_id": "<document-uuid>",
    "output_path": "/tmp/frame_120_edited.png",
    "source_clip_id": "<clip-uuid>",
    "frame_number": 120
  }
}
```

Returns the output path, dimensions, and the source metadata for Tazama.

### 5. Insert Back into Tazama

Use `tazama_add_clip` to add the edited frame as an Image clip:

```json
{
  "name": "tazama_add_clip",
  "arguments": {
    "track": "Video 1",
    "source": "/tmp/frame_120_edited.png",
    "start_frame": 120,
    "duration_frames": 1
  }
}
```

## MCP Tool Reference

| Tool | Application | Purpose |
|------|-------------|---------|
| `tazama_extract_frame` | Tazama | Extract a single frame as PNG |
| `rasa_import_video_frame` | Rasa | Import a frame PNG with source metadata |
| `rasa_export_for_video` | Rasa | Export document as PNG for video insertion |
| `tazama_add_clip` | Tazama | Insert the edited image back into the timeline |

## Current Limitations

- **Single frame only:** Each round-trip handles one frame. Batch frame editing
  is not yet supported.
- **File system required:** Both applications must have access to the same file
  system (or the orchestrating agent must transfer files).
- **No embedded metadata:** Source tracking (clip ID, frame number) is carried
  in MCP parameters, not in the PNG file. If the metadata is lost between
  steps, the user must manually specify the insertion point.
- **GStreamer dependency:** Frame extraction requires GStreamer to be installed
  and initialized. If GStreamer is unavailable, `tazama_extract_frame` will
  return an error.
- **No automatic replacement:** The workflow adds a new Image clip rather than
  replacing the original frame in the source video clip.
