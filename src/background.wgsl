@group(0)
@binding(0)
var dst: texture_storage_2d_array<rgba16float, write>;
 
const PI: f32 = 3.1415926535897932384626433832795;

struct Face {
    forward: vec3<f32>,
    up: vec3<f32>,
    right: vec3<f32>,
}

fn pcg(n: u32) -> u32 {
    var h = n * 747796405u + 2891336453u;
    h = ((h >> ((h >> 28u) + 4u)) ^ h) * 277803737u;
    return (h >> 22u) ^ h;
}

fn pcg3d(p: vec3u) -> vec3u {
    var v = p * 1664525u + 1013904223u;
    v.x += v.y*v.z; v.y += v.z*v.x; v.z += v.x*v.y;
    v ^= v >> vec3u(16u);
    v.x += v.y*v.z; v.y += v.z*v.x; v.z += v.x*v.y;
    return v;
}

fn hash13(q: vec3<u32> ) -> f32 {
    var j = q;
	j *= vec3(1597334673u,3812015801u, 2798796415u);
	let n: u32 = (j.x ^ j.y ^ j.z) * 1597334673u;
	return f32(n) * 2.328306437080797e-10;
}


fn hash33(p: vec3<f32>) -> vec3<f32> {
	var q: vec3<u32> = vec3<u32>(vec3<i32>(p)) * vec3<u32>(1597334673u, 3812015801u, 2798796415u);
	q = (q.x ^ q.y ^ q.z)*vec3<u32>(1597334673u, 3812015801u, 2798796415u);
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
    for (var i=0;i<10;i += 1) {
        t += a*noise3(s);
        s *= 2.0;
        a *= 0.5;
    }
    return t;
}


fn sph(p :vec3<f32> ) -> f32 {
    let base = floor(p) + 0.5;
    return distance(p,base) - 0.005;
}

@compute
@workgroup_size(16, 16, 1)
fn full(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
) {
    // If texture size is not divisible by 32, we
    // need to make sure we don't try to write to
    // pixels that don't exist.
    if gid.x >= u32(textureDimensions(dst).x) {
        return;
    }

    var FACES: array<Face, 6> = array(
        // FACES +X
        Face(
            vec3(1.0, 0.0, 0.0),  // forward
            vec3(0.0, 1.0, 0.0),  // up
            vec3(0.0, 0.0, -1.0), // right
        ),
        // FACES -X
        Face (
            vec3(-1.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            vec3(0.0, 0.0, 1.0),
        ),
        // FACES +Y
        Face (
            vec3(0.0, -1.0, 0.0),
            vec3(0.0, 0.0, 1.0),
            vec3(1.0, 0.0, 0.0),
        ),
        // FACES -Y
        Face (
            vec3(0.0, 1.0, 0.0),
            vec3(0.0, 0.0, -1.0),
            vec3(1.0, 0.0, 0.0),
        ),
        // FACES +Z
        Face (
            vec3(0.0, 0.0, 1.0),
            vec3(0.0, 1.0, 0.0),
            vec3(1.0, 0.0, 0.0),
        ),
        // FACES -Z
        Face (
            vec3(0.0, 0.0, -1.0),
            vec3(0.0, 1.0, 0.0),
            vec3(-1.0, 0.0, 0.0),
        ),
    );

    // Get texture coords relative to cubemap face
    let dst_dimensions = vec2<f32>(textureDimensions(dst));
    let cube_uv = vec2<f32>(gid.xy) / dst_dimensions * 2.0 - 1.0;

    // Get spherical coordinate from cube_uv
    let face = FACES[gid.z];
    let spherical = normalize(face.forward + face.right * cube_uv.x + face.up * cube_uv.y);

    // // Get coordinate on the equirectangular texture
    // let inv_atan = vec2(0.1591, 0.3183);
    // let eq_uv = vec2(atan2(spherical.z, spherical.x), asin(spherical.y)) * inv_atan + 0.5;
    // let eq_pixel = vec2<i32>(eq_uv * vec2<f32>(textureDimensions(src)));

    // We use textureLoad() as textureSample() is not allowed in compute shaders
    // var sample = textureLoad(src, eq_pixel, 0);

    let rd = (spherical);

    // var l = 0.0;
    // for (var t=12.0;t<200.0;t *= 2.0 ) {
    //     let p = rd * t;
    //     l += noise3(rd) / t;
    // }
    // let c = pow(abs(l - 0.75),4.0) * vec3(0.2,0.5,3.0) * noise3(rd*10.0)  ;
    

    
        var col = vec3(0.0);
    
    for (var s = 40.0; s < 500.0;s *= 1.3) {
        
        let fl = rd * s + vec3(3.31231, 18.7872, -2.22134);
        let ori = round(fl);
        var di = 10e10;
        var dcol = vec3(0.0);
        for (var x=0.0;x<1.5;x+=1.0){
        for (var y=0.0;y<1.5;y+=1.0){
        for (var z=0.0;z<1.5;z+=1.0){
            let disp = vec3(x,y,z);
            let o = ori + disp;
            let dl = hash33(o)* - 0.5;
            let nd = distance(o+dl,fl);
            if nd < di {
                di = nd;
                dcol = hash33(o + vec3(3.31231, 18.7872, -2.22134));
            }
        }}}
        col += vec3(smoothstep(0.1   ,0.0   ,di))*10.0*dcol;

    }

    var c = 0.0;
    var d = 0.0;
    var m = 0.0;
    var mt = 0.0;
    for (var t=0.0;t<10.;t+=0.5) {
        let d = max(0.0, nfbm(rd * t ) - 0.75 );
        mt += d;
        m +=  d*d * 1.0 * exp(-mt*0.1);
    }
    
    // let color = c + 0.01;
    // let color = max(0.0, dot(rd, vec3(1.0,0.0,0.0))) * vec3(1.0);

    // let col = mix(vec3(2.6,0.4,0.123), vec3(0.02,0.23,0.3), exp(-m*0.5)) * sin(m*100.0) * 10.0 * 1.;

    textureStore(dst, gid.xy, gid.z, vec4( col , 1.0));
}
