use crate::shader_manager::ShaderHandle;
use anymap::AnyMap;
use wgpu::{Device, ShaderModule};

// Traits
pub trait PipelineModuleDescriptor<const NUM_MODULES: usize, PipelineType> {
    type OutputReasorce<'a>: From<[&'a ShaderModule; NUM_MODULES]>;
    type Constructor: PipelineConstructor<PipelineType>;

    fn as_arr(&self) -> [ShaderHandle; NUM_MODULES];

    fn constructor(
        constructor: Box<
            dyn for<'a, 'b, 'c> Fn(
                &'a AnyMap,
                &'b Device,
                Self::OutputReasorce<'c>,
            ) -> PipelineType,
        >,
    ) -> Box<Self::Constructor>;
}
pub trait PipelineConstructor<PipelineType> {
    fn construct(
        &mut self,
        moduels: &[&ShaderModule],
        map: &AnyMap,
        device: &Device,
    ) -> PipelineType;
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
pub struct SplitPipelineConstructor<PipelineType> {
    constructor: Box<dyn Fn(&AnyMap, &Device, SplitPipelineModuleResource) -> PipelineType>,
}

impl<'a> From<[&'a ShaderModule; 2]> for SplitPipelineModuleResource<'a> {
    fn from(value: [&'a ShaderModule; 2]) -> Self {
        Self {
            vertex: &value[0],
            fragment: &value[1],
        }
    }
}

impl<PipelineType> PipelineModuleDescriptor<2, PipelineType> for SplitPipelineModuleDescriptor {
    type OutputReasorce<'a> = SplitPipelineModuleResource<'a>;
    type Constructor = SplitPipelineConstructor<PipelineType>;

    fn constructor(
        constructor: Box<dyn Fn(&AnyMap, &Device, Self::OutputReasorce<'_>) -> PipelineType>,
    ) -> Box<Self::Constructor> {
        Box::new(Self::Constructor { constructor })
    }

    fn as_arr(&self) -> [ShaderHandle; 2] {
        [self.vertex, self.fragment]
    }
}

impl<PipelineType> PipelineConstructor<PipelineType> for SplitPipelineConstructor<PipelineType> {
    fn construct(
        &mut self,
        moduels: &[&ShaderModule],
        map: &AnyMap,
        device: &Device,
    ) -> PipelineType {
        let res = SplitPipelineModuleResource {
            vertex: moduels[0],
            fragment: moduels[1],
        };

        (self.constructor)(map, device, res)
    }
}

// Unified

pub struct UnifiedPipelineModuleDescriptor {
    pub handle: ShaderHandle,
}

pub struct UnifiedPipelineModuleResorce<'a> {
    pub module: &'a ShaderModule,
}

pub struct UnifiedPipelineConstructor<PipelineType> {
    constructor: Box<dyn Fn(&AnyMap, &Device, UnifiedPipelineModuleResorce<'_>) -> PipelineType>,
}
impl Into<[ShaderHandle; 1]> for UnifiedPipelineModuleDescriptor {
    fn into(self) -> [ShaderHandle; 1] {
        [self.handle]
    }
}

impl<'a> From<[&'a ShaderModule; 1]> for UnifiedPipelineModuleResorce<'a> {
    fn from(value: [&'a ShaderModule; 1]) -> Self {
        Self { module: value[0] }
    }
}

impl<PipelineType> PipelineModuleDescriptor<1, PipelineType> for UnifiedPipelineModuleDescriptor {
    type OutputReasorce<'a> = UnifiedPipelineModuleResorce<'a>;
    type Constructor = UnifiedPipelineConstructor<PipelineType>;

    fn constructor(
        constructor: Box<dyn Fn(&AnyMap, &Device, Self::OutputReasorce<'_>) -> PipelineType>,
    ) -> Box<Self::Constructor> {
        Box::new(Self::Constructor { constructor })
    }

    fn as_arr(&self) -> [ShaderHandle; 1] {
        [self.handle]
    }
}

impl<PipelineType> PipelineConstructor<PipelineType> for UnifiedPipelineConstructor<PipelineType> {
    fn construct(
        &mut self,
        moduels: &[&ShaderModule],
        map: &AnyMap,
        device: &Device,
    ) -> PipelineType {
        let res = UnifiedPipelineModuleResorce { module: moduels[0] };
        (self.constructor)(map, device, res)
    }
}

impl std::ops::Deref for UnifiedPipelineModuleResorce<'_> {
    type Target = ShaderModule;

    fn deref(&self) -> &Self::Target {
        self.module
    }
}
