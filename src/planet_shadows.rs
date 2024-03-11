use bevy_ecs::system::Resource;

use glam::Vec3;
use wgpu::{
    include_wgsl, BindGroupLayout, BlendComponent, ColorWrites, Device, Extent3d,
    PipelineLayoutDescriptor, PrimitiveState, RenderPipelineDescriptor, SurfaceConfiguration,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, VertexAttribute,
};

use crate::texture;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct RawPlanetShadow {
    position: Vec3,
    shadow_direction: Vec3,
    shadow_length: f32,
    shadow_height: f32,
    radius: f32,
}

#[derive(Resource)]
pub struct PlanetShadows {
    result_texture: wgpu::Texture,
    pipeline: wgpu::RenderPipeline,
}
impl RawPlanetShadow {
    const ATRIBS: [VertexAttribute; 5] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32, 3 => Float32, 4 => Float32];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATRIBS,
        }
    }
}

impl PlanetShadows {
    pub fn new(
        device: &Device,
        config: &SurfaceConfiguration,
        camera_bind_group_layout: &BindGroupLayout,
    ) -> Self {
        let result_texture = device.create_texture(&TextureDescriptor {
            label: Some("Planet shadow texture"),
            size: Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rg16Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Planet shadow pipeline layout"),
            bind_group_layouts: &[camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader_module = device.create_shader_module(include_wgsl!("planet_shadows.wgsl"));

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Planet shadow pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[RawPlanetShadow::desc()],
            },
            primitive: PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Greater, // 1.
                stencil: wgpu::StencilState::default(),        // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: TextureFormat::Rg16Float,
                    blend: Some(wgpu::BlendState {
                        color: BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Max,
                        },
                    }),
                    write_mask: ColorWrites::RED | ColorWrites::GREEN,
                })],
            }),

            multiview: None,
        });

        Self {
            result_texture,
            pipeline,
        }
    }
}
