mod vertex;

use wgpu::StorageTextureAccess;
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
    window::WindowBuilder,
};

use crate::vertex::Vertex;

const VOLUME_SIZE: u32 = 800;

fn create_plane(size: f32) -> (Vec<Vertex>, Vec<u16>) {
    // Set up position data and UV coordinates (2 attributes)
    let vertex_data = [
        Vertex::new([size, -size, 0.0, 1.0], [1.0, 1.0]),
        Vertex::new([size, size, 0.0, 1.0], [1.0, 0.0]),
        Vertex::new([-size, -size, 0.0, 1.0], [0.0, 1.0]),
        Vertex::new([-size, size, 0.0, 1.0], [0.0, 0.0]),
    ];

    // Set up indices (draw a pair of triangles)
    let index_data: &[u16] = &[0, 1, 2, 2, 1, 3];

    (vertex_data.to_vec(), index_data.to_vec())
}

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    plane_vertex_buffer: wgpu::Buffer,
    plane_index_buffer: wgpu::Buffer,
    application_data_buffer: wgpu::Buffer,
    bind_group_compute: wgpu::BindGroup,
    bind_group_render: wgpu::BindGroup,
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,
    current_frame: u32,
}

impl State {
    // Creating some of the types requires async code
    async fn new(window: &Window) -> Self {
        // Set up adapter, device, and surface
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        // Create the device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        // Configure and create the display surface
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        // Set up buffers to draw full-screen quad
        let (plane_vertex_data, plane_index_data) = create_plane(1.0);
        let plane_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Plane Vertex Buffer"),
            contents: bytemuck::cast_slice(&plane_vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let plane_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Plane Index Buffer"),
            contents: bytemuck::cast_slice(&plane_index_data),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Describe how the vertex buffer is laid out in memory (how many attributes, etc.)
        let attributes = wgpu::vertex_attr_array![
            // Attrib #0: position, vec4
            0 => Float32x4,
            // Attrib #1: uv, vec2
            1 => Float32x2
        ];
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &attributes,
        };

        // Set up uniforms that contain simple application data
        let application_data = [
            0.0,
        ].to_vec();
        let application_data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Application Data Buffer"),
            contents: bytemuck::cast_slice(&application_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Set up volume data texture
        let volume_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: VOLUME_SIZE,
                height: VOLUME_SIZE,
                depth_or_array_layers: VOLUME_SIZE,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba8Sint,
            usage: wgpu::TextureUsages::STORAGE_BINDING,
            label: None,
        });
        let volume_texture_view = volume_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Set up bind group layouts
        let bind_group_layout_compute =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Sint,
                        view_dimension: wgpu::TextureViewDimension::D3,
                    },
                    count: None,
                }],
                label: None,
            });

        let bind_group_layout_render =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new((application_data.len() * std::mem::size_of::<f32>()) as _),
                        },
                        count: None
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::ReadOnly,
                            format: wgpu::TextureFormat::Rgba8Sint,
                            view_dimension: wgpu::TextureViewDimension::D3
                        },
                        count: None,
                    },
                ],
                label: None,
            });

        // Set up bind groups
        let bind_group_compute = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout_compute,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&volume_texture_view),
            }],
            label: Some("bind_group_compute"),
        });

        let bind_group_render = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout_render,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: application_data_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&volume_texture_view),
                },
            ],
            label: Some("bind_group_render"),
        });

        // Set up shader modules, pipeline layouts, and pipelines
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout_compute],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Render Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("render.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout_render],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: "vs_main",
                buffers: &[vertex_buffer_layout],
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,
            plane_vertex_buffer,
            plane_index_buffer,
            application_data_buffer,
            bind_group_compute,
            bind_group_render,
            compute_pipeline,
            render_pipeline,
            current_frame: 0u32,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {}

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Increment frame
        self.current_frame += 1;

        // Update uniform buffer
        self.queue.write_buffer(
            &self.application_data_buffer,0,bytemuck::cast_slice(&[
                self.current_frame as f32
            ]),
        );

        // Grab ref to output surface
        let output = self.surface.get_current_texture()?;
        let output_texture_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create command buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: None,
            });

        // Encode commands in the command buffer
        encoder.push_debug_group("Compute Pass");
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            pass.set_pipeline(&self.compute_pipeline);
            pass.set_bind_group(0, &self.bind_group_compute, &[]);
            pass.dispatch_workgroups(VOLUME_SIZE / 4, VOLUME_SIZE / 4, VOLUME_SIZE / 4);
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("Render Pass");
        {
            // First, describe our render pass
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            // Then, draw objects
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group_render, &[]);
            render_pass
                .set_index_buffer(self.plane_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_vertex_buffer(0, self.plane_vertex_buffer.slice(..));
            render_pass.draw_indexed(0..6, 0, 0..1);
        }
        encoder.pop_debug_group();

        // Finish command buffer and submit it to the command queue
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub async fn run() {
    println!("Starting application");

    // Window setup
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    window.set_inner_size(winit::dpi::LogicalSize::new(512.0, 512.0));

    // Setup
    let mut state = State::new(&window).await;

    // Event loop
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    });
}
