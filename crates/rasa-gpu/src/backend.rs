use rasa_core::color::BlendMode;
use rasa_core::pixel::PixelBuffer;

/// Abstraction over CPU and GPU rendering backends.
pub trait RenderBackend: Send + Sync {
    /// Name of the backend (for logging/diagnostics).
    fn name(&self) -> &str;

    /// Whether this backend is GPU-accelerated.
    fn is_gpu(&self) -> bool;

    /// Composite src onto dst with the given blend mode and opacity.
    fn composite(&self, dst: &mut PixelBuffer, src: &PixelBuffer, mode: BlendMode, opacity: f32);

    /// Apply Gaussian blur in-place.
    fn gaussian_blur(&self, buf: &mut PixelBuffer, radius: u32);

    /// Apply sharpen (unsharp mask) in-place.
    fn sharpen(&self, buf: &mut PixelBuffer, radius: u32, amount: f32);

    /// Apply brightness/contrast adjustment in-place.
    fn brightness_contrast(&self, buf: &mut PixelBuffer, brightness: f32, contrast: f32);

    /// Invert colors in-place.
    fn invert(&self, buf: &mut PixelBuffer);

    /// Convert to grayscale in-place.
    fn grayscale(&self, buf: &mut PixelBuffer);
}

/// CPU fallback backend — uses rasa-engine's CPU implementations.
pub struct CpuBackend;

impl RenderBackend for CpuBackend {
    fn name(&self) -> &str {
        "CPU"
    }

    fn is_gpu(&self) -> bool {
        false
    }

    fn composite(&self, dst: &mut PixelBuffer, src: &PixelBuffer, mode: BlendMode, opacity: f32) {
        let w = dst.width.min(src.width);
        let h = dst.height.min(src.height);
        for y in 0..h {
            for x in 0..w {
                let base = dst.get(x, y).unwrap();
                let top = src.get(x, y).unwrap();
                let result = rasa_core::blend::blend(base, top, mode, opacity);
                dst.set(x, y, result);
            }
        }
    }

    fn gaussian_blur(&self, buf: &mut PixelBuffer, radius: u32) {
        cpu_gaussian_blur(buf, radius);
    }

    fn sharpen(&self, buf: &mut PixelBuffer, radius: u32, amount: f32) {
        cpu_sharpen(buf, radius, amount);
    }

    fn brightness_contrast(&self, buf: &mut PixelBuffer, brightness: f32, contrast: f32) {
        let factor = (1.0 + contrast) / (1.0 - contrast.min(0.9999));
        for px in buf.pixels_mut() {
            let a = px.a;
            px.r = ((px.r + brightness) * factor + 0.5 * (1.0 - factor)).clamp(0.0, 1.0);
            px.g = ((px.g + brightness) * factor + 0.5 * (1.0 - factor)).clamp(0.0, 1.0);
            px.b = ((px.b + brightness) * factor + 0.5 * (1.0 - factor)).clamp(0.0, 1.0);
            px.a = a;
        }
    }

    fn invert(&self, buf: &mut PixelBuffer) {
        for px in buf.pixels_mut() {
            px.r = 1.0 - px.r;
            px.g = 1.0 - px.g;
            px.b = 1.0 - px.b;
        }
    }

    fn grayscale(&self, buf: &mut PixelBuffer) {
        for px in buf.pixels_mut() {
            let lum = 0.2126 * px.r + 0.7152 * px.g + 0.0722 * px.b;
            px.r = lum;
            px.g = lum;
            px.b = lum;
        }
    }
}

/// Select the best available backend. Returns GPU if available, otherwise CPU.
pub fn select_backend(force_cpu: bool) -> Box<dyn RenderBackend> {
    if force_cpu {
        return Box::new(CpuBackend);
    }

    // Try GPU backend
    match super::device::GpuDevice::new() {
        Ok(device) => {
            tracing::info!(
                "GPU backend: {} ({})",
                device.adapter_name(),
                device.backend_name()
            );
            Box::new(GpuBackend { device })
        }
        Err(e) => {
            tracing::warn!("GPU unavailable, falling back to CPU: {e}");
            Box::new(CpuBackend)
        }
    }
}

/// GPU-accelerated backend.
///
/// Uses wgpu compute shaders for supported operations, with CPU fallback
/// for operations where GPU dispatch overhead exceeds the benefit.
pub struct GpuBackend {
    device: super::device::GpuDevice,
}

impl RenderBackend for GpuBackend {
    fn name(&self) -> &str {
        "GPU"
    }

    fn is_gpu(&self) -> bool {
        true
    }

    fn composite(&self, dst: &mut PixelBuffer, src: &PixelBuffer, mode: BlendMode, opacity: f32) {
        let shader = match mode {
            BlendMode::Normal => super::kernels::COMPOSITE_NORMAL,
            BlendMode::Multiply => super::kernels::COMPOSITE_MULTIPLY,
            BlendMode::Screen => super::kernels::COMPOSITE_SCREEN,
            // Other blend modes fall back to CPU
            _ => {
                CpuBackend.composite(dst, src, mode, opacity);
                return;
            }
        };
        let w = dst.width.min(src.width);
        let h = dst.height.min(src.height);
        super::pipeline::dispatch_composite_shader(&self.device, dst, src, shader, w, h, opacity);
    }

    fn gaussian_blur(&self, buf: &mut PixelBuffer, radius: u32) {
        // Blur requires multi-pass with intermediate buffers — CPU path for now
        // (GPU blur pipeline needs separate horizontal+vertical dispatch with kernel upload)
        cpu_gaussian_blur(buf, radius);
    }

    fn sharpen(&self, buf: &mut PixelBuffer, radius: u32, amount: f32) {
        cpu_sharpen(buf, radius, amount);
    }

    fn brightness_contrast(&self, buf: &mut PixelBuffer, brightness: f32, contrast: f32) {
        let factor = (1.0 + contrast) / (1.0 - contrast.min(0.9999));
        let pixel_count = (buf.width * buf.height) as u32;
        // Params: count, brightness, contrast_factor, padding
        let params: [u32; 4] = [
            pixel_count,
            brightness.to_bits(),
            factor.to_bits(),
            0,
        ];
        let params_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(params.as_ptr() as *const u8, 16)
        };
        super::pipeline::dispatch_pixel_shader(
            &self.device,
            buf,
            super::kernels::BRIGHTNESS_CONTRAST,
            params_bytes,
        );
    }

    fn invert(&self, buf: &mut PixelBuffer) {
        let pixel_count = (buf.width * buf.height) as u32;
        let params: [u32; 4] = [pixel_count, 0, 0, 0];
        let params_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(params.as_ptr() as *const u8, 16)
        };
        super::pipeline::dispatch_pixel_shader(
            &self.device,
            buf,
            super::kernels::INVERT,
            params_bytes,
        );
    }

    fn grayscale(&self, buf: &mut PixelBuffer) {
        let pixel_count = (buf.width * buf.height) as u32;
        let params: [u32; 4] = [pixel_count, 0, 0, 0];
        let params_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(params.as_ptr() as *const u8, 16)
        };
        super::pipeline::dispatch_pixel_shader(
            &self.device,
            buf,
            super::kernels::GRAYSCALE,
            params_bytes,
        );
    }
}

// ── CPU filter implementations (shared by both backends) ──

fn cpu_gaussian_blur(buf: &mut PixelBuffer, radius: u32) {
    if radius == 0 {
        return;
    }
    let kernel = build_gaussian_kernel(radius);
    let (w, h) = buf.dimensions();

    // Horizontal pass
    let mut temp = PixelBuffer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let mut r = 0.0_f32;
            let mut g = 0.0_f32;
            let mut b = 0.0_f32;
            let mut a = 0.0_f32;
            for (i, &weight) in kernel.iter().enumerate() {
                let sx = (x as i32 + i as i32 - radius as i32).clamp(0, w as i32 - 1) as u32;
                let px = buf.get(sx, y).unwrap();
                r += px.r * weight;
                g += px.g * weight;
                b += px.b * weight;
                a += px.a * weight;
            }
            temp.set(x, y, rasa_core::color::Color::new(r, g, b, a));
        }
    }

    // Vertical pass
    for y in 0..h {
        for x in 0..w {
            let mut r = 0.0_f32;
            let mut g = 0.0_f32;
            let mut b = 0.0_f32;
            let mut a = 0.0_f32;
            for (i, &weight) in kernel.iter().enumerate() {
                let sy = (y as i32 + i as i32 - radius as i32).clamp(0, h as i32 - 1) as u32;
                let px = temp.get(x, sy).unwrap();
                r += px.r * weight;
                g += px.g * weight;
                b += px.b * weight;
                a += px.a * weight;
            }
            buf.set(x, y, rasa_core::color::Color::new(r, g, b, a));
        }
    }
}

fn cpu_sharpen(buf: &mut PixelBuffer, radius: u32, amount: f32) {
    if radius == 0 || amount.abs() < 1e-6 {
        return;
    }
    let (w, h) = buf.dimensions();
    let mut blurred = PixelBuffer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            blurred.set(x, y, buf.get(x, y).unwrap());
        }
    }
    cpu_gaussian_blur(&mut blurred, radius);

    for y in 0..h {
        for x in 0..w {
            let orig = buf.get(x, y).unwrap();
            let blur = blurred.get(x, y).unwrap();
            buf.set(
                x,
                y,
                rasa_core::color::Color::new(
                    (orig.r + amount * (orig.r - blur.r)).clamp(0.0, 1.0),
                    (orig.g + amount * (orig.g - blur.g)).clamp(0.0, 1.0),
                    (orig.b + amount * (orig.b - blur.b)).clamp(0.0, 1.0),
                    orig.a,
                ),
            );
        }
    }
}

fn build_gaussian_kernel(radius: u32) -> Vec<f32> {
    let size = (radius * 2 + 1) as usize;
    let sigma = radius as f32 / 3.0;
    let mut kernel = Vec::with_capacity(size);
    let mut sum = 0.0_f32;
    for i in 0..size {
        let x = i as f32 - radius as f32;
        let v = (-x * x / (2.0 * sigma * sigma)).exp();
        kernel.push(v);
        sum += v;
    }
    for v in &mut kernel {
        *v /= sum;
    }
    kernel
}

#[cfg(test)]
mod tests {
    use super::*;
    use rasa_core::color::Color;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.02
    }

    #[test]
    fn cpu_backend_name() {
        let backend = CpuBackend;
        assert_eq!(backend.name(), "CPU");
        assert!(!backend.is_gpu());
    }

    #[test]
    fn cpu_composite_normal() {
        let backend = CpuBackend;
        let mut dst = PixelBuffer::filled(2, 2, Color::WHITE);
        let mut src = PixelBuffer::new(2, 2);
        src.set(0, 0, Color::new(1.0, 0.0, 0.0, 1.0));
        backend.composite(&mut dst, &src, BlendMode::Normal, 1.0);
        let px = dst.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 1.0));
        assert!(approx_eq(px.g, 0.0));
    }

    #[test]
    fn cpu_invert() {
        let backend = CpuBackend;
        let mut buf = PixelBuffer::filled(2, 2, Color::WHITE);
        backend.invert(&mut buf);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 0.0));
        assert!(approx_eq(px.g, 0.0));
        assert!(approx_eq(px.b, 0.0));
    }

    #[test]
    fn cpu_grayscale() {
        let backend = CpuBackend;
        let mut buf = PixelBuffer::filled(1, 1, Color::new(1.0, 0.0, 0.0, 1.0));
        backend.grayscale(&mut buf);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, px.g));
        assert!(approx_eq(px.g, px.b));
    }

    #[test]
    fn cpu_brightness_contrast() {
        let backend = CpuBackend;
        let mut buf = PixelBuffer::filled(1, 1, Color::new(0.5, 0.5, 0.5, 1.0));
        backend.brightness_contrast(&mut buf, 0.1, 0.0);
        let px = buf.get(0, 0).unwrap();
        assert!(px.r > 0.5);
    }

    #[test]
    fn cpu_blur() {
        let backend = CpuBackend;
        let mut buf = PixelBuffer::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                let c = if (x + y) % 2 == 0 {
                    Color::WHITE
                } else {
                    Color::BLACK
                };
                buf.set(x, y, c);
            }
        }
        backend.gaussian_blur(&mut buf, 2);
        let px = buf.get(4, 4).unwrap();
        assert!(px.r > 0.1 && px.r < 0.9);
    }

    #[test]
    fn force_cpu_returns_cpu_backend() {
        let backend = select_backend(true);
        assert_eq!(backend.name(), "CPU");
        assert!(!backend.is_gpu());
    }

    #[test]
    fn trait_object_works() {
        let backend: Box<dyn RenderBackend> = Box::new(CpuBackend);
        let mut buf = PixelBuffer::filled(2, 2, Color::WHITE);
        backend.invert(&mut buf);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 0.0));
    }
}
