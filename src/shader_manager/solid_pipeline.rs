use std::{marker::PhantomData, sync::Arc};

use crate::{
    background::Background, camera::CameraUniform, gpu::RD, shader_manager::ShaderHandle, texture,
};
use bevy_ecs::{
    system::{In, IntoSystem, Res, SystemId},
    world::World,
};
use wgpu::{
    BindGroupLayout, FragmentState, PipelineLayoutDescriptor, ShaderModule, TextureFormat,
    VertexBufferLayout,
};

use super::{
    pipeline_shader_resource::PipelineShaderDescriptor, PipelineConstructor,
    PipelineModuleDescriptor,
};

pub struct SolidSplitShaders {
    pub vertex: ShaderHandle,
    pub fragment: ShaderHandle,
}

pub struct SolidShader {
    pub handle: ShaderHandle,
}

impl<M, S: IntoSystem<(), SolidPipelineModuleResourceExtras, M> + 'static>
    PipelineShaderDescriptor<1, wgpu::RenderPipeline, S, M> for SolidShader
{
    type Descriptor = SolidPipelineModuleDescriptor<1, S, M>;

    fn into_descriptor(self, system: S) -> SolidPipelineModuleDescriptor<1, S, M> {
        SolidPipelineModuleDescriptor {
            shaders: [self.handle],
            system,
            phantom: PhantomData,
        }
    }
}

impl<M, S: IntoSystem<(), SolidPipelineModuleResourceExtras, M> + 'static>
    PipelineShaderDescriptor<2, wgpu::RenderPipeline, S, M> for SolidSplitShaders
{
    type Descriptor = SolidPipelineModuleDescriptor<2, S, M>;

    fn into_descriptor(self, system: S) -> SolidPipelineModuleDescriptor<2, S, M> {
        SolidPipelineModuleDescriptor {
            shaders: [self.vertex, self.fragment],
            system,
            phantom: PhantomData,
        }
    }
}

pub struct SolidPipelineModuleDescriptor<const N: usize, S, M>
where
    S: IntoSystem<(), SolidPipelineModuleResourceExtras, M> + 'static,
{
    shaders: [ShaderHandle; N],
    system: S,
    phantom: PhantomData<M>,
}

pub struct SolidPipelineModuleResourceExtras {
    pub vertex_buffers: Arc<[VertexBufferLayout<'static>]>,
    pub extra_bind_groups: Vec<Arc<BindGroupLayout>>,
}

pub struct SolidPipelineModuleResource {
    pub vertex: Arc<ShaderModule>,
    pub fragment: Arc<ShaderModule>,
    pub extras: SolidPipelineModuleResourceExtras,
}

impl<
        'a,
        const N: usize,
        System: bevy_ecs::system::IntoSystem<(), SolidPipelineModuleResourceExtras, M>,
        M,
    > PipelineModuleDescriptor<N, wgpu::RenderPipeline, System, M>
    for SolidPipelineModuleDescriptor<N, System, M>
{
    // type Shaders = SolidSplitShaders;
    fn constructor(self, world: &mut World) -> SolidPipelineConstructor {
        SolidPipelineConstructor {
            descriptor_system_id: world.register_system(self.system),
            system_id: world.register_system(solid_pass_render_descriptor),
        }
    }

    fn as_arr(&self) -> [ShaderHandle; N] {
        self.shaders
    }
}

impl PipelineConstructor<wgpu::RenderPipeline> for SolidPipelineConstructor {
    fn construct(&mut self, moduels: &[Arc<ShaderModule>], rr: &mut World) -> wgpu::RenderPipeline {
        let extras = rr.run_system(self.descriptor_system_id).unwrap();
        rr.run_system_with_input(
            self.system_id,
            SolidPipelineModuleResource {
                extras,
                vertex: moduels[0].clone(),
                fragment: moduels.get(1).unwrap_or(&moduels[0]).clone(),
            },
        )
        .unwrap()
    }
}

pub struct SolidPipelineConstructor {
    descriptor_system_id: SystemId<(), SolidPipelineModuleResourceExtras>,
    system_id: SystemId<SolidPipelineModuleResource, wgpu::RenderPipeline>,
}

pub fn solid_pass_render_descriptor<'a>(
    In(descriptor): In<SolidPipelineModuleResource>,
    device: RD,
    camera: Res<CameraUniform>,
    bg: Res<Background>,
) -> wgpu::RenderPipeline {
    let a = [
        &camera.bind_group_layout,
        &bg.texture_cube_bind_group_layout,
    ];

    let b = descriptor
        .extras
        .extra_bind_groups
        .iter()
        .map(|x| x.as_ref())
        .collect::<Vec<_>>();

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("tri mesh pipeline layout"),
        bind_group_layouts: [a.as_slice(), b.as_slice()]
            .iter()
            .flat_map(|x| x.iter().map(|x| *x))
            .collect::<Vec<_>>()
            .as_slice(),
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Tri mesh render pipeline"),
        layout: Some(&pipeline_layout),
        cache: None,
        vertex: wgpu::VertexState {
            module: &descriptor.vertex,
            entry_point: "vs_main",
            buffers: &descriptor.extras.vertex_buffers,
            compilation_options: Default::default(),
        },
        fragment: Some(FragmentState {
            module: &descriptor.fragment,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: TextureFormat::Rgba16Float,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: texture::Texture::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Greater, // 1.
            stencil: wgpu::StencilState::default(),        // 2.
            bias: wgpu::DepthBiasState::default(),
        }), // 1.
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList, // 1.
            strip_index_format: None,
            front_face: wgpu::FrontFace::Cw, // 2.
            cull_mode: Some(wgpu::Face::Back),
            // cull_mode: None,
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        multisample: wgpu::MultisampleState {
            count: 1,                         // 2.
            mask: !0,                         // 3.
            alpha_to_coverage_enabled: false, // 4.
        },
        multiview: None, // 5.
    })
}
