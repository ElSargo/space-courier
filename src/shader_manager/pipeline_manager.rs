use crate::shader_manager::PipelineModuleDescriptor;
use bevy_ecs::{system::Resource, world::World};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    ops::Deref,
    sync::{Arc, RwLock, RwLockReadGuard},
};
use wgpu::ShaderModule;

use super::{pipeline_shader_resource::PipelineConstructor, PipelineHandle, ShaderHandle};

pub type Adder<PipelineType> =
    Box<dyn FnOnce(&mut PipelineManager<PipelineType>, &mut World) + Send + Sync>;

pub(super) struct PipelineManager<PipelineType> {
    creation_queue: Receiver<Adder<PipelineType>>,
    pipelines_requirements: BTreeMap<ShaderHandle, Vec<PipelineHandle>>,
    pipline_reasoureces: BTreeMap<PipelineHandle, Vec<ShaderHandle>>,
    pipeline_connstructors: BTreeMap<PipelineHandle, Box<dyn PipelineConstructor<PipelineType>>>,
    pipelines: Arc<RwLock<BTreeMap<PipelineHandle, PipelineType>>>,
    pipelines_to_init: BTreeSet<PipelineHandle>,
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
                creation_queue: rcv,
                pipelines_requirements: BTreeMap::new(),
                pipelines_to_init: BTreeSet::new(),

                pipelines: pipelines.clone(),

                pipeline_connstructors: BTreeMap::new(),

                pipline_reasoureces: BTreeMap::new(),
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
            self.pipelines_requirements
                .entry(shader_handle)
                .or_insert(vec![])
                .push(pipeline_handle)
        }

        self.pipline_reasoureces
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
        self.pipelines_to_init.remove(pipline_handle);
        if let Some(reasources) = self.pipline_reasoureces.get(pipline_handle) {
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
        if let Some(pipelines_to_reconstruct) = self.pipelines_requirements.get(&shader).cloned() {
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
        while let Ok(f) = self.creation_queue.try_recv() {
            f(self, rr);
        }
        self.recompile_dependants(shader_handle, &shaders, rr);
        for pipeline_handle in self.pipelines_to_init.clone().iter() {
            self.proccess_pipline(&pipeline_handle, &shaders, rr);
        }
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
