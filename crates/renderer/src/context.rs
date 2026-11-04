//! WGPU graphics state and pipeline management.

use crate::camera::Camera;
use crate::shader::CHUNK_SHADER;
use crate::vertex::{BlockVertex, ChunkMesh};
use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

/// GPU rendering context managing Surface, Device, Queue, and Pipelines.
pub struct RenderContext {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    /// Physical pixel size of the window surface.
    pub size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth_texture_view: wgpu::TextureView,
    /// Persistent GPU vertex buffer; reallocated only when the mesh grows.
    vertex_buffer: Option<wgpu::Buffer>,
    /// Persistent GPU index buffer; reallocated only when the mesh grows.
    index_buffer: Option<wgpu::Buffer>,
    /// Number of indices currently stored in `index_buffer`.
    index_count: u32,
    /// Capacity of `vertex_buffer` in bytes (to detect growth).
    vertex_buf_cap: u64,
    /// Capacity of `index_buffer` in bytes.
    index_buf_cap: u64,
}

impl RenderContext {
    /// Initializes WGPU surface and pipeline for a given window.
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // SAFETY: window Arc is kept alive
        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find suitable GPU adapter"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: Some("Minecraft Device"),
                },
                None,
            )
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            // AutoNoVsync: prefer Immediate/Mailbox to allow > 60 FPS.
            // Falls back to Fifo only if the driver/surface has no other option.
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Camera Uniform Buffer
        let camera_uniform = CameraUniform {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        };

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        // Depth Texture
        let depth_texture_view = Self::create_depth_view(&device, &config);

        // Shader and Pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Chunk Shader"),
            source: wgpu::ShaderSource::Wgsl(CHUNK_SHADER.into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Chunk Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[BlockVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            camera_buffer,
            camera_bind_group,
            depth_texture_view,
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
            vertex_buf_cap: 0,
            index_buf_cap: 0,
        })
    }

    fn create_depth_view(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> wgpu::TextureView {
        let size = wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Resizes the surface when window size changes.
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture_view = Self::create_depth_view(&self.device, &self.config);
        }
    }

    /// Updates the camera View-Projection uniform matrix.
    pub fn update_camera(&self, camera: &Camera) {
        let vp = camera.view_projection_matrix();
        let uniform = CameraUniform {
            view_proj: vp.to_cols_array_2d(),
        };
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    /// Renders a chunk mesh to the window surface.
    ///
    /// GPU buffers are reused across frames and only reallocated when the mesh grows,
    /// eliminating a large per-frame allocation on the GPU allocator.
    pub fn render(&mut self, mesh: &ChunkMesh) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Update persistent GPU buffers only when the mesh has geometry.
        if !mesh.is_empty() {
            let v_bytes: &[u8] = bytemuck::cast_slice(&mesh.vertices);
            let i_bytes: &[u8] = bytemuck::cast_slice(&mesh.indices);

            // Reallocate vertex buffer if capacity is insufficient.
            if v_bytes.len() as u64 > self.vertex_buf_cap {
                self.vertex_buffer = Some(self.device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: Some("Vertex Buffer"),
                        contents: v_bytes,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    },
                ));
                self.vertex_buf_cap = v_bytes.len() as u64;
            } else if let Some(buf) = &self.vertex_buffer {
                // Buffer is large enough — upload in-place with no allocation.
                self.queue.write_buffer(buf, 0, v_bytes);
            }

            // Reallocate index buffer if capacity is insufficient.
            if i_bytes.len() as u64 > self.index_buf_cap {
                self.index_buffer = Some(self.device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: Some("Index Buffer"),
                        contents: i_bytes,
                        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    },
                ));
                self.index_buf_cap = i_bytes.len() as u64;
            } else if let Some(buf) = &self.index_buffer {
                self.queue.write_buffer(buf, 0, i_bytes);
            }

            self.index_count = mesh.indices.len() as u32;
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Minecraft sky-blue background colour.
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.47,
                            g: 0.65,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let (Some(v_buf), Some(i_buf)) = (&self.vertex_buffer, &self.index_buffer) {
                if self.index_count > 0 {
                    render_pass.set_pipeline(&self.render_pipeline);
                    render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                    render_pass.set_vertex_buffer(0, v_buf.slice(..));
                    render_pass.set_index_buffer(i_buf.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..self.index_count, 0, 0..1);
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
