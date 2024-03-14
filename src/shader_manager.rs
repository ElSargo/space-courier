use notify::{event::AccessMode, recommended_watcher, Event, RecommendedWatcher, Watcher};
use rand::{thread_rng, Rng};
use std::{
    collections::{BTreeMap, BTreeSet},
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use wgpu::{Device, RenderPipeline, ShaderModule, ShaderModuleDescriptor};

pub struct ShaderManager {
    gpu_device: Arc<Device>,
    whatcher: RecommendedWatcher,
    compile_queue_sender: UnboundedSender<(PathBuf, Option<ShaderHandle>)>,
    pipelines_requirements: BTreeMap<ShaderHandle, Vec<PipelineHandle>>,
    compiled_shaders_queue: UnboundedReceiver<(ShaderModule, ShaderHandle)>,
    pipline_reasoureces: BTreeMap<PipelineHandle, Vec<ShaderHandle>>,
    pipeline_connstructors: BTreeMap<PipelineHandle, PipelineConstructor>,
    shaders: BTreeMap<ShaderHandle, StoredShader>,
    pipelines: BTreeMap<PipelineHandle, StoredPipeline>,
    pipelines_to_init: BTreeSet<PipelineHandle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShaderHandle {
    id: u64,
}

struct StoredShader {
    module: ShaderModule,
    generation: u64,
}

struct StoredPipeline {
    render_pipeline: RenderPipeline,
    generations: Vec<u64>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Clone, Copy)]
pub struct PipelineHandle {
    id: u64,
}

type PipelineConstructor = Box<dyn FnMut(Vec<&ShaderModule>, &Device) -> RenderPipeline>;

macro_rules! log {
    ($msg:tt) => {
        println!(
            "{}{}",
            color_eyre::owo_colors::OwoColorize::bold(&color_eyre::owo_colors::OwoColorize::blue(
                &"[ShaderManager]: "
            )),
            color_eyre::owo_colors::OwoColorize::blue(&format!($msg))
        );
    };
}

impl ShaderManager {
    pub fn new(device: Arc<Device>) -> Result<Self, notify::Error> {
        let (compile_queue_sender, rcv) = unbounded_channel();
        let cqs = compile_queue_sender.clone();
        let whatcher = recommended_watcher(move |event| match event {
            Ok(Event { kind, paths, .. }) => {
                if let notify::EventKind::Access(notify::event::AccessKind::Close(
                    AccessMode::Write,
                )) = kind
                {
                    log!("fs event: {kind:#?}");
                    for path in paths {
                        cqs.send((path, None)).expect(
                            "Unable to send fs-msg from whatcher to shader loader, channel error",
                        );
                    }
                }
            }
            Err(ref e) => eprintln!("Whatcher error: {e}"),
        })?;

        let (compiled_shaders_queue_sender, compiled_shaders_queue) = unbounded_channel();

        tokio::spawn(compile_shaders(
            rcv,
            device.clone(),
            compiled_shaders_queue_sender,
        ));

        Ok(Self {
            gpu_device: device,
            whatcher,
            compile_queue_sender,
            pipelines_requirements: BTreeMap::new(),
            compiled_shaders_queue,
            pipline_reasoureces: BTreeMap::new(),
            pipeline_connstructors: BTreeMap::new(),
            shaders: BTreeMap::new(), // constructors: HashMap::new(),
            pipelines: BTreeMap::new(),
            pipelines_to_init: BTreeSet::new(),
        })
    }

    pub fn load_shader(&mut self, path: impl Into<PathBuf>) -> ShaderHandle {
        let path = path.into();
        let path = path
            .canonicalize()
            .expect(&format!("Unable to resolve file path {path:#?}"));

        self.whatcher
            .watch(&path, notify::RecursiveMode::NonRecursive)
            .expect("File whatcher failed to accept new path");

        let handle = path_to_shader_id(&path);
        log!("Sent {path:#?} to queue");
        self.compile_queue_sender
            .send((path, Some(handle)))
            .expect("Unable to send shader to load queue");

        handle
    }

    pub fn add_pipeline(
        &mut self,
        res: Vec<ShaderHandle>,
        connstructor: PipelineConstructor,
    ) -> PipelineHandle {
        let mut rng = thread_rng();
        let id: u64 = rng.gen();
        let pipeline_handle = PipelineHandle { id };
        for shader_handle in res.iter() {
            self.pipelines_requirements
                .entry(*shader_handle)
                .or_insert(vec![])
                .push(pipeline_handle)
        }
        self.pipeline_connstructors
            .insert(pipeline_handle, connstructor);
        self.pipline_reasoureces.insert(pipeline_handle, res);
        pipeline_handle
    }

    pub fn get_pipeline(&self, pipeline_handle: &PipelineHandle) -> Option<&RenderPipeline> {
        self.pipelines
            .get(pipeline_handle)
            .map(|sp| &sp.render_pipeline)
    }

    pub fn construct_piplines(&mut self) {
        while let Ok((module, shader_handle)) = self.compiled_shaders_queue.try_recv() {
            match self.shaders.get_mut(&shader_handle) {
                Some(ss) => {
                    ss.module = module;
                    ss.generation += 1;
                }
                None => {
                    self.shaders.insert(
                        shader_handle,
                        StoredShader {
                            module,
                            generation: 0,
                        },
                    );
                }
            }

            if let Some(pipelines_to_reconstruct) =
                self.pipelines_requirements.get(&shader_handle).cloned()
            {
                for pipline_handle in pipelines_to_reconstruct {
                    self.proccess_pipline(&pipline_handle);
                }
            }
            for pipeline_handle in self.pipelines_to_init.clone().iter() {
                self.proccess_pipline(&pipeline_handle);
            }
        }
    }

    fn proccess_pipline(&mut self, pipline_handle: &PipelineHandle) {
        self.pipelines_to_init.remove(pipline_handle);
        if let Some(reasources) = self.pipline_reasoureces.get(pipline_handle) {
            let mut compiled_shaders = Vec::with_capacity(reasources.len());
            for sh in reasources {
                if let Some(shader) = self.shaders.get(sh) {
                    compiled_shaders.push(shader);
                } else {
                    return;
                }
            }

            let shader_generations = compiled_shaders
                .iter()
                .map(|s| s.generation)
                .collect::<Vec<_>>();

            let should_recompile = if let Some(cached_pipeline) = self.pipelines.get(pipline_handle)
            {
                shader_generations != cached_pipeline.generations
            } else {
                true
            };

            if should_recompile {
                if let Some(constructor) = self.pipeline_connstructors.get_mut(pipline_handle) {
                    let shader_modules = compiled_shaders.iter().map(|s| &s.module).collect();
                    let new_pipeline = constructor(shader_modules, &self.gpu_device);
                    // = self
                    //     .gpu_device
                    //     .create_render_pipeline(&new_pipeline_descriptor);
                    match self.pipelines.get_mut(pipline_handle) {
                        Some(stored_pipeline) => {
                            stored_pipeline.render_pipeline = new_pipeline;
                            stored_pipeline.generations = shader_generations;
                        }
                        None => {
                            self.pipelines.insert(
                                *pipline_handle,
                                StoredPipeline {
                                    render_pipeline: new_pipeline,
                                    generations: shader_generations,
                                },
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Stable hash
fn path_to_shader_id(path: &PathBuf) -> ShaderHandle {
    use std::hash::Hasher;
    let mut hasher = seahash::SeaHasher::default();
    hasher.write(path.as_os_str().as_bytes());
    ShaderHandle {
        id: hasher.finish(),
    }
}

async fn compile_shaders(
    mut incomming: UnboundedReceiver<(PathBuf, Option<ShaderHandle>)>,
    device: Arc<Device>,
    outgoing: UnboundedSender<(ShaderModule, ShaderHandle)>,
) {
    log!("Compiling compiler running");
    while let Some((path, handle)) = incomming.recv().await {
        log!("Compiling {path:#?}");
        match tokio::fs::read_to_string(&path).await {
            Ok(source) => {
                device.push_error_scope(wgpu::ErrorFilter::Validation);

                let shader = device.create_shader_module(ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(source)),
                });

                if let Some(e) = device.pop_error_scope().await {
                    match e {
                        wgpu::Error::OutOfMemory { source } => eprintln!("{source:#?}"),
                        wgpu::Error::Validation { description, .. } => eprintln!("{description}"),
                    }
                } else {
                    log!("Compiled {path:#?}");
                    let handle = handle.unwrap_or_else(|| path_to_shader_id(&path));
                    outgoing
                        .send((shader, handle))
                        .expect("Unable to send compiled shader to queue");
                };
            }
            Err(e) => eprintln!("{e}"),
        }
    }
}
