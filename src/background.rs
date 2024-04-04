#![allow(dead_code)]

use std::{os::unix::raw::dev_t, sync::Arc};

use crate::{
    shader_manager::{PipelineHandle, ShaderManager},
    texture::TexCube,
};
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    CommandEncoder, ComputePassDescriptor, Device, ShaderStages, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

pub struct Background {
    texture: TexCube,
    texture_array_view: TextureView,
    texture_array_bind_group: wgpu::BindGroup,
    pub texture_cube_bind_group: wgpu::BindGroup,
    pub texture_cube_bind_group_layout: wgpu::BindGroupLayout,
    pipeline: PipelineHandle,
}

static FORMAT: TextureFormat = wgpu::TextureFormat::Rgba16Float;
const DIM: u32 = 2048;
static SIZE: wgpu::Extent3d = wgpu::Extent3d {
    width: DIM,
    height: DIM,
    depth_or_array_layers: 6,
};
impl Background {
    pub fn new(device: &Device, shader_manager: &mut ShaderManager) -> Self {
        let storage_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Background storage texture layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: FORMAT,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                },
                count: None,
            }],
        });
        let cube_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Background storage texture layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    multisampled: false,
                },
                count: None,
            }],
        });

        let pipline_layout = Arc::new(device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Background full pipeline layout"),
                bind_group_layouts: &[&storage_layout],
                push_constant_ranges: &[],
            },
        ));

        let shader = shader_manager.load_shader("./src/background.wgsl");
        let pipeline = shader_manager.add_compute_pipeline(
            vec![shader],
            Box::new(move |shdars, device| {
                let layout = pipline_layout.clone();
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Background pipeline"),
                    layout: Some(&layout),
                    module: shdars[0],
                    entry_point: "full",
                })
            }),
        );

        let tex = device.create_texture(&TextureDescriptor {
            label: Some("Background storage"),
            size: SIZE,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: FORMAT,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view_cube = tex.create_view(&TextureViewDescriptor {
            label: Some("Background texture cube view"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            aspect: wgpu::TextureAspect::All,
            array_layer_count: Some(6),
            ..Default::default()
        });

        let view_array = tex.create_view(&TextureViewDescriptor {
            label: Some("Background texture cube view"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            aspect: wgpu::TextureAspect::All,
            array_layer_count: Some(6),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Background sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            min_filter: wgpu::FilterMode::Nearest,
            mag_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_array_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Background storage bind group"),
            layout: &storage_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view_array),
            }],
        });

        let texture_cube_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Background cube bind group"),
            layout: &cube_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view_cube),
            }],
        });

        let cube = TexCube {
            texture: tex,
            view: view_cube,
            sampler,
        };

        Self {
            texture: cube,
            pipeline,
            texture_array_bind_group,
            texture_array_view: view_array,
            texture_cube_bind_group,
            texture_cube_bind_group_layout: cube_layout,
        }
    }

    pub fn compute(&self, encoder: &mut CommandEncoder, shader_manager: &ShaderManager) {
        if let Some(pipeline) = shader_manager.get_compute_pipeline(&self.pipeline) {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Background pass"),
                ..Default::default()
            });

            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &self.texture_array_bind_group, &[]);
            pass.dispatch_workgroups(DIM / 16, DIM / 16, 6);
        }
    }
}
