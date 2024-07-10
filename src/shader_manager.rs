use anymap::AnyMap;

use notify::{event::AccessMode, recommended_watcher, Event, RecommendedWatcher, Watcher};
use rand::{thread_rng, Rng};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use wgpu::{ComputePipeline, Device, RenderPipeline, ShaderModule, ShaderModuleDescriptor};

pub struct ShaderManager {
    gpu_device: Arc<Device>,
    whatcher: RecommendedWatcher,
    compile_queue_sender: UnboundedSender<(PathBuf, Option<ShaderHandle>)>,
    compiled_shaders_queue: UnboundedReceiver<(ShaderModule, ShaderHandle)>,
    shaders: ShaderStorage,
    compute_manager: PipelineManager<wgpu::ComputePipeline>,
    render_manager: PipelineManager<wgpu::RenderPipeline>,
}

struct PipelineManager<T> {
    pipelines_requirements: BTreeMap<ShaderHandle, Vec<PipelineHandle>>,
    pipline_reasoureces: BTreeMap<PipelineHandle, Vec<ShaderHandle>>,
    pipeline_connstructors:
        BTreeMap<PipelineHandle, Box<dyn FnMut(&AnyMap, &Device, &ShaderStorage) -> T>>,
    pipelines: BTreeMap<PipelineHandle, StoredPipeline<T>>,
    pipelines_to_init: BTreeSet<PipelineHandle>,
}

impl<T> PipelineManager<T> {
    fn new() -> Self {
        Self {
            pipelines_requirements: BTreeMap::new(),
            pipelines_to_init: BTreeSet::new(),

            pipelines: BTreeMap::new(),

            pipeline_connstructors: BTreeMap::new(),

            pipline_reasoureces: BTreeMap::new(),
        }
    }

    pub fn add_pipeline(
        &mut self,
        res: Vec<ShaderHandle>,
        connstructor: Box<dyn FnMut(&AnyMap, &Device, &ShaderStorage) -> T>,
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

    pub fn get_pipeline(&self, pipeline_handle: &PipelineHandle) -> Option<&T> {
        self.pipelines
            .get(pipeline_handle)
            .map(|sp| &sp.render_pipeline)
    }

    fn proccess_pipline(
        &mut self,
        pipline_handle: &PipelineHandle,
        shaders: &ShaderStorage,
        map: &AnyMap,
        gpu_device: &Device,
    ) {
        self.pipelines_to_init.remove(pipline_handle);
        if let Some(reasources) = self.pipline_reasoureces.get(pipline_handle) {
            let mut compiled_shaders = Vec::with_capacity(reasources.len());
            for sh in reasources {
                if let Some(shader) = shaders.get(*sh) {
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
                    // let shader_modules = compiled_shaders.iter().map(|s| &s.module).collect();
                    let new_pipeline = constructor(&map, gpu_device, shaders);
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

    fn recompile_dependants(
        &mut self,
        shader: ShaderHandle,
        shaders: &ShaderStorage,
        gpu_device: &Device,
        map: &AnyMap,
    ) {
        if let Some(pipelines_to_reconstruct) = self.pipelines_requirements.get(&shader).cloned() {
            for pipline_handle in pipelines_to_reconstruct {
                self.proccess_pipline(&pipline_handle, &shaders, map, gpu_device);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShaderHandle {
    id: u64,
}

struct StoredShader {
    module: ShaderModule,
    generation: u64,
}

struct StoredPipeline<T> {
    render_pipeline: T,
    generations: Vec<u64>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Clone, Copy)]
pub struct PipelineHandle {
    id: u64,
}

macro_rules! log_expand {
    (  $txt:expr, $col:ident ) => {
        &color_eyre::owo_colors::OwoColorize::$col(&format!($txt))
    };
    (  $txt:expr, $col:ident, $($a:expr, $b:ident), + ) => {
        &format!("{}{}", log_expand!($txt, $col ), log_expand!($($a, $b), +))
    };
    ( $msg:expr ) => { &format!($msg) };

}

macro_rules! log {
    ($($t:tt), +) => {
        println!(
            "{}{}",
            &color_eyre::owo_colors::OwoColorize::bold(&color_eyre::owo_colors::OwoColorize::blue(
                &"[ShaderManager]: "
            )),
            log_expand!($($t), +)
        )
    }
}

pub struct ShaderStorage {
    shaders: HashMap<ShaderHandle, StoredShader>,
}
impl ShaderStorage {
    pub fn get_module<'a>(&'a self, handle: ShaderHandle) -> Option<&'a wgpu::ShaderModule> {
        self.shaders.get(&handle).map(|s| &s.module)
    }

    fn new() -> Self {
        Self {
            shaders: HashMap::new(),
        }
    }

    fn get_mut<'a>(&'a mut self, handle: ShaderHandle) -> Option<&'a mut StoredShader> {
        self.shaders.get_mut(&handle)
    }

    fn insert(&mut self, handle: ShaderHandle, item: StoredShader) {
        self.shaders.insert(handle, item);
    }

    fn get<'a>(&'a self, handle: ShaderHandle) -> Option<&'a StoredShader> {
        self.shaders.get(&handle)
    }
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
            compiled_shaders_queue,
            shaders: ShaderStorage::new(), // constructors: HashMap::new(),
            compute_manager: PipelineManager::new(),
            render_manager: PipelineManager::new(),
        })
    }

    pub fn load_shader(&self, path: impl Into<PathBuf>) -> ShaderHandle {
        let mut p: PathBuf = ".src/main.rs".into();
        p.pop();
        p.push("src/");

        let path = path.into();
        let path = path
            .canonicalize()
            .expect(&format!("Unable to resolve file path {path:#?}"));

        self.whatcher
            .watch(&path, notify::RecursiveMode::NonRecursive)
            .expect("File whatcher failed to accept new path");

        let handle = path_to_shader_id(&path);

        log!(
            "Sent ",
            bright_red,
            "{path:#?}",
            bright_purple,
            " to queue",
            white
        );
        self.compile_queue_sender
            .send((path, Some(handle)))
            .expect("Unable to send shader to load queue");

        handle
    }

    pub fn construct_piplines(&mut self, map: &AnyMap) {
        while let Ok((module, shader_handle)) = self.compiled_shaders_queue.try_recv() {
            match self.shaders.get_mut(shader_handle) {
                Some(mut ss) => {
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

            self.render_manager.recompile_dependants(
                shader_handle,
                &self.shaders,
                &self.gpu_device,
                map,
            );
            for pipeline_handle in self.render_manager.pipelines_to_init.clone().iter() {
                self.render_manager.proccess_pipline(
                    &pipeline_handle,
                    &self.shaders,
                    map,
                    &self.gpu_device,
                );
            }
            self.compute_manager.recompile_dependants(
                shader_handle,
                &self.shaders,
                &self.gpu_device,
                map,
            );
            for pipeline_handle in self.compute_manager.pipelines_to_init.clone().iter() {
                self.compute_manager.proccess_pipline(
                    &pipeline_handle,
                    &self.shaders,
                    map,
                    &self.gpu_device,
                );
            }
        }
    }

    pub fn add_render_pipeline(
        &mut self,
        res: Vec<ShaderHandle>,
        connstructor: Box<dyn FnMut(&AnyMap, &Device, &ShaderStorage) -> RenderPipeline>,
    ) -> PipelineHandle {
        self.render_manager.add_pipeline(res, connstructor)
    }

    pub fn add_compute_pipeline(
        &mut self,
        res: Vec<ShaderHandle>,
        connstructor: Box<dyn FnMut(&AnyMap, &Device, &ShaderStorage) -> ComputePipeline>,
    ) -> PipelineHandle {
        self.compute_manager.add_pipeline(res, connstructor)
    }

    pub fn get_render_pipeline(&self, pipeline_handle: &PipelineHandle) -> Option<&RenderPipeline> {
        self.render_manager.get_pipeline(pipeline_handle)
    }

    pub fn get_compute_pipeline(
        &self,
        pipeline_handle: &PipelineHandle,
    ) -> Option<&ComputePipeline> {
        self.compute_manager.get_pipeline(pipeline_handle)
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
    while let Some((path, handle)) = incomming.recv().await {
        log!("Compiling ", bright_yellow, "{path:#?}", bright_purple);
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
                    log!("Compiled ", green, "{path:#?}", bright_purple);
                    let handle = handle.unwrap_or_else(|| path_to_shader_id(&path));
                    if let Err(e) = outgoing.send((shader, handle)) {
                        eprintln!("{e:#?}");
                        panic!("Shader compile error")
                    }

                    // .expect("Unable to send compiled shaderg to queue");
                };
            }
            Err(e) => eprintln!("{e}"),
        }
    }
}

#[macro_export]
macro_rules! load_shader {
    ( $sm:expr, $path:expr) => {{
        let mut path: std::path::PathBuf = file!().into();
        path.pop();
        path.push($path);
        $sm.load_shader(path)
    }};
}
