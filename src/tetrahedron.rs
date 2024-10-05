// use crate::transform::Transform;
// use crate::vertex_layout;
// use crate::{
//     gpu::SolidDrawable,
//     instance_buffer,
//     shader_manager::{ShaderManagerCtrl, SolidPipelineModuleResourceExtras, SolidShader},
// };
// use bevy_ecs::system::{Commands, Res};
// use bytemuck::cast_slice;
// use glam::{vec3, Mat4, Quat, Vec3};
// use rand::{thread_rng, Rng};
// use std::{any::Any, sync::Arc};
// use wgpu::{
//     util::{BufferInitDescriptor, DeviceExt},
//     BufferUsages, IndexFormat,
// };

// use crate::{
//     ecs::Initailiser,
//     gpu::{SolidRenderable, RD},
// };

// const SQRT_3_FRAC_2: f32 = 0.8660254;
// const SQRT_3_FRAC_4: f32 = 0.4330127;
// const VERTS: [Vec3; 4] = [
//     vec3(0.0, 1.0, 0.0),
//     vec3(SQRT_3_FRAC_2, -0.5, 0.0),
//     vec3(SQRT_3_FRAC_4, -0.5, 0.75),
//     vec3(SQRT_3_FRAC_4, -0.5, -0.75),
// ];
// #[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
// pub struct Vertex {
//     position: [f32; 3],
// }
// vertex_layout!(Vertex,[0=>Float32x3] );
// #[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
// pub struct Instance {
//     mat: Mat4,
// }
// vertex_layout!(Instance,[ 1 => Float32x4  ,2 => Float32x4,  2 => Float32x4,  4 => Float32x4 ]);

// fn init(device: RD, mut commands: Commands, shader_manager: Res<ShaderManagerCtrl>) {
//     const BATCH_SIZE: usize = 100;

//     let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
//         label: Some("Stars batch vertex buffer"),
//         usage: BufferUsages::VERTEX,
//         contents: cast_slice(make_vertex_data(BATCH_SIZE).as_slice()),
//     });

//     let index_data: Vec<u16> = (0..100)
//         .flat_map(|idx| {
//             let base = idx * 4;
//             [0, 1, 2, 1, 2, 3, 0, 2, 3, 0, 1, 3]
//                 .iter()
//                 .map(move |x| x + base)
//         })
//         .collect();

//     let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
//         label: Some("Stars batch index buffer"),
//         usage: BufferUsages::VERTEX,
//         contents: cast_slice(index_data.as_slice()),
//     });

//     let pipeline = shader_manager.add_render_pipeline(
//         SolidShader {
//             handle: shader_manager.load_shader("./src/stars.wgsl"),
//         },
//         || SolidPipelineModuleResourceExtras {
//             vertex_buffers: Arc::new([Vertex::desc(), Instance::desc()]),
//             extra_bind_groups: vec![],
//         },
//     );

//     commands.spawn((
//         Transform::default(),
//         Transform::default(),
//         SolidRenderable {
//             pipeline_handle: pipeline,
//             renderable: Box::new(Stars {
//                 vertex_buffer,
//                 index_buffer,
//             }),
//         },
//         instance_buffer!(@ "Star instance buffer", &device),
//     ));
// }

// pub fn init_mod(initailiser: &mut Initailiser) {
//     initailiser.init.add_systems(init);
// }

// fn make_vertex_data(batch_size: usize) -> Vec<Vec3> {
//     let batch_bounds = Vec3::ONE * 10.0;
//     let scale_bounds = 1.0..2.0;
//     let mut rng = thread_rng();

//     let mut batch = Vec::with_capacity(batch_size * 4);
//     for _ in 0..batch_size {
//         let position = rng.gen::<Vec3>() * batch_bounds;

//         let rotation: Quat = rng.gen();
//         let scale = rng.gen_range(scale_bounds.clone());

//         for vert in VERTS {
//             batch.push(rotation.mul_vec3(vert) * scale + position);
//         }
//     }

//     batch
// }

// #[derive(Debug)]
// struct Stars {
//     vertex_buffer: wgpu::Buffer,
//     index_buffer: wgpu::Buffer,
// }

// impl SolidDrawable for Stars {
//     fn draw<'a: 'b, 'b>(
//         &'a self,
//         render_pass: &mut wgpu::RenderPass<'b>,
//         instances: std::ops::Range<u32>,
//         _asset: &'a dyn Any,
//     ) {
//         render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
//         render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
//         render_pass.draw_indexed(0..self.index_buffer.size() as u32 / 2, 0, instances);
//     }

//     fn aquire(&self) -> Option<Arc<dyn Any>> {
//         Some(Arc::new(()))
//     }
// }
