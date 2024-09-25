use std::{ops::Deref, sync::Arc};

use crate::shader_manager::ShaderHandle;
use bevy_ecs::{
    system::{Resource, SystemId},
    world::World,
};
use wgpu::ShaderModule;

// Traits
pub trait PipelineModuleDescriptor<const NUM_MODULES: usize, PipelineType, OutputResource> {
    fn as_arr(&self) -> [ShaderHandle; NUM_MODULES];
    fn constructor(
        id: SystemId<OutputResource, PipelineType>,
    ) -> impl for<'a> PipelineConstructor<PipelineType> + 'static;
}

pub trait PipelineConstructor<PipelineType>: Send + Sync {
    fn construct(&mut self, moduels: &[Arc<ShaderModule>], rr: &mut World) -> PipelineType;
}

// Split pipeline ( different vert and frag moduels )
pub struct SplitPipelineModuleDescriptor {
    pub vertex: ShaderHandle,
    pub fragment: ShaderHandle,
}
pub struct SplitPipelineModuleResource {
    pub vertex: Arc<ShaderModule>,
    pub fragment: Arc<ShaderModule>,
}

pub struct SplitPipelineConstructor<PipelineType> {
    system_id: SystemId<SplitPipelineModuleResource, PipelineType>,
}

impl<'a> From<[Arc<ShaderModule>; 2]> for SplitPipelineModuleResource {
    fn from(value: [Arc<ShaderModule>; 2]) -> Self {
        Self {
            vertex: value[0].clone(),
            fragment: value[1].clone(),
        }
    }
}

impl<'a, PipelineType: 'static>
    PipelineModuleDescriptor<2, PipelineType, SplitPipelineModuleResource>
    for SplitPipelineModuleDescriptor
{
    // type <'a> = SplitPipelineModuleResource;

    fn constructor(
        system_id: SystemId<SplitPipelineModuleResource, PipelineType>,
    ) -> SplitPipelineConstructor<PipelineType> {
        SplitPipelineConstructor { system_id }
    }

    fn as_arr(&self) -> [ShaderHandle; 2] {
        [self.vertex, self.fragment]
    }
}

impl<PipelineType: 'static> PipelineConstructor<PipelineType>
    for SplitPipelineConstructor<PipelineType>
{
    fn construct(&mut self, moduels: &[Arc<ShaderModule>], rr: &mut World) -> PipelineType {
        rr.run_system_with_input(
            self.system_id,
            SplitPipelineModuleResource {
                vertex: moduels[0].clone(),
                fragment: moduels[1].clone(),
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
pub struct UnifiedPipelineModuleResource {
    pub module: Arc<ShaderModule>,
}

pub struct UnifiedPipelineConstructor<PipelineType> {
    system_id: SystemId<UnifiedPipelineModuleResource, PipelineType>,
}

impl<'a, PipelineType: 'static>
    PipelineModuleDescriptor<1, PipelineType, UnifiedPipelineModuleResource>
    for UnifiedPipelineModuleDescriptor
{
    // type <'a> = UnifiedPipelineModuleResource<'a>;

    fn constructor(
        system_id: SystemId<UnifiedPipelineModuleResource, PipelineType>,
    ) -> UnifiedPipelineConstructor<PipelineType> {
        UnifiedPipelineConstructor { system_id }
    }

    fn as_arr(&self) -> [ShaderHandle; 1] {
        [self.handle]
    }
}

impl<PipelineType: 'static> PipelineConstructor<PipelineType>
    for UnifiedPipelineConstructor<PipelineType>
{
    fn construct(&mut self, moduels: &[Arc<ShaderModule>], rr: &mut World) -> PipelineType {
        rr.run_system_with_input(
            self.system_id,
            UnifiedPipelineModuleResource {
                module: moduels[0].clone(),
            },
        )
        .unwrap()
    }
}

impl Deref for UnifiedPipelineModuleResource {
    type Target = ShaderModule;

    fn deref(&self) -> &Self::Target {
        &self.module
    }
}
