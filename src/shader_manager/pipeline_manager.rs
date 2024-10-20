use crate::shader_manager::PipelineModuleDescriptor;
use bevy_ecs::{system::Resource, world::World};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::{
    collections::{BTreeMap, HashMap},
    ops::Deref,
    sync::{Arc, RwLock, RwLockReadGuard},
};
use wgpu::ShaderModule;

use super::{pipeline_shader_resource::PipelineConstructor, PipelineHandle, ShaderHandle};

pub type Adder<PipelineType> =
    Box<dyn FnOnce(&mut PipelineManager<PipelineType>, &mut World) + Send + Sync>;

pub(super) struct PipelineManager<PipelineType> {
    /// Queue of procedures that register a pipelines constructor on the world
    /// This exists so code can create render pipelines without needing exclusive world access
    pipelines_to_register: Receiver<Adder<PipelineType>>,
    /// Map of pipelines that need to be recompiled when a shader changes
    pipelines_depending_on: BTreeMap<ShaderHandle, Vec<PipelineHandle>>,
    /// Map of the shader modules a pipeline needs to be created
    modules_for_pipeline: BTreeMap<PipelineHandle, Vec<ShaderHandle>>,
    /// Functions that create the actuall pipelines
    /// These are passed the required modules and can access the world
    pipeline_connstructors: BTreeMap<PipelineHandle, Box<dyn PipelineConstructor<PipelineType>>>,
    /// Where the pipelines are stored
    pipelines: Arc<RwLock<BTreeMap<PipelineHandle, PipelineType>>>,
}

#[derive(Resource)]
pub struct PipelinesView<PipelineType>(Arc<RwLock<BTreeMap<PipelineHandle, PipelineType>>>);

impl<T> Clone for PipelinesView<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub struct PipelineView<'a, PipelineType> {
    guard: RwLockReadGuard<'a, BTreeMap<PipelineHandle, PipelineType>>,
    handle: PipelineHandle,
}

impl<'a, PipelineType> Deref for PipelineView<'a, PipelineType> {
    type Target = PipelineType;

    fn deref(&self) -> &Self::Target {
        self.guard.get(&self.handle).map(|sp|sp).expect(
            "\
Pipeline dereferenced without being created, something is broken with the shader manager, 
this shouldn't happen since the manager validates that the pipeline exists before returning the view"
        )
    }
}

impl<PipelineType: 'static> PipelineManager<PipelineType> {
    pub fn new() -> (
        Self,
        PipelinesView<PipelineType>,
        Sender<Adder<PipelineType>>,
    ) {
        let pipelines = Arc::new(RwLock::new(BTreeMap::new()));
        let (send, rcv) = unbounded();
        (
            Self {
                pipelines_to_register: rcv,
                pipelines_depending_on: BTreeMap::new(),

                pipelines: pipelines.clone(),

                pipeline_connstructors: BTreeMap::new(),

                modules_for_pipeline: BTreeMap::new(),
            },
            PipelinesView(pipelines),
            send,
        )
    }

    pub fn add_pipeline<const N: usize, Descriptor, System, M>(
        &mut self,
        descriptor: Descriptor,
        pipeline_handle: PipelineHandle,
        rr: &mut World,
    ) where
        Descriptor: PipelineModuleDescriptor<N, PipelineType, System, M>, // Descriptor: PipelineModuleDescriptor<N, PipelineType, System, M> + Send + Sync,
    {
        for shader_handle in descriptor.as_arr() {
            self.pipelines_depending_on
                .entry(shader_handle)
                .or_insert(vec![])
                .push(pipeline_handle)
        }

        self.modules_for_pipeline
            .insert(pipeline_handle, descriptor.as_arr().to_vec());
        self.pipeline_connstructors
            .insert(pipeline_handle, Box::new(descriptor.constructor(rr)));
    }

    pub fn proccess_pipline(
        &mut self,
        pipline_handle: &PipelineHandle,
        shaders: &HashMap<ShaderHandle, Arc<ShaderModule>>,
        rr: &mut World,
    ) {
        if let Some(reasources) = self.modules_for_pipeline.get(pipline_handle) {
            let mut compiled_shaders = Vec::with_capacity(reasources.len());
            for sh in reasources {
                if let Some(shader) = shaders.get(sh) {
                    compiled_shaders.push(shader);
                } else {
                    return;
                }
            }
            if let Some(constructor) = self.pipeline_connstructors.get_mut(pipline_handle) {
                let new_pipeline = constructor.construct(
                    compiled_shaders
                        .iter()
                        .map(|s| (**s).clone())
                        .collect::<Vec<_>>()
                        .as_slice(),
                    rr,
                );
                let mut guard = self.pipelines.write().unwrap();
                guard.insert(*pipline_handle, new_pipeline);
            }
        }
    }

    pub fn recompile_dependants(
        &mut self,
        shader: ShaderHandle,
        shaders: &HashMap<ShaderHandle, Arc<ShaderModule>>,
        rr: &mut World,
    ) {
        if let Some(pipelines_to_reconstruct) = self.pipelines_depending_on.get(&shader).cloned() {
            for pipline_handle in pipelines_to_reconstruct {
                self.proccess_pipline(&pipline_handle, &shaders, rr);
            }
        }
    }

    pub fn process_pipelines(
        &mut self,
        shaders: &HashMap<ShaderHandle, Arc<ShaderModule>>,
        rr: &mut World,
        shader_handle: ShaderHandle,
    ) {
        while let Ok(f) = self.pipelines_to_register.try_recv() {
            f(self, rr);
        }
        self.recompile_dependants(shader_handle, &shaders, rr);
    }
}

impl<PipelineType> PipelinesView<PipelineType> {
    pub fn get(&self, handle: &PipelineHandle) -> Option<PipelineView<PipelineType>> {
        let guard = self.0.read().unwrap();
        guard.contains_key(handle).then_some(PipelineView {
            guard,
            handle: *handle,
        })
    }
}
