use thiserror::Error;

#[derive(Debug, Error)]
pub enum GpuError {
    #[error("no suitable GPU adapter found")]
    NoAdapter,
    #[error("failed to request GPU device: {0}")]
    DeviceRequest(String),
    #[error("GPU operation failed: {0}")]
    Operation(String),
}

/// Manages the wgpu device and queue for GPU compute operations.
pub struct GpuDevice {
    adapter_name: String,
    backend: String,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl GpuDevice {
    /// Attempt to initialize a GPU device.
    pub fn new() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL,
            ..Default::default()
        });

        let adapter = pollster_block(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok_or(GpuError::NoAdapter)?;

        let info = adapter.get_info();
        let adapter_name = info.name.clone();
        let backend = format!("{:?}", info.backend);

        let (device, queue) = pollster_block(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("rasa-gpu"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
            None,
        ))
        .map_err(|e| GpuError::DeviceRequest(e.to_string()))?;

        Ok(Self {
            adapter_name,
            backend,
            device,
            queue,
        })
    }

    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    pub fn backend_name(&self) -> &str {
        &self.backend
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Create a GPU buffer from pixel data (RGBA f32).
    pub fn create_buffer_from_pixels(
        &self,
        data: &[f32],
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        use wgpu::util::DeviceExt;
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("pixel_buffer"),
                contents: bytemuck_cast_slice(data),
                usage,
            })
    }

    /// Read back GPU buffer contents as f32 values.
    pub fn read_buffer(&self, buffer: &wgpu::Buffer, size: u64) -> Vec<f32> {
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, size);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        self.device.poll(wgpu::Maintain::Wait);
        receiver.recv().unwrap().unwrap();

        let data = slice.get_mapped_range();
        let result: Vec<f32> = bytemuck_cast_from_slice(&data).to_vec();
        drop(data);
        staging.unmap();
        result
    }
}

/// Capabilities detected on the GPU.
#[derive(Debug, Clone)]
pub struct GpuCapabilities {
    pub adapter_name: String,
    pub backend: String,
    pub max_buffer_size: u64,
    pub max_compute_workgroup_size: [u32; 3],
}

impl GpuCapabilities {
    pub fn from_device(device: &GpuDevice, adapter: &wgpu::Adapter) -> Self {
        let info = adapter.get_info();
        let limits = device.device().limits();
        Self {
            adapter_name: info.name.clone(),
            backend: format!("{:?}", info.backend),
            max_buffer_size: limits.max_buffer_size,
            max_compute_workgroup_size: [
                limits.max_compute_workgroup_size_x,
                limits.max_compute_workgroup_size_y,
                limits.max_compute_workgroup_size_z,
            ],
        }
    }
}

// ── Helpers ──

/// Simple blocking future executor (avoids pulling in full async runtime for GPU init).
fn pollster_block<F: std::future::Future>(f: F) -> F::Output {
    // Use a simple spin-based block_on
    use std::pin::pin;
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};

    struct NoopWaker;
    impl Wake for NoopWaker {
        fn wake(self: Arc<Self>) {}
    }

    let waker = Waker::from(Arc::new(NoopWaker));
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(f);

    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(result) => return result,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// Safe cast of &[f32] to &[u8] for buffer upload.
fn bytemuck_cast_slice(data: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) }
}

/// Safe cast of &[u8] to &[f32] for buffer readback.
fn bytemuck_cast_from_slice(data: &[u8]) -> &[f32] {
    assert!(data.len().is_multiple_of(4));
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const f32, data.len() / 4) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytemuck_roundtrip() {
        let floats: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
        let bytes = bytemuck_cast_slice(&floats);
        assert_eq!(bytes.len(), 16);
        let back = bytemuck_cast_from_slice(bytes);
        assert_eq!(back, &[1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn gpu_device_may_fail() {
        // In CI/headless environments, GPU init will fail — that's expected.
        // This test just ensures it doesn't panic.
        let _result = GpuDevice::new();
    }
}
