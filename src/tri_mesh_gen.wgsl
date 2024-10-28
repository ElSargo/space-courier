@group(0) 
@binding(0)  
var<storage,read_write> vertex_data: array<f32>;
var<push_constant> size: Push;

struct Push {
    offset: u32,
    size: u32,
    position: vec2f
}



const s3: f32 = sqrt(3.);
const s34: f32 = sqrt(3. / 4.);
const c : f32 = sqrt(sqrt(3.) - 1.) * 2.;
const i3: f32 = 1. / 3.;
const axis: vec2f = vec2(s3 / 2., -.5); 
struct Hex {
     id: vec2<i32>,
    position: vec2f
};

fn hex_tile(p: vec2f) -> Hex {
    let t = dot(p, axis);
    let j = 1.48194195; // Floating point jank
    let s = fract((p.x + j * 2. * i3) / j) * j - j * i3;
    let l = select(2. * s, -s, s > 0.0);
    let u = round((p.x * 4. + l + 0.5) * i3 - t / s34);
    let b = vec2(0, u * 2. * s34);
    let t1 = dot(p - b, axis);
    let v = round(t1 / c);
    return Hex(
        vec2<i32>(i32(u), i32(v)),
        b + v * c * axis
    );
}

fn hash13(p: vec3f) -> f32 {
    let q = vec3<u32>(vec3<i32>(p)) * vec3<u32>(1597334673u, 3812015801u, 2798796415u);
    let n = (q.x ^ q.y ^ q.z) * 1597334673u;
    return f32(n) * 2.328306437080797e-10;
}

fn hash33(p: vec3<f32>) -> vec3<f32> {
    var q: vec3<u32> = vec3<u32>(vec3<i32>(p)) * vec3<u32>(1597334673u, 3812015801u, 2798796415u);
    q = (q.x ^ q.y ^ q.z) * vec3<u32>(1597334673u, 3812015801u, 2798796415u);
    return vec3<f32>(q) * 2.328306437080797e-10;
}
// MIT License. © Stefan Gustavson, Munrocket
//
fn mod289(x: vec4f) -> vec4f { return x - floor(x * (1. / 289.)) * 289.; }
fn perm4(x: vec4f) -> vec4f { return mod289(((x * 34.) + 1.) * x); }

fn noise3(p: vec3f) -> f32 {
    let a = floor(p);
    var d: vec3f = p - a;
    d = d * d * (3. - 2. * d);

    let b = a.xxyy + vec4f(0., 1., 0., 1.);
    let k1 = perm4(b.xyxy);
    let k2 = perm4(k1.xyxy + b.zzww);

    let c = k2 + a.zzzz;
    let k3 = perm4(c);
    let k4 = perm4(c + 1.);

    let o1 = fract(k3 * (1. / 41.));
    let o2 = fract(k4 * (1. / 41.));

    let o3 = o2 * d.z + o1 * (1. - d.z);
    let o4 = o3.yw * d.x + o3.xz * (1. - d.x);

    return o4.y * d.y + o4.x * (1. - d.y);
}

fn nfbm(p: vec3f) -> f32 {
    var s = p;
    var a = 0.5;
    var t = 0.0;
    for (var i = 0; i < 8; i += 1) {
        t += a * noise3(s);
        s *= 2.0;
        a *= 0.7;
    }
    return t;
}


fn hash12(p: vec2f) -> f32 {
    var p3 = fract(p.xyx * .1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn height(p: vec2f) -> f32 {

    let hx = hex_tile(p * 0.6);
    let h = max(nfbm(vec3(p.x * 0.005, 0.0, p.y * 0.005 + 1.0)) - 0.9, 0.0) * 100.0 ;
    let g = max(nfbm(vec3(p.x * 0.02, 0.0, p.y * 0.02)) - 0.8, 0.0) * 20.0 ;
    let k = max(nfbm(vec3(p.x * 0.2, 0.0, p.y * 0.5)) - 0.9, 0.0) * 5.0;
    let o = max(nfbm(vec3(hx.position.x * 0.2, 0.0, hx.position.y)) - 0.9, 0.0) * 5.0;
    return  (h + g) * smoothstep(4.0, 10.0, abs(p.x * 0.25 + p.y * 0.75)) ;
}

@compute
@workgroup_size(16, 16, 1)
fn full(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
) {
    if max(gid.x, gid.y) >= size.size {
        return;
    }

    let output_index = size.offset + size.size * gid.y + gid.x;

    // 3_f32.sqrt() / 2.0
    let x = f32(gid.x) + 0.5 * f32(gid.y % 2);
    let y = f32(gid.y) * sqrt(3.0) * 0.5;

    let z = height(vec2(x, y) + size.position);

    vertex_data[output_index] = z;
}
