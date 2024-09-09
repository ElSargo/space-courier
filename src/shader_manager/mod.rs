use anymap::AnyMap;
mod pipeline_shader_resource;
use notify::{
    event::{AccessMode, MetadataKind},
    recommended_watcher, Event, RecommendedWatcher, RecursiveMode, Watcher,
};
use rand::{thread_rng, Rng};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    Mutex,
};
use wgpu::{ComputePipeline, Device, RenderPipeline, ShaderModule, ShaderModuleDescriptor};

pub use self::pipeline_shader_resource::{
    PipelineConstructor, PipelineModuleDescriptor, SplitPipelineModuleDescriptor,
    SplitPipelineModuleResource, UnifiedPipelineModuleDescriptor, UnifiedPipelineModuleResorce,
};

pub struct ShaderManager {
    gpu_device: Arc<Device>,
    whatcher: Arc<tokio::sync::Mutex<RecommendedWatcher>>,
    compile_queue_sender: UnboundedSender<(PathBuf, ShaderHandle)>,
    compiled_shaders_queue: UnboundedReceiver<(ShaderModule, ShaderHandle)>,
    shaders: ShaderStorage,
    compute_manager: PipelineManager<wgpu::ComputePipeline>,
    render_manager: PipelineManager<wgpu::RenderPipeline>,
}

struct PipelineManager<PipelineType> {
    pipelines_requirements: BTreeMap<ShaderHandle, Vec<PipelineHandle>>,
    pipline_reasoureces: BTreeMap<PipelineHandle, Vec<ShaderHandle>>,
    pipeline_connstructors: BTreeMap<PipelineHandle, Box<dyn PipelineConstructor<PipelineType>>>,
    pipelines: BTreeMap<PipelineHandle, StoredPipeline<PipelineType>>,
    pipelines_to_init: BTreeSet<PipelineHandle>,
}

impl<PipelineType> PipelineManager<PipelineType> {
    fn new() -> Self {
        Self {
            pipelines_requirements: BTreeMap::new(),
            pipelines_to_init: BTreeSet::new(),

            pipelines: BTreeMap::new(),

            pipeline_connstructors: BTreeMap::new(),

            pipline_reasoureces: BTreeMap::new(),
        }
    }

    pub fn add_pipeline<const N: usize, R: PipelineModuleDescriptor<N, PipelineType>>(
        &mut self,
        res: R,
        connstructor: Box<dyn for<'a> Fn(&AnyMap, &Device, R::OutputReasorce<'a>) -> PipelineType>,
    ) -> PipelineHandle
    where
        <R as PipelineModuleDescriptor<N, PipelineType>>::Constructor: 'static,
    {
        let mut rng = thread_rng();
        let id: u64 = rng.gen();
        let pipeline_handle = PipelineHandle { id };
        for shader_handle in res.as_arr() {
            self.pipelines_requirements
                .entry(shader_handle)
                .or_insert(vec![])
                .push(pipeline_handle)
        }
        self.pipeline_connstructors
            .insert(pipeline_handle, R::constructor(connstructor));
        self.pipline_reasoureces
            .insert(pipeline_handle, res.as_arr().to_vec());
        pipeline_handle
    }

    pub fn get_pipeline(&self, pipeline_handle: &PipelineHandle) -> Option<&PipelineType> {
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
                    let new_pipeline = constructor.construct(
                        compiled_shaders
                            .iter()
                            .map(|s| &s.module)
                            .collect::<Vec<_>>()
                            .as_slice(),
                        &map,
                        gpu_device,
                    );
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
        let mut compile_time_map = HashMap::new();

        let (re_whatch_send, re_whatch_rcv) = unbounded_channel();

        let whatcher = Arc::new(Mutex::new(recommended_watcher(move |event| {
            println!("Recived event: {:#?}", event);
            match event {
                Ok(Event { kind, paths, .. }) if fs_event_is_relevent(kind) => {
                    send_paths(paths, &mut compile_time_map, &cqs);
                }
                Ok(Event {
                    kind: notify::event::EventKind::Remove(_),
                    paths,
                    ..
                }) => {
                    re_whatch_send.send(paths).unwrap();
                }
                Err(ref e) => eprintln!("Whatcher error: {e}"),
                _ => {}
            }
        })?));
        tokio::spawn(re_whatch_paths(re_whatch_rcv, whatcher.clone()));

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

    pub fn load_shader(&mut self, path: impl Into<PathBuf>) -> ShaderHandle {
        let mut p: PathBuf = ".src/main.rs".into();
        p.pop();
        p.push("src/");

        let path = path.into();
        let path = path
            .canonicalize()
            .expect(&format!("Unable to resolve file path {path:#?}"));

        log!(
            "Sent ",
            bright_red,
            "{path:#?}",
            bright_purple,
            " to queue",
            white
        );

        let handle = path_to_shader_id(&path);

        self.compile_queue_sender
            .send((path.clone(), handle))
            .expect("Unable to send shader to load queue");

        let w = self.whatcher.clone();
        tokio::spawn(async move {
            w.lock()
                .await
                .watch(&path, notify::RecursiveMode::NonRecursive)
                .expect("File whatcher failed to accept new path");
        });

        handle
    }

    pub fn construct_piplines(&mut self, map: &AnyMap) {
        while let Ok((module, shader_handle)) = self.compiled_shaders_queue.try_recv() {
            match self.shaders.get_mut(shader_handle) {
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

    pub fn add_render_pipeline<
        const NUM_MODULES: usize,
        R: PipelineModuleDescriptor<NUM_MODULES, wgpu::RenderPipeline>,
    >(
        &mut self,
        res: R,
        connstructor: Box<
            dyn for<'a> Fn(&AnyMap, &Device, R::OutputReasorce<'a>) -> RenderPipeline,
        >,
    ) -> PipelineHandle
    where
        <R as PipelineModuleDescriptor<NUM_MODULES, wgpu::RenderPipeline>>::Constructor: 'static,
    {
        self.render_manager.add_pipeline(res, connstructor)
    }
    pub fn add_compute_pipeline<
        const NUM_MODULES: usize,
        R: PipelineModuleDescriptor<NUM_MODULES, wgpu::ComputePipeline>,
    >(
        &mut self,
        res: R,
        connstructor: Box<
            dyn for<'a> Fn(&AnyMap, &Device, R::OutputReasorce<'a>) -> ComputePipeline,
        >,
    ) -> PipelineHandle
    where
        <R as PipelineModuleDescriptor<NUM_MODULES, wgpu::ComputePipeline>>::Constructor: 'static,
    {
        self.compute_manager.add_pipeline(res, connstructor)
    }

    // pub fn add_compute_pipeline(
    //     &mut self,
    //     res: Vec<ShaderHandle>,
    //     connstructor: Box<dyn FnMut(&AnyMap, &Device, &ShaderStorage) -> ComputePipeline>,
    // ) -> PipelineHandle {
    //     self.compute_manager.add_pipeline(res, connstructor)
    // }

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

async fn re_whatch_paths(
    mut re_whatch_rcv: UnboundedReceiver<Vec<PathBuf>>,
    whatcher: Arc<Mutex<notify::INotifyWatcher>>,
) {
    while let Some(paths) = re_whatch_rcv.recv().await {
        {
            let mut whatcher = whatcher.lock().await;
            for path in paths {
                whatcher.watch(&path, RecursiveMode::NonRecursive).unwrap();
            }
        }
    }
}

fn send_paths(
    paths: Vec<PathBuf>,
    compile_time_map: &mut HashMap<ShaderHandle, std::time::Instant>,
    cqs: &UnboundedSender<(PathBuf, ShaderHandle)>,
) {
    let recived_on = std::time::Instant::now();
    for path in paths {
        println!("Sending: {:#?}", path);

        let id = path_to_shader_id(&path);

        if let Some(false) = compile_time_map
            .get(&id)
            .map(|last_time| recived_on.duration_since(*last_time).as_millis() > 100)
        {
            println!("Skipping {:#?}", path);
            continue;
        }

        compile_time_map.insert(id, recived_on);

        cqs.send((path, id))
            .expect("Unable to send fs-msg from whatcher to shader loader, channel error");
    }
}

fn fs_event_is_relevent(kind: notify::EventKind) -> bool {
    matches!(
        kind,
        notify::EventKind::Access(notify::event::AccessKind::Close(AccessMode::Write))
            | notify::EventKind::Modify(
                notify::event::ModifyKind::Data(notify::event::DataChange::Any)
                    | notify::event::ModifyKind::Metadata(MetadataKind::WriteTime)
                    | notify::event::ModifyKind::Name(notify::event::RenameMode::From),
            )
    )
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
    mut incomming: UnboundedReceiver<(PathBuf, ShaderHandle)>,
    device: Arc<Device>,
    outgoing: UnboundedSender<(ShaderModule, ShaderHandle)>,
) {
    while let Some((path, handle)) = incomming.recv().await {
        log!("Compiling ", bright_yellow, "{path:#?}", bright_purple);
        let (outgoing, device) = (outgoing.clone(), device.clone());
        // handle.map(| | )
        tokio::task::spawn(compile_shader(path, device, handle, outgoing));
    }
}

async fn compile_shader(
    path: PathBuf,
    device: Arc<Device>,
    handle: ShaderHandle,
    outgoing: UnboundedSender<(ShaderModule, ShaderHandle)>,
) {
    match tokio::fs::read_to_string(&path).await {
        Ok(source) => {
            let shader;
            {
                // Catch compile error scope
                device.push_error_scope(wgpu::ErrorFilter::Validation);

                shader = device.create_shader_module(ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(source)),
                });

                if let Some(e) = device.pop_error_scope().await {
                    print_complile_error(e);
                    return;
                }
            }

            log!("Compiled ", green, "{path:#?}", bright_purple);
            outgoing.send((shader, handle)).unwrap();
        }
        Err(e) => eprintln!("{e}"),
    }
}

fn print_complile_error(e: wgpu::Error) {
    match e {
        wgpu::Error::OutOfMemory { source } => eprintln!("{source:#?}"),
        wgpu::Error::Validation { description, .. } => {
            eprintln!("{description}")
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
