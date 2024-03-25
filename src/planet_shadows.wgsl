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
    @location(9) displacment: f32,
    @location(10) planet_position: vec3<f32>,
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

fn lines_closest_points(ao: vec3<f32>, ad: vec3<f32>, bo: vec3<f32>, bd: vec3<f32>) -> vec2<f32> {
    let c = bo - ao;
    let adb = dot(ad, bd);
    let adc = dot(ad, c);
    let bdc = dot(bd, c);
    let aa = dot(ad, ad);
    let bb = dot(bd, bd);
    let aabb = aa * bb - adb * adb;
    let a = (-adb * bdc + adc * bb) / aabb;
    let b = (adb * adc - bdc * aa) / aabb;

    return vec2(a, b);
}


fn sd_line(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    var h = dot(pa, ba) / dot(ba, ba);
    h = clamp(h,0.,1.);
    return length(pa - ba * h);
}

fn cone(s: Cone, r: Ray) -> ICone {
    var out = ICone(-1.0, -1.0, false);
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
        out.near = t1 * 1.;
        out.far = t2 * 1.;
        out.hit = true;
        return ICone(t1, t2, true);
    } 
    else if !t1v && t2v {
        out.near = t2;
        out.far = 10000.0;
        out.hit = true;
        return ICone(t2, 10000.0, true);
    } 
    else if !t1v && !t2v {
        // return out;
    } 
    
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
    out.umbra_cosa = instance.umbra_cosa ;
    out.umbra_h = instance.umbra_h;
    out.shadow_dir = instance.shadow_dir;
    out.world_position = vertex_position;
    out.radius = instance.radius;
    out.length_coef = in.length_coef;
    out.displacment = in.displacment;
    out.planet_position = position;
    return out;
}

//
// Penumbra intergral aproximation
// 
// t = ykw
// p = planet radius
// w = distance from planet to sun
// s = sun radius
// a = height above view ray
// b = distance from planet
// j = da/dt
// k = db/dt
// Formula 
// integral(1 - (((p/(b + k t) - (a + j t)/(b + k t) + (a + j t)/(b + k t + w) + s/(b + k t + w)) (b + k t + w))/(2 s))^2 (3 - (2 ((p/(b + k t) - (a + j t)/(b + k t) + (a + j t)/(b + k t + w) + s/(b + k t + w)) (b + k t + w)))/(2 s))) 
// Intergal computed by wolfram
// (-(((b j + k (-a + p))^3 w^3)/(b + k t)^2) + 2 k t (k (p - s) - j w)^2 (k (p + 2 s) - j w) + (6 (b j + k (-a + p))^2 w^2 (-(k p) + j w))/(b + k t) + 6 (b j + k (-a + p)) w (k^2 (p^2 - s^2) - 2 j k p w + j^2 w^2) Log[b + k t])/(8 k^4 s^3)

fn pow2(x: f32) -> f32 {return x * x * x;}
fn pow3(x: f32) -> f32 {return x * x * x;}
fn pow4(x: f32) -> f32 {return x * x * x;}

fn penumbra_int(t: f32, p: f32, w: f32, s: f32, a: f32, b: f32, j: f32, k: f32) -> f32 {
    let beu = ((pow3(b * j + k * (-a + p)) * pow3(w)) / pow2(b + k * t));
    let teu = 2.0 * k * t * pow2(k * (p - s) - j * w) * (k * (p + 2.0 * s) - j * w);
    let geu = (6.0 * pow2(b * j + k * (-a + p)) * pow2(w) * (-(k * p) + j * w)) / (b + k * t);
    let seu = 6.0 * (b * j + k * (-a + p)) * w * (pow2(k) * (pow2(p) - pow2(s)) - 2.0 * j * k * p * w + pow2(j) * pow2(w)) * log(b + k * t);
    let neu = -beu + teu + geu + seu;
    return  neu / (8.0 * pow4(k) * pow3(s));
}



fn fog_int(ro: vec3<f32>, rd: vec3<f32>, t: f32) -> f32 {
    let a = (rd.y * ro.z - ro.y * rd.z);
    let b = (rd.y * ro.z - ro.y * rd.z);
    let c = ro.x * rd.x + ro.y * rd.y + ro.z * rd.z + rd.x * rd.x * t + rd.y * rd.y * t + rd.z * rd.z * t ;
    let d = rd.x * rd.x * (ro.y * ro.y + ro.z * ro.z) + a * a - 2.0 * ro.x * rd.x * (ro.y * rd.y + ro.z * rd.z) + ro.x * ro.x * (rd.y * rd.y + rd.z * rd.z) ;
    let e = rd.x * rd.x * (ro.y * ro.y + ro.z * ro.z) + b * b - 2.0 * ro.x * rd.x * (ro.y * rd.y + ro.z * rd.z) + ro.x * ro.x * (rd.y * rd.y + rd.z * rd.z) ;
    return atan(c / sqrt(d)) / sqrt(e);
}


fn sphSoftShadow(ro: vec3<f32>, rd: vec3<f32>, sph: vec4<f32>, k: f32) -> f32 {
    let oc = ro - sph.xyz;
    let b = dot(oc, rd);
    let c = dot(oc, oc) - sph.w * sph.w;
    let h = b * b - c;

    if b > 0.0 { return step(-0.0001, c); } else { return smoothstep(0.0, 1.0, h * k / b); }
}


fn sph_ratio(
    p: f32, // planet radius
    w: f32, // distance from planet to sun
    s: f32, // sun radius
    a: f32, // height above view ray
    q: f32// distance from planet
) -> f32 {
    let iqw = 1.0 / (q + w);
    return (p / q - a / q + a * iqw + s * iqw) / (2.0 * s * iqw);
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let screen_uv = in.clip_position.xy / vec2<f32>(textureDimensions(depth_texture).xy);
    let ro = camera.view_pos.xyz;
    let rd = normalize(in.world_position - ro);
    let depth_inv = textureSample(depth_texture, depth_sampler, screen_uv);
    let view_pos = camera.inv_proj * (vec4( screen_uv * 2.0 - 1.0, depth_inv, 1.0) );
    let depth_lin = length(view_pos.xyz / view_pos.w);

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
    // umbra.far = min(umbra.far, depth_lin);
    var penumbra = cone(
        Cone(
            pow(in.penumbra_cosa,1.),
            0.0,
            in.penumbra_origin,
            in.shadow_dir
        ),
        Ray(
            ro,
            rd
        )
    );
    var fog = 0.0;
    if umbra.hit && umbra.near < (depth_lin) {
        // fog += (fog_int(ro, rd, umbra.near) - fog_int(ro, rd, umbra.far)) * 100.0;
        // fog += ();
    }



    penumbra.far = min(penumbra.far, depth_lin);
    if penumbra.hit && penumbra.near < depth_lin {

        let cp = lines_closest_points(ro,rd,in.planet_position,-in.shadow_dir);
        let rd_mid = ro + rd * cp.x;
        let sh_mid = in.planet_position + cp.y * -in.shadow_dir;
        let p_mid = (penumbra.near + penumbra.far) * 0.5;
    
        let h_near = sd_line(ro + rd * penumbra.near, in.penumbra_origin, in.shadow_dir);
        let h_mid = sd_line(ro + rd * p_mid, in.penumbra_origin, in.shadow_dir);
        let h_far = sd_line(ro + rd * penumbra.far, in.penumbra_origin, in.shadow_dir);
        let d_near = distance(in.penumbra_origin, ro + rd * penumbra.near);
        let d_mid = distance(in.penumbra_origin, ro + rd * p_mid);
        let mid_point = ro + rd * p_mid;
        let md = distance(mid_point, in.penumbra_origin);
        let b = pow2(in.radius / md) / max(0.0, pow2(20.0 / length(mid_point)))  ;
        let k = normalize( rd_mid - sh_mid );
        let v = normalize( in.planet_position - k * 10.);
        let sina = sqrt(1.0 - in.penumbra_cosa * in.penumbra_cosa);
        let radi = pow( in.radius + sina * -cp.y, 1.0 ) / in.radius;
        let a = max(0.0,sphSoftShadow(
            rd_mid,
            -v,
            vec4(in.penumbra_origin, in.radius ),
            0.2
        ));

        // let s = (cos( 3.141 / ( a + 1.0 ) ) + 1.0  ) * 0.5;
        // -1.0
        // / (2.0 - sqrt(1.0 + pow2(in.penumbra_cosa))*cp.y)
        // * 1.1
        // * (1.0 - a)
        // * (1.0 - a ) 
        // ;
        // fog += -(1.0  -smoothstep(0.0,0.0001,a) );
        // fog = -(1.-a);


        // var a = 0.0;
        // let dt = 0.1;
        // for (var t = 0.0; t < 1.0; t += dt) {
        //     let point = ro + rd * mix(penumbra.near,penumbra.far,t);
        //     let h = sd_line(point, in.planet_position,in.planet_position +  in.shadow_dir * 10000.0);
        //     // fog += cos(h) / distance(point, in.planet_position);
        //     let q = sqrt(dot2(point) - pow2(h));
        //     a += dt * (smoothstep(1.1,0.0,sph_ratio(
        //         in.radius, // planet radius
        //         q, // distance from planet to sun
        //         20.0, // sun radius
        //         h, // height above view ray
        //         // distance(point,in.planet_position)// distance from planet
        //         length(in.planet_position)
        //     )));
        //     // a += *0.1;
        // }

        fog +=  (fog_int(ro, rd, penumbra.near) - fog_int(ro, rd, penumbra.far)) * (1.0 - a) * 10.0 / max(0.01,radi) ;
        // fog += (1.0 - a);

    }
    return vec4(fog);

}
