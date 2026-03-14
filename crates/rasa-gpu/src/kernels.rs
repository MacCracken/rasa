/// WGSL compute shader sources for GPU operations.
///
/// These are the shader sources that will be compiled into compute pipelines
/// when the GPU backend is fully wired up.
///
/// Composite two RGBA buffers with Normal blend mode.
pub const COMPOSITE_NORMAL: &str = r#"
@group(0) @binding(0) var<storage, read> src: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> dst: array<vec4<f32>>;

struct Params {
    width: u32,
    height: u32,
    opacity: f32,
    _padding: u32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if x >= params.width || y >= params.height {
        return;
    }
    let idx = y * params.width + x;
    let base = dst[idx];
    let top = src[idx];
    let top_a = top.a * params.opacity;

    let out_a = top_a + base.a * (1.0 - top_a);
    if out_a <= 0.0 {
        dst[idx] = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        return;
    }

    let out_r = (top.r * top_a + base.r * base.a * (1.0 - top_a)) / out_a;
    let out_g = (top.g * top_a + base.g * base.a * (1.0 - top_a)) / out_a;
    let out_b = (top.b * top_a + base.b * base.a * (1.0 - top_a)) / out_a;
    dst[idx] = vec4<f32>(
        clamp(out_r, 0.0, 1.0),
        clamp(out_g, 0.0, 1.0),
        clamp(out_b, 0.0, 1.0),
        clamp(out_a, 0.0, 1.0),
    );
}
"#;

/// Invert colors of an RGBA buffer.
pub const INVERT: &str = r#"
@group(0) @binding(0) var<storage, read_write> pixels: array<vec4<f32>>;

struct Params {
    count: u32,
}
@group(0) @binding(1) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= params.count {
        return;
    }
    let px = pixels[idx];
    pixels[idx] = vec4<f32>(1.0 - px.r, 1.0 - px.g, 1.0 - px.b, px.a);
}
"#;

/// Convert to grayscale using luminance weights.
pub const GRAYSCALE: &str = r#"
@group(0) @binding(0) var<storage, read_write> pixels: array<vec4<f32>>;

struct Params {
    count: u32,
}
@group(0) @binding(1) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= params.count {
        return;
    }
    let px = pixels[idx];
    let lum = 0.2126 * px.r + 0.7152 * px.g + 0.0722 * px.b;
    pixels[idx] = vec4<f32>(lum, lum, lum, px.a);
}
"#;

/// Brightness/contrast adjustment.
pub const BRIGHTNESS_CONTRAST: &str = r#"
@group(0) @binding(0) var<storage, read_write> pixels: array<vec4<f32>>;

struct Params {
    count: u32,
    brightness: f32,
    contrast_factor: f32,
    _padding: u32,
}
@group(0) @binding(1) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= params.count {
        return;
    }
    let px = pixels[idx];
    let f = params.contrast_factor;
    let b = params.brightness;
    let r = clamp((px.r + b) * f + 0.5 * (1.0 - f), 0.0, 1.0);
    let g = clamp((px.g + b) * f + 0.5 * (1.0 - f), 0.0, 1.0);
    let blue = clamp((px.b + b) * f + 0.5 * (1.0 - f), 0.0, 1.0);
    pixels[idx] = vec4<f32>(r, g, blue, px.a);
}
"#;

/// Horizontal Gaussian blur pass.
pub const BLUR_HORIZONTAL: &str = r#"
@group(0) @binding(0) var<storage, read> input: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> kernel: array<f32>;

struct Params {
    width: u32,
    height: u32,
    radius: u32,
    _padding: u32,
}
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if x >= params.width || y >= params.height {
        return;
    }

    var acc = vec4<f32>(0.0);
    for (var i = 0u; i <= params.radius * 2u; i = i + 1u) {
        let sx = clamp(i32(x) + i32(i) - i32(params.radius), 0, i32(params.width) - 1);
        let idx = u32(y) * params.width + u32(sx);
        acc = acc + input[idx] * kernel[i];
    }
    output[y * params.width + x] = acc;
}
"#;

/// Vertical Gaussian blur pass.
pub const BLUR_VERTICAL: &str = r#"
@group(0) @binding(0) var<storage, read> input: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> kernel: array<f32>;

struct Params {
    width: u32,
    height: u32,
    radius: u32,
    _padding: u32,
}
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if x >= params.width || y >= params.height {
        return;
    }

    var acc = vec4<f32>(0.0);
    for (var i = 0u; i <= params.radius * 2u; i = i + 1u) {
        let sy = clamp(i32(y) + i32(i) - i32(params.radius), 0, i32(params.height) - 1);
        let idx = u32(sy) * params.width + x;
        acc = acc + input[idx] * kernel[i];
    }
    output[y * params.width + x] = acc;
}
"#;

/// Composite with Multiply blend mode.
pub const COMPOSITE_MULTIPLY: &str = r#"
@group(0) @binding(0) var<storage, read> src: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> dst: array<vec4<f32>>;

struct Params {
    width: u32,
    height: u32,
    opacity: f32,
    _padding: u32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if x >= params.width || y >= params.height {
        return;
    }
    let idx = y * params.width + x;
    let base = dst[idx];
    let top = src[idx];
    let top_a = top.a * params.opacity;

    if top_a <= 0.0 { return; }

    let blended_r = base.r * top.r;
    let blended_g = base.g * top.g;
    let blended_b = base.b * top.b;

    let out_a = top_a + base.a * (1.0 - top_a);
    if out_a <= 0.0 {
        dst[idx] = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        return;
    }
    let out_r = (blended_r * top_a + base.r * base.a * (1.0 - top_a)) / out_a;
    let out_g = (blended_g * top_a + base.g * base.a * (1.0 - top_a)) / out_a;
    let out_b = (blended_b * top_a + base.b * base.a * (1.0 - top_a)) / out_a;
    dst[idx] = vec4<f32>(
        clamp(out_r, 0.0, 1.0),
        clamp(out_g, 0.0, 1.0),
        clamp(out_b, 0.0, 1.0),
        clamp(out_a, 0.0, 1.0),
    );
}
"#;

/// Composite with Screen blend mode.
pub const COMPOSITE_SCREEN: &str = r#"
@group(0) @binding(0) var<storage, read> src: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> dst: array<vec4<f32>>;

struct Params {
    width: u32,
    height: u32,
    opacity: f32,
    _padding: u32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if x >= params.width || y >= params.height {
        return;
    }
    let idx = y * params.width + x;
    let base = dst[idx];
    let top = src[idx];
    let top_a = top.a * params.opacity;

    if top_a <= 0.0 { return; }

    let blended_r = 1.0 - (1.0 - base.r) * (1.0 - top.r);
    let blended_g = 1.0 - (1.0 - base.g) * (1.0 - top.g);
    let blended_b = 1.0 - (1.0 - base.b) * (1.0 - top.b);

    let out_a = top_a + base.a * (1.0 - top_a);
    if out_a <= 0.0 {
        dst[idx] = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        return;
    }
    let out_r = (blended_r * top_a + base.r * base.a * (1.0 - top_a)) / out_a;
    let out_g = (blended_g * top_a + base.g * base.a * (1.0 - top_a)) / out_a;
    let out_b = (blended_b * top_a + base.b * base.a * (1.0 - top_a)) / out_a;
    dst[idx] = vec4<f32>(
        clamp(out_r, 0.0, 1.0),
        clamp(out_g, 0.0, 1.0),
        clamp(out_b, 0.0, 1.0),
        clamp(out_a, 0.0, 1.0),
    );
}
"#;

/// GPU brush dab — paint a circular dab onto a buffer.
pub const BRUSH_DAB: &str = r#"
@group(0) @binding(0) var<storage, read_write> pixels: array<vec4<f32>>;

struct Params {
    width: u32,
    height: u32,
    center_x: f32,
    center_y: f32,
    radius: f32,
    hardness: f32,
    opacity: f32,
    _padding: u32,
    color_r: f32,
    color_g: f32,
    color_b: f32,
    color_a: f32,
}
@group(0) @binding(1) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if x >= params.width || y >= params.height {
        return;
    }

    let px = f32(x) + 0.5;
    let py = f32(y) + 0.5;
    let dx = px - params.center_x;
    let dy = py - params.center_y;
    let dist = sqrt(dx * dx + dy * dy);

    if dist > params.radius {
        return;
    }

    let t = dist / params.radius;
    var alpha: f32;
    if t <= params.hardness {
        alpha = 1.0;
    } else {
        let fade = (t - params.hardness) / (1.0 - params.hardness);
        alpha = max(1.0 - fade, 0.0);
    }
    alpha = alpha * params.opacity * params.color_a;

    let idx = y * params.width + x;
    let base = pixels[idx];
    let top_a = alpha;
    let out_a = top_a + base.a * (1.0 - top_a);
    if out_a <= 0.0 {
        return;
    }
    let out_r = (params.color_r * top_a + base.r * base.a * (1.0 - top_a)) / out_a;
    let out_g = (params.color_g * top_a + base.g * base.a * (1.0 - top_a)) / out_a;
    let out_b = (params.color_b * top_a + base.b * base.a * (1.0 - top_a)) / out_a;
    pixels[idx] = vec4<f32>(
        clamp(out_r, 0.0, 1.0),
        clamp(out_g, 0.0, 1.0),
        clamp(out_b, 0.0, 1.0),
        clamp(out_a, 0.0, 1.0),
    );
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_SHADERS: &[(&str, &str)] = &[
        ("composite_normal", COMPOSITE_NORMAL),
        ("composite_multiply", COMPOSITE_MULTIPLY),
        ("composite_screen", COMPOSITE_SCREEN),
        ("invert", INVERT),
        ("grayscale", GRAYSCALE),
        ("brightness_contrast", BRIGHTNESS_CONTRAST),
        ("blur_horizontal", BLUR_HORIZONTAL),
        ("blur_vertical", BLUR_VERTICAL),
        ("brush_dab", BRUSH_DAB),
    ];

    #[test]
    fn shader_sources_not_empty() {
        for (name, src) in ALL_SHADERS {
            assert!(!src.is_empty(), "{name} shader is empty");
        }
    }

    #[test]
    fn shaders_contain_compute_annotation() {
        for (name, src) in ALL_SHADERS {
            assert!(src.contains("@compute"), "{name} missing @compute");
        }
    }

    #[test]
    fn shaders_contain_main_entry() {
        for (name, src) in ALL_SHADERS {
            assert!(src.contains("fn main("), "{name} missing fn main");
        }
    }
}
