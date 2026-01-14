mod texture;
use std::{
    collections::HashMap,
    mem,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Ok;
use glam::{Vec2, vec2};
use wgpu::{util::DeviceExt, wgc::pipeline};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
    map::{Map, TileData, TileType, TileTypes},
    raycaster::WallInstance,
    renderer::texture::{Texture, load_asset},
};

struct TileTextureMaps {
    wall_image_map: HashMap<usize, usize>,
    floor_image_map: HashMap<usize, usize>,
    ceiling_image_map: HashMap<usize, usize>,
}

struct Textures {
    wall_texture_arr: Option<Texture>,
    floor_texture_arr: Option<Texture>,
    ceiling_texture_arr: Option<Texture>,
}

pub(crate) enum TextureCategory {
    Wall,
    Floor,
    Ceiling,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: Vec2,
    uv: Vec2,
}

const VERTICES: [Vertex; 4] = [
    Vertex {
        pos: vec2(0.0, 0.0),
        uv: vec2(0.0, 0.0),
    },
    Vertex {
        pos: vec2(1.0, 0.0),
        uv: vec2(1.0, 0.0),
    },
    Vertex {
        pos: vec2(1.0, 1.0),
        uv: vec2(1.0, 0.0),
    },
    Vertex {
        pos: vec2(0.0, 1.0),
        uv: vec2(0.0, 1.0),
    },
];

const INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];

pub(crate) struct Renderer {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    is_surface_configured: bool,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    quad_vertex_buffer: wgpu::Buffer,
    quad_index_buffer: wgpu::Buffer,
    quad_instance_buffer: wgpu::Buffer,
    textures: Textures,
    tile_texture_maps: TileTextureMaps,
    wall_instances: Vec<WallInstance>,
    last_frame_time: Option<Instant>,
    delta_time: Duration,
}

impl Renderer {
    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    pub async fn new(window: &Arc<Window>, map: &Map) -> anyhow::Result<Self> {
        // let tile_types: &TileTypes;
        let window = window.clone();
        let size = window.inner_size();
        let (surface, device, queue, config) = wgpu_init(&window, size).await?;

        let (textures, tile_texture_maps) = load_textures(map, &device, &queue)?;
        let wall_texture_arr = textures.wall_texture_arr.as_ref().unwrap();
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let vertex_buffer_layouts = [
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<WallInstance>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![2 => Float32, 3 => Float32, 4 => Float32, 5 => Float32, 6 => Uint32],
            },
        ];

        let wall_instances = vec![WallInstance::default(); config.width as usize];

        let quad_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(&VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let quad_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Index Buffer"),
            contents: bytemuck::cast_slice(&INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let quad_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quad Instance Buffer"),
            size: (mem::size_of::<WallInstance>() * config.width as usize) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&wall_texture_arr.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&wall_texture_arr.sampler),
                },
            ],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &vertex_buffer_layouts,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: Default::default(),
        });

        Ok(Renderer {
            window,
            surface,
            is_surface_configured: false,
            device,
            queue,
            config,
            render_pipeline,
            bind_group,
            quad_vertex_buffer,
            quad_index_buffer,
            quad_instance_buffer,
            textures,
            tile_texture_maps,
            wall_instances,
            last_frame_time: Some(Instant::now()),
            delta_time: Duration::default(),
        })
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        let now = Instant::now();
        self.delta_time = Instant::now() - self.last_frame_time.unwrap_or(now);

        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

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
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            self.queue.write_buffer(
                &self.quad_instance_buffer,
                0,
                bytemuck::cast_slice(&self.wall_instances),
            );

            render_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.quad_instance_buffer.slice(..));
            render_pass
                .set_index_buffer(self.quad_index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            render_pass.set_pipeline(&self.render_pipeline);

            render_pass.set_bind_group(0, &self.bind_group, &[]);

            render_pass.draw_indexed(0..6, 0, 0..self.config.width);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        self.last_frame_time = Some(now);

        Ok(())
    }

    pub fn set_wall_instance(
        &mut self,
        index: usize,
        instance: WallInstance,
    ) -> anyhow::Result<()> {
        self.wall_instances[index] = instance;

        Ok(())
    }

    pub fn get_texture_index(
        &self,
        k: u8,
        texture_category: &TextureCategory,
    ) -> anyhow::Result<usize> {
        match texture_category {
            TextureCategory::Wall => Ok(*self
                .tile_texture_maps
                .wall_image_map
                .get(&(k as usize))
                .unwrap()),
            TextureCategory::Floor => Ok(*self
                .tile_texture_maps
                .floor_image_map
                .get(&(k as usize))
                .unwrap()),
            TextureCategory::Ceiling => Ok(*self
                .tile_texture_maps
                .ceiling_image_map
                .get(&(k as usize))
                .unwrap()),
        }
    }

    pub fn delta_time(&self) -> Duration {
        self.delta_time
    }
}

async fn wgpu_init(
    window: &Arc<Window>,
    size: PhysicalSize<u32>,
) -> anyhow::Result<(
    wgpu::Surface<'static>,
    wgpu::Device,
    wgpu::Queue,
    wgpu::SurfaceConfiguration,
)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });

    let surface = instance
        .create_surface(window.clone())
        .expect("Failed to create surface");

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await?;

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
        })
        .await?;

    let surface_caps = surface.get_capabilities(&adapter);

    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    Ok((surface, device, queue, config))
}

pub fn load_textures(
    map: &Map,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<(Textures, TileTextureMaps)> {
    let mut wall_image_map: HashMap<usize, usize> = HashMap::new();
    let mut floor_image_map: HashMap<usize, usize> = HashMap::new();
    let mut ceiling_image_map: HashMap<usize, usize> = HashMap::new();

    let mut wall_byte_array: Vec<Vec<u8>> = Vec::new();
    let mut floor_byte_array: Vec<Vec<u8>> = Vec::new();
    let mut ceiling_byte_array: Vec<Vec<u8>> = Vec::new();

    for (k, v) in map.tile_types() {
        match v {
            TileType::Wall(data) => {
                wall_image_map.insert(*k as usize, wall_byte_array.len());

                wall_byte_array.push(load_asset(data.texture_path)?);
            }
            TileType::Floor(data) => {
                floor_image_map.insert(*k as usize, floor_byte_array.len());

                floor_byte_array.push(load_asset(data.texture_path)?);
            }
            TileType::Ceiling(data) => {
                ceiling_image_map.insert(*k as usize, ceiling_byte_array.len());

                ceiling_byte_array.push(load_asset(data.texture_path)?);
            }
            TileType::FloorCeiling(data) => {
                floor_image_map.insert(*k as usize, floor_byte_array.len());
                ceiling_image_map.insert(*k as usize, ceiling_byte_array.len() + 1);

                floor_byte_array.push(load_asset(data.texture_path_f)?);
                ceiling_byte_array.push(load_asset(data.texture_path_c)?);
            }
        };
    }

    let wall_texture_arr =
        texture::Texture::from_bytes_array(device, queue, &wall_byte_array, "Wall Texture Array");

    let floor_texture_arr =
        texture::Texture::from_bytes_array(device, queue, &floor_byte_array, "Floor Texture Array");

    let ceiling_texture_arr = texture::Texture::from_bytes_array(
        device,
        queue,
        &ceiling_byte_array,
        "Ceiling Texture Array",
    );

    Ok((
        Textures {
            wall_texture_arr,
            floor_texture_arr,
            ceiling_texture_arr,
        },
        TileTextureMaps {
            wall_image_map,
            floor_image_map,
            ceiling_image_map,
        },
    ))
}
