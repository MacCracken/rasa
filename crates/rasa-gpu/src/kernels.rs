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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_sources_not_empty() {
        assert!(!COMPOSITE_NORMAL.is_empty());
        assert!(!INVERT.is_empty());
        assert!(!GRAYSCALE.is_empty());
        assert!(!BRIGHTNESS_CONTRAST.is_empty());
        assert!(!BLUR_HORIZONTAL.is_empty());
    }

    #[test]
    fn shaders_contain_compute_annotation() {
        assert!(COMPOSITE_NORMAL.contains("@compute"));
        assert!(INVERT.contains("@compute"));
        assert!(GRAYSCALE.contains("@compute"));
        assert!(BRIGHTNESS_CONTRAST.contains("@compute"));
        assert!(BLUR_HORIZONTAL.contains("@compute"));
    }
}
