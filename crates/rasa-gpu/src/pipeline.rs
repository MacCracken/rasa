use rasa_core::color::Color;
use rasa_core::pixel::PixelBuffer;
use wgpu;

use crate::device::GpuDevice;

/// Execute a simple per-pixel compute shader (invert, grayscale, brightness/contrast).
/// The shader must operate on a single storage buffer with a uniform params buffer.
pub fn dispatch_pixel_shader(
    device: &GpuDevice,
    buf: &mut PixelBuffer,
    shader_source: &str,
    params_data: &[u8],
) {
    let (w, h) = buf.dimensions();
    let pixel_count = (w * h) as usize;
    if pixel_count == 0 {
        return;
    }

    // Upload pixel data as f32 RGBA
    let flat: Vec<f32> = buf
        .pixels()
        .iter()
        .flat_map(|c| [c.r, c.g, c.b, c.a])
        .collect();

    let pixel_buffer = device.create_buffer_from_pixels(
        &flat,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    );

    let params_buffer = device.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("params"),
        size: params_data.len() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    device.queue().write_buffer(&params_buffer, 0, params_data);

    let shader = device
        .device()
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pixel_shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

    let bind_group_layout =
        device
            .device()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

    let pipeline_layout = device
        .device()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    let pipeline = device
        .device()
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("pixel_compute"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

    let bind_group = device
        .device()
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: pixel_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

    let workgroups = (pixel_count as u32 + 255) / 256;
    let mut encoder = device
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }
    device.queue().submit(Some(encoder.finish()));

    // Read back
    let result = device.read_buffer(&pixel_buffer, (pixel_count * 4 * 4) as u64);
    for i in 0..pixel_count {
        let r = result[i * 4];
        let g = result[i * 4 + 1];
        let b = result[i * 4 + 2];
        let a = result[i * 4 + 3];
        let x = (i % w as usize) as u32;
        let y = (i / w as usize) as u32;
        buf.set(x, y, Color::new(r, g, b, a));
    }
}

/// Execute a composite compute shader (src + dst buffers with params).
pub fn dispatch_composite_shader(
    device: &GpuDevice,
    dst: &mut PixelBuffer,
    src: &PixelBuffer,
    shader_source: &str,
    width: u32,
    height: u32,
    opacity: f32,
) {
    let pixel_count = (width * height) as usize;
    if pixel_count == 0 {
        return;
    }

    let src_flat: Vec<f32> = src
        .pixels()
        .iter()
        .flat_map(|c| [c.r, c.g, c.b, c.a])
        .collect();
    let dst_flat: Vec<f32> = dst
        .pixels()
        .iter()
        .flat_map(|c| [c.r, c.g, c.b, c.a])
        .collect();

    let src_buffer = device.create_buffer_from_pixels(&src_flat, wgpu::BufferUsages::STORAGE);
    let dst_buffer = device.create_buffer_from_pixels(
        &dst_flat,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    );

    // Params: width, height, opacity, padding
    let params: [u32; 4] = [width, height, opacity.to_bits(), 0];
    let params_bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(params.as_ptr() as *const u8, 16) };

    let params_buffer = device.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("composite_params"),
        size: 16,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    device.queue().write_buffer(&params_buffer, 0, params_bytes);

    let shader = device
        .device()
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("composite_shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

    let bind_group_layout =
        device
            .device()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

    let pipeline_layout = device
        .device()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    let pipeline = device
        .device()
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("composite_compute"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

    let bind_group = device
        .device()
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: src_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: dst_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

    let wg_x = (width + 15) / 16;
    let wg_y = (height + 15) / 16;
    let mut encoder = device
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(wg_x, wg_y, 1);
    }
    device.queue().submit(Some(encoder.finish()));

    // Read back dst
    let result = device.read_buffer(&dst_buffer, (pixel_count * 4 * 4) as u64);
    for i in 0..pixel_count {
        let r = result[i * 4];
        let g = result[i * 4 + 1];
        let b = result[i * 4 + 2];
        let a = result[i * 4 + 3];
        let x = (i % width as usize) as u32;
        let y = (i / width as usize) as u32;
        dst.set(x, y, Color::new(r, g, b, a));
    }
}
