use std::ops::Deref;

use crate::shader_manager::ShaderHandle;
use bevy_ecs::{
    system::{Resource, SystemId},
    world::World,
};
use wgpu::{Device, PipelineCacheDescriptor, ShaderModule};
use winit::monitor;

// Traits
pub trait PipelineModuleDescriptor<const NUM_MODULES: usize, PipelineType, OutputResource> {
    fn as_arr(&self) -> [ShaderHandle; NUM_MODULES];
    fn constructor(
        id: SystemId<OutputResource, PipelineType>,
    ) -> impl PipelineConstructor<'static, PipelineType>;
}

pub trait PipelineConstructor<'b, PipelineType> {
    fn construct(&mut self, moduels: &[&ShaderModule], rr: &World) -> PipelineType;
}

// Split pipeline ( different vert and frag moduels )
pub struct SplitPipelineModuleDescriptor {
    pub vertex: ShaderHandle,
    pub fragment: ShaderHandle,
}
pub struct SplitPipelineModuleResource<'a> {
    pub vertex: &'a ShaderModule,
    pub fragment: &'a ShaderModule,
}

pub struct SplitPipelineConstructor<'a, PipelineType> {
    system_id: SystemId<SplitPipelineModuleResource<'a>, PipelineType>,
}

impl<'a> From<[&'a ShaderModule; 2]> for SplitPipelineModuleResource<'a> {
    fn from(value: [&'a ShaderModule; 2]) -> Self {
        Self {
            vertex: &value[0],
            fragment: &value[1],
        }
    }
}

impl<'a, PipelineType> PipelineModuleDescriptor<2, PipelineType, SplitPipelineModuleResource<'a>>
    for SplitPipelineModuleDescriptor
{
    // type <'a> = SplitPipelineModuleResource<'a>;

    fn constructor(
        system_id: SystemId<SplitPipelineModuleResource<'a>, PipelineType>,
    ) -> SplitPipelineConstructor<'_, PipelineType> {
        SplitPipelineConstructor { system_id }
    }

    fn as_arr(&self) -> [ShaderHandle; 2] {
        [self.vertex, self.fragment]
    }
}

impl<PipelineType> PipelineConstructor<'_, PipelineType>
    for SplitPipelineConstructor<'_, PipelineType>
{
    fn construct(&mut self, moduels: &[&ShaderModule], rr: &World) -> PipelineType {
        rr.run_system_with_input(
            self.system_id,
            SplitPipelineModuleResource {
                vertex: moduels[0],
                fragment: moduels[1],
            },
        )
        .unwrap()
    }
}

// Unified

pub struct UnifiedPipelineModuleDescriptor {
    pub handle: ShaderHandle,
}

#[derive(Resource)]
pub struct UnifiedPipelineModuleResource<'a> {
    pub module: &'a ShaderModule,
}

pub struct UnifiedPipelineConstructor<'a, PipelineType> {
    system_id: SystemId<UnifiedPipelineModuleResource<'a>, PipelineType>,
}

impl<'a, PipelineType> PipelineModuleDescriptor<1, PipelineType, UnifiedPipelineModuleResource<'a>>
    for UnifiedPipelineModuleDescriptor
{
    // type <'a> = UnifiedPipelineModuleResource<'a>;

    fn constructor(
        system_id: SystemId<UnifiedPipelineModuleResource<'a>, PipelineType>,
    ) -> UnifiedPipelineConstructor<'_, PipelineType> {
        UnifiedPipelineConstructor { system_id }
    }

    fn as_arr(&self) -> [ShaderHandle; 1] {
        [self.handle]
    }
}

impl<PipelineType> PipelineConstructor<'_, PipelineType>
    for UnifiedPipelineConstructor<'_, PipelineType>
{
    fn construct(&mut self, moduels: &[&ShaderModule], rr: &World) -> PipelineType {
        rr.run_system_with_input(
            self.system_id,
            UnifiedPipelineModuleResource { module: moduels[0] },
        )
        .unwrap()
    }
}

impl Deref for UnifiedPipelineModuleResource<'_> {
    type Target = ShaderModule;

    fn deref(&self) -> &Self::Target {
        self.module
    }
}
