use super::{
    path_to_shader_id, pipeline_manager::Adder, PipelineHandle, PipelineManager, ShaderHandle,
};
use crate::shader_manager::pipeline_shader_resource::ShadersD;
use crate::{log, log_expand};
use bevy_ecs::system::{IntoSystem, Resource};
use crossbeam_channel::Sender;
use rand::random;
use std::{
    marker::{Send, Sync},
    path::PathBuf,
};

/// Loads a shader using a path relative to the current file
#[macro_export]
macro_rules! load_shader {
    ( $sm:expr, $path:expr) => {{
        let mut path: std::path::PathBuf = file!().into();
        path.pop();
        path.push($path);
        $sm.load_shader(path)
    }};
}

/// Ecs accesible struct to obtain shader and pipeline handles
/// Each method imideatly returns a handle without blocking and
/// shceduals the requested reasorce to be created
#[derive(Resource, Clone)]
pub struct ShaderLoader {
    /// Used to tell the whatcher which files should be compiled
    shader_path_whitelist_queue: Sender<(PathBuf, ShaderHandle)>,
    /// Used to create render pipelines, they are queues beacuse the creation
    /// is defered until right before the render stage when the provied consturtors
    /// can have exclusive access to the ecs world
    compute_constructor_registration_queue: Sender<Adder<wgpu::ComputePipeline>>,
    render_constructor_registration_queue: Sender<Adder<wgpu::RenderPipeline>>,
}

impl ShaderLoader {
    pub(super) fn new(
        shader_src_filter_adder: Sender<(PathBuf, ShaderHandle)>,
        compute_creation_queue: Sender<Adder<wgpu::ComputePipeline>>,
        render_creation_queue: Sender<Adder<wgpu::RenderPipeline>>,
    ) -> Self {
        Self {
            shader_path_whitelist_queue: shader_src_filter_adder,
            compute_constructor_registration_queue: compute_creation_queue,
            render_constructor_registration_queue: render_creation_queue,
        }
    }

    pub fn load_shader(&self, path: impl Into<PathBuf>) -> ShaderHandle {
        let path = path.into();
        let absolute_path = path
            .canonicalize()
            .expect(&format!("Unable to resolve file path {path:#?}"));

        log!(
            "Sent ",
            bright_red,
            "{absolute_path:#?}",
            bright_purple,
            " to queue",
            white
        );

        let handle = path_to_shader_id(&absolute_path);

        self.shader_path_whitelist_queue
            .send((absolute_path, handle))
            .expect("Unable to send shader to load queue");

        handle
    }

    pub fn add_render_pipeline<'a, System, Shaders, I, O, M>(
        &'a self,
        shaders: Shaders,
        system: System, // connstructor: impl IntoSystem<OutputResource, wgpu::RenderPipeline, M> + Send + Sync + 'static,
    ) -> PipelineHandle
    where
        Shaders: ShadersD<wgpu::RenderPipeline, System, I, O, M> + Send + Sync + 'static,
        System: IntoSystem<I, O, M> + Send + Sync + 'static,
    {
        let pipeline_handle = PipelineHandle { id: random() };

        // let descriptor = Shaders::into_descriptor(shaders, system);

        self.render_constructor_registration_queue
            .send(Box::new(
                move |render_manager: &mut PipelineManager<wgpu::RenderPipeline>, rr| {
                    render_manager.add_pipeline(shaders, system, pipeline_handle, rr)
                },
            ))
            .expect("Render creatation queue closed for some reason, this shouldn't happen");
        pipeline_handle
    }

    pub fn add_compute_pipeline<'a, System, Shaders, I, O, M>(
        &'a self,
        shaders: Shaders,
        system: System, // connstructor: impl IntoSystem<OutputResource, wgpu::computePipeline, M> + Send + Sync + 'static,
    ) -> PipelineHandle
    where
        Shaders: ShadersD<wgpu::ComputePipeline, System, I, O, M> + Send + Sync + 'static,
        System: IntoSystem<I, O, M> + Send + Sync + 'static,
    {
        let pipeline_handle = PipelineHandle { id: random() };

        // let descriptor = Shaders::into_descriptor(shaders, system);

        self.compute_constructor_registration_queue
            .send(Box::new(
                move |compute_manager: &mut PipelineManager<wgpu::ComputePipeline>, rr| {
                    compute_manager.add_pipeline(shaders, system, pipeline_handle, rr)
                },
            ))
            .expect("compute creatation queue closed for some reason, this shouldn't happen");
        pipeline_handle
    }
}
