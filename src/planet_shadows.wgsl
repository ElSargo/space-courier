@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var color_texture: texture_2d<f32>;
@group(1) @binding(1) var color_sampler: sampler;
@group(1) @binding(2) var depth_texture: texture_depth_2d;
@group(1) @binding(3) var depth_sampler: sampler;

struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
};

struct VertexInput {
    @location(0) length_coef: f32,
    @location(1) displacment: f32,
};

struct Instance {
    @location(2) penumbra_origin_length: f32,
    @location(3) penumbra_cosa: f32,
    @location(4) umbra_origin_length: f32,
    @location(5) umbra_cosa: f32,
    @location(6) umbra_h: f32,
    @location(7) shadow_dir: vec3<f32>,

    @location(8) radius: f32,
    @location(9) back_length: f32,
    @location(10) back_width: f32
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) penumbra_origin: vec3<f32>,
    @location(1) penumbra_cosa: f32,
    @location(2) umbra_origin: vec3<f32>,
    @location(3) umbra_cosa: f32,
    @location(4) umbra_h: f32,
    @location(5) shadow_dir: vec3<f32>,
    @location(6) world_position: vec3<f32>,
    @location(7) radius: f32,
    @location(8) length_coef: f32,
};

fn dot2(a: vec3<f32>) -> f32 {
    return dot(a, a);
}

struct Cone {
    cosa: f32,
    h: f32,
    c: vec3<f32>,
    v: vec3<f32>,
};

struct Ray {
  o: vec3<f32>,
  d: vec3<f32>  
};


struct ICone {
    near: f32,
    far: f32,
    hit: bool
};

fn lines_closest_points(ao: vec3<f32>, ad: vec3<f32>, bo: vec3<f32>, bd: vec3<f32> ) -> vec2<f32> {
    let c = bo - ao;
    let adb = dot(ad,bd);
    let adc = dot(ad,c);
    let bdc = dot(bd,c);
    let aa = dot(ad,ad);
    let bb = dot(bd,bd);
    let aabb = aa*bb - adb*adb;
    let a  = ( -adb*bdc + adc*bb ) / aabb;
    let b = ( adb*adc - bdc*aa ) / aabb;

    return vec2(a,b);
}


fn cone(s: Cone, r: Ray) -> ICone {
    var out = ICone(-1.0,-1.0, false);
    let co = r.o - s.c;

    let a = dot(r.d, s.v) * dot(r.d, s.v) - s.cosa * s.cosa;
    let b = 2. * (dot(r.d, s.v) * dot(co, s.v) - dot(r.d, co) * s.cosa * s.cosa);
    let c = dot(co, s.v) * dot(co, s.v) - dot(co, co) * s.cosa * s.cosa;

    var det = b * b - 4. * a * c;
    if det < 0. { return out; };

    det = sqrt(det);
    var t1 = (-b - det) / (2. * a);
    var t2 = (-b + det) / (2. * a);

    // This is a bit messy; there ought to be a more elegant solution.
    // if t1 < 0. || t2 > 0. && t2 < t1 { let t = t1; t1 = t2; t2 = t; };
    // if t1 < 0. {return vec2(-1.0); }


    if t1 > t2 {
        let t = t1;
        t1 = t2;
        t2 = t;
    }

    let cp1 = r.o + t1 * r.d - s.c;
    let cp2 = r.o + t2 * r.d - s.c;
    let  h1 = dot(cp1, s.v);
    let  h2 = dot(cp2, s.v);
    let t1v = h1 > 0.0;
    let t2v = h2 > 0.0;

    if t1v && t2v {
        return ICone(t1, t2, true);
    } else if !t1v && t2v {
        return ICone(t2, 10000.0, true);
    } else if !t1v && !t2v {
        return out;
    } 
    
    // if h < 0. || h > s.h { return vec2(-1.0); } 
    return out;
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32, in: VertexInput, instance: Instance) -> VertexOutput {
    var out: VertexOutput  ;
    let position = instance.shadow_dir * (instance.umbra_origin_length - instance.umbra_h);
    let center_line_positon = position + instance.shadow_dir * in.length_coef * instance.back_length;
    let view = normalize(position - camera.view_pos.xyz);
    let tangent = normalize(cross(instance.shadow_dir, view)) ;
    let width = mix(instance.radius, instance.back_width, in.length_coef);
    let displacement = tangent * in.displacment * width;
    let vertex_position = center_line_positon + displacement ;

    out.clip_position = camera.view_proj * vec4(vertex_position, 1.0);

    out.penumbra_origin = instance.shadow_dir * instance.penumbra_origin_length;
    out.penumbra_cosa = instance.penumbra_cosa;
    out.umbra_origin = instance.shadow_dir * instance.umbra_origin_length;
    out.umbra_cosa = instance.umbra_cosa;
    out.umbra_h = instance.umbra_h;
    out.shadow_dir = instance.shadow_dir;
    out.world_position = vertex_position;
    out.radius = instance.radius;
    out.length_coef = in.length_coef;
    return out;
}
fn fog_int(ro: vec3<f32>, rd: vec3<f32>, t: f32) -> f32 {
    let a = (rd.y * ro.z - ro.y * rd.z);
    let b = (rd.y * ro.z - ro.y * rd.z);
    let c = ro.x * rd.x + ro.y * rd.y + ro.z * rd.z + rd.x * rd.x * t + rd.y * rd.y * t + rd.z * rd.z * t ;
    let d = rd.x * rd.x * (ro.y * ro.y + ro.z * ro.z) + a * a - 2.0 * ro.x * rd.x * (ro.y * rd.y + ro.z * rd.z) + ro.x * ro.x * (rd.y * rd.y + rd.z * rd.z) ;
    let e = rd.x * rd.x * (ro.y * ro.y + ro.z * ro.z) + b * b - 2.0 * ro.x * rd.x * (ro.y * rd.y + ro.z * rd.z) + ro.x * ro.x * (rd.y * rd.y + rd.z * rd.z) ;
    return atan(c / sqrt(d)) / sqrt(e);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let screen_uv = in.clip_position.xy / vec2<f32>(textureDimensions(depth_texture).xy);
    let ro = camera.view_pos.xyz;
    let rd = normalize(in.world_position - ro);
    let depth = textureSample(depth_texture, depth_sampler, screen_uv);
    let depth_lin = 1. / ( depth - 0.1 );

    // var penumbra_cone = cone_intersecttion(ro,rd,0.9,100.0,in.penumbra_origin, in.shadow_dir);
    var umbra = cone(
        Cone(
            in.umbra_cosa,
            in.umbra_h,
            in.umbra_origin,
            -in.shadow_dir
        ),
        Ray(
            ro,
            rd
        )
    );
    umbra.far = min(umbra.far, depth_lin);
    var penumbra = cone(
        Cone(
            in.penumbra_cosa,
            0.0,
            in.penumbra_origin,
            in.shadow_dir
        ),
        Ray(
            ro,
            rd
        )
    );
    penumbra.far = min(penumbra.far, depth_lin);
    var fog = 0.0;
    if umbra.hit {
        fog += (fog_int(ro, rd, umbra.near) - fog_int(ro, rd, umbra.far)) * 10.0;
    }
    if penumbra.hit {
        fog += (fog_int(ro, rd, penumbra.near) - fog_int(ro, rd, penumbra.far)) * in.radius * in.radius * 1.0 * pow(1.0 - in.length_coef, 46.0);
    }
    return vec4(fog);
    
     // if depth_lin < umbra_cone.x || umbra_cone.x <= 0.0 || umbra_cone.y < 0.0{
     //     return vec4(0.0);
     // }

    // return vec4(umbra_cone.y-umbra_cone.x)*0.01;
    // return vec4( 1000.0 / dot(in.world_position,in.world_position));
    
    // if depth < (in.clip_position.z) {
    //     return vec4(0.25);
    // } else {
    //     return vec4(0.0);
    // }
    // return vec4(vec2(depth), 0.0,0.0);
}
