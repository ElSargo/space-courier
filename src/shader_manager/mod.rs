mod loader;
mod pipeline_manager;
mod pipeline_shader_resource;
mod pipeline_view;
mod solid_pipeline;
mod split_pipeline;

pub use self::{
    loader::ShaderLoader,
    pipeline_shader_resource::PipelineConstructor,
    pipeline_view::{ComputePipelines, RenderPipelines},
    solid_pipeline::{SolidPipelineModuleResource, SolidPipelineModuleResourceExtras, SolidShader},
    split_pipeline::{SplitShaderResource, SplitShaders},
};
use bevy_ecs::{component::Component, world::World};
use crossbeam_channel::{unbounded, Receiver, Sender};
use notify::{
    event::{AccessMode, MetadataKind},
    recommended_watcher, Event, RecommendedWatcher, Watcher,
};
use parking_lot::Mutex;
use pipeline_manager::PipelineManager;
use pollster::FutureExt;
use std::{
    collections::HashMap,
    mem::transmute,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::Instant,
};
use wgpu::{Device, ShaderModule, ShaderModuleDescriptor};

pub struct ShaderManager {
    _file_whatcher: RecommendedWatcher,
    compiled_shaders_queue: Receiver<(ShaderModule, ShaderHandle)>,
    shaders: HashMap<ShaderHandle, Arc<ShaderModule>>,
    compute_manager: PipelineManager<wgpu::ComputePipeline>,
    render_manager: PipelineManager<wgpu::RenderPipeline>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShaderHandle {
    id: u64,
}

#[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Clone, Copy)]
pub struct PipelineHandle {
    id: u64,
}

#[macro_export]
macro_rules! log_expand {
    (  $txt:expr, $col:ident ) => {
        &color_eyre::owo_colors::OwoColorize::$col(&format!($txt))
    };
    (  $txt:expr, $col:ident, $($a:expr, $b:ident), + ) => {
        &format!("{}{}", log_expand!($txt, $col ), log_expand!($($a, $b), +))
    };
    ( $msg:expr ) => { &format!($msg) };

}

#[macro_export]
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

impl ShaderManager {
    pub fn new(device: Arc<Device>, rr: &mut World) -> Result<Self, notify::Error> {
        let (compile_queue_sender, compile_queue_rcv) = unbounded();
        let (to_filter_sender, to_filter_rcv) = unbounded();

        let (compiled_shaders_queue_sender, compiled_shaders_queue) = unbounded();

        std::thread::spawn(move || {
            compile_shaders(
                compile_queue_rcv,
                device.clone(),
                compiled_shaders_queue_sender,
            )
        });

        let (compute_manager, compute_queue_send) = PipelineManager::new();
        let (render_manager, render_queue_send) = PipelineManager::new();

        rr.insert_resource(ShaderLoader::new(
            to_filter_sender,
            compute_queue_send,
            render_queue_send,
        ));
        rr.init_resource::<RenderPipelines>();
        rr.init_resource::<ComputePipelines>();

        Ok(Self {
            _file_whatcher: begin_whatching_fs(to_filter_rcv, compile_queue_sender),
            compiled_shaders_queue,
            shaders: HashMap::new(),
            compute_manager,
            render_manager,
        })
    }

    pub fn construct_piplines(&mut self, rr: &mut World) {
        while let Ok((module, shader_handle)) = self.compiled_shaders_queue.try_recv() {
            self.shaders.insert(shader_handle, module.into());

            self.render_manager
                .process_pipelines(&self.shaders, rr, shader_handle);
            self.compute_manager
                .process_pipelines(&self.shaders, rr, shader_handle);
        }
    }
}

// Is safe since Instant boils down to a time spec which is two i64s
const UNIX_EPOCH: Instant = unsafe { transmute(0u128) };

fn begin_whatching_fs(
    to_unfilter: Receiver<(PathBuf, ShaderHandle)>,
    to_compile: Sender<(PathBuf, ShaderHandle)>,
) -> RecommendedWatcher {
    let compile_time_map = Arc::new(Mutex::new(HashMap::new()));
    let (a, b) = (to_compile.clone(), compile_time_map.clone());
    thread::spawn(move || add_filters(to_unfilter.clone(), a, b));

    let mut whatcher = recommended_watcher(create_fs_event_handler(compile_time_map, to_compile))
        .expect("Whatcher failed to spawn: can't load anything, exiting");

    whatcher
        .watch(&Path::new("./src"), notify::RecursiveMode::Recursive)
        .expect("Can't whatch paths");

    whatcher
}

fn create_fs_event_handler(
    compile_time_map: Arc<Mutex<HashMap<ShaderHandle, Instant>>>,
    to_compile: Sender<(PathBuf, ShaderHandle)>,
) -> impl Fn(Result<Event, notify::Error>) {
    move |event| {
        match event {
            Ok(Event { kind, paths, .. }) if fs_event_is_relevent(kind) => {
                send_paths(gen_path_ids(paths), &compile_time_map, &to_compile);
            }

            Err(ref e) => eprintln!("Whatcher error: {e}"),

            // Ignore everything else;
            _ => {}
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

fn gen_path_ids(paths: Vec<PathBuf>) -> Vec<(PathBuf, ShaderHandle)> {
    let paths = paths
        .into_iter()
        .flat_map(|p| {
            p.canonicalize().map(|path| {
                let id = path_to_shader_id(&path);
                (path, id)
            })
        })
        .collect();
    paths
}

fn add_filters(
    to_unfilter: Receiver<(PathBuf, ShaderHandle)>,
    to_compile: Sender<(PathBuf, ShaderHandle)>,
    compile_time_map: Arc<Mutex<HashMap<ShaderHandle, std::time::Instant>>>,
) {
    loop {
        let mut p = vec![to_unfilter.recv().unwrap()];
        while let Ok(i) = to_unfilter.try_recv() {
            p.push(i);
        }
        {
            let mut lk = compile_time_map.lock();
            for (_, id) in p.iter() {
                lk.entry(*id).or_insert(UNIX_EPOCH);
            }
        }

        send_paths(
            // Avoid allocating unsessisary vecs by only creating a new one when needed
            p,
            &compile_time_map,
            &to_compile,
        );
    }
}

fn send_paths(
    paths: Vec<(PathBuf, ShaderHandle)>,
    compile_time_map: &Mutex<HashMap<ShaderHandle, std::time::Instant>>,
    cqs: &Sender<(PathBuf, ShaderHandle)>,
) {
    let recived_on = std::time::Instant::now();
    let mut lk = compile_time_map.lock();
    for (path, id) in paths.into_iter() {
        let Some(last_time) = lk.get_mut(&id) else {
            continue;
        };

        if recived_on.duration_since(*last_time).as_millis() < 100 {
            continue;
        }
        *last_time = recived_on;
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

fn compile_shaders(
    incomming: Receiver<(PathBuf, ShaderHandle)>,
    device: Arc<Device>,
    outgoing: Sender<(ShaderModule, ShaderHandle)>,
) {
    while let Ok(id) = incomming.recv() {
        let (outgoing, device) = (outgoing.clone(), device.clone());
        std::thread::spawn(move || compile_shader(id, device, outgoing));
    }
}

fn compile_shader(
    (path, id): (PathBuf, ShaderHandle),
    device: Arc<Device>,
    outgoing: Sender<(ShaderModule, ShaderHandle)>,
) {
    log!("Compiling ", bright_yellow, "{path:#?}", bright_purple);
    match std::fs::read_to_string(&path) {
        Ok(source) => {
            let shader;
            {
                // Catch compile error scope
                device.push_error_scope(wgpu::ErrorFilter::Validation);
                shader = device.create_shader_module(ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(source)),
                });

                if let Some(e) = device.pop_error_scope().block_on() {
                    print_complile_error(e);
                    return;
                }
            }

            log!("Compiled ", green, "{path:#?}", bright_purple);
            outgoing.send((shader, id)).unwrap();
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
        wgpu::Error::Internal {
            source: _,
            description: _,
        } => todo!(),
    }
}
