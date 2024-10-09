@group(0) 
@binding(0)  
var<storage,read_write> vertex_data: array<f32>;
var<push_constant> size: u32;

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
    for (var i = 0; i < 6; i += 1) {
        t += a * noise3(s);
        s *= 2.0;
        a *= 0.5;
    }
    return t;
}

fn height(p: vec2f) -> f32 {
    let h = nfbm(vec3(p.x * 0.1, 0.0, p.y * 0.1)) - 0.5;
    return max(h, 0.0) * 15.0   ;
}

@compute
@workgroup_size(16, 16, 1)
fn full(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
) {
    if gid.x >= size {
        return;
    }

    let output_index = size * gid.y + gid.x;

    // 3_f32.sqrt() / 2.0
    let x = f32(gid.x) + 0.5 * f32(gid.y % 2);
    let z = f32(gid.y) * sqrt(3.0) * 0.5;

    let y = height(vec2(x,z));

    vertex_data[output_index] = y;
}
