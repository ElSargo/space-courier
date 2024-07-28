@group(0) @binding(0) var dst: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(1) var<uniform> time: f32;
 
const PI: f32 = 3.1415926535897932384626433832795;

struct Face {
    forward: vec3<f32>,
    up: vec3<f32>,
    right: vec3<f32>,
}


fn pow2(x: f32) -> f32 {
    return x * x;
}

fn zero_at(a: f32, b: f32) -> f32 {
    return a / (a - b);
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
    for (var i = 0; i < 6; i += 1) {
        t += a * noise3(s);
        s *= 2.0;
        a *= 0.5;
    }
    return t;
}


// License: WTFPL, author: sam hocevar, found: https://stackoverflow.com/a/17897228/418488
const hsv2rgb_K: vec4f = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
fn hsv2rgb(c: vec3f) -> vec3f {
    let p = abs(fract(c.xxx + hsv2rgb_K.xyz) * 6.0 - hsv2rgb_K.www);
    return c.z * mix(hsv2rgb_K.xxx, clamp(p - hsv2rgb_K.xxx, vec3(0.0), vec3(1.0)), c.y);
}
// From https://iquilezles.org/articles/intersectors/
fn sphIntersect(ro: vec3f, rd: vec3f, ce: vec3f, ra: f32) -> vec2f {
    let oc = ro - ce;
    let b = dot(oc, rd);
    let c = dot(oc, oc) - ra * ra;
    var h = b * b - c;
    if h < 0.0 {return vec2(-1.0);}; // no intersection
    h = sqrt(h);
    return vec2(-b - h, -b + h);
}

fn warp(ro: vec3f, l_ro: f32, rd: vec3f, dt: f32) -> vec3f {
    let delta_rd = -ro / pow2(l_ro) * dt;
    return normalize(rd + delta_rd);
}

fn oklch_to_linear_rbg(lch: vec3f) -> vec3f {
    let hcl = lch.zyx * vec3(1.0, 0.33, 1.0);

    let lab = vec3(
        hcl.z,
        hcl.y * cos(hcl.x * PI * 2.0),
        hcl.y * sin(hcl.x * PI * 2.0)
    );

    var lms = vec3(
        lab.x + 0.3963377774f * lab.y + 0.2158037573f * lab.z,
        lab.x - 0.1055613458f * lab.y - 0.0638541728f * lab.z,
        lab.x - 0.0894841775f * lab.y - 1.2914855480f * lab.z
    );

    // lms = pow(max(lms, vec3(0.0)), vec3(3.0));
    lms = pow(lms, vec3(3.0));

    let rgb = vec3(
        4.0767416621f * lms.x - 3.3077115913f * lms.y + 0.2309699292f * lms.z,
        -1.2684380046f * lms.x + 2.6097574011f * lms.y - 0.3413193965f * lms.z,
        -0.0041960863f * lms.x - 0.7034186147f * lms.y + 1.7076147010f * lms.z
    );

    return rgb;
}


fn march_acretion_disk(ro: vec3f, rd: vec3f, thickness: f32) -> vec4f {


    let a = atan2(ro.x, ro.z) + 2.0 * acos(-1.0);
    let l = length(ro);

    let ring = modf(l * 20.0 ).fract - 0.5;
    let strip = modf(a * 20.0).fract - 0.5;

    let ang = PI / 6.0 ;
    let period = 20.0 * (1.00);
    
    let triangle_a = modf(100.0 + (sin(ang) * ro.x + cos(ang) * ro.z) * period  ).fract - 0.5;
    let triangle_b = modf(100.0 + (sin(-ang) * ro.x + cos(-ang) * ro.z) * period ).fract - 0.5;

    


    let light: f32 = max(max(
        smoothstep(0.04, 0.03, abs(triangle_a)),
        smoothstep(0.04, 0.03, abs(triangle_b))),
        // 0.0
        smoothstep(0.06, 0.04, abs(modf(100.0 + ro.x*20.0).fract)),
        
    );

    let blend = (l - start) / (end - start);

    let color = oklch_to_linear_rbg(vec3(0.75 / length(ro), 0.7, mix(0.0, -0.1, blend)));

    return vec4(mix( color ,  oklch_to_linear_rbg(vec3(1.0,1.8,0.7)), light), 1.0);
    // + 
    // stars(ro, rd * vec3(1.0, -1.0, 1.0)), 0.0);

    
    // let brightness = nfbm(20.0*vec3(a,l,0.0)+time*vec3(1.0,0.0,0.01)   );
    // return vec4(
    //     brightness*brightness*vec3(1.0,0.8,0.2) * 10.0,
    //     1.0
    // );
    
    // let dab = vec2(1.0, -1.0) * thickness / rd.y;
    // let a = min(dab.x, dab.y);
    // let b = max(dab.x, dab.y);
    // let delta = (b - a) / 10.0;
    // // let delta = 1.0;
    // var transmission = vec3(1.0);
    // var acc = vec3(1.0);
    // var emission = vec3(0.0);
    // let dt = 0.01;
    // for (var t = 0.0; t < thickness; t += thickness*dt ) {
    //     let p = ro + rd * t;
    //     let lp = length(p.xz);

    //     let dd = 1.0 - exp(-2.0 * pow2(mid - lp) );


    //     if start < lp && end > lp {

    //     // 
    //         let a = atan2(p.x, p.z);
    //         let scale = 20.0;
    //         // let d = max(0.0, -0.2 + nfbm(p * 10.0) - dd*0.0 - 000.0 * pow2(p.y)  ) * 10.0;
    //         // let d = smoothstep(0.2,0.0,p.y - nfbm(p*10.0)*0.15);
    //         let d = max( 0.0, -0.1 + exp( -10.0*( abs(p.y - nfbm(p*10.0)*0.15) ) - 0.0   ));
    //         let col_a = vec3(0.9, 0.6, 0.15);
    //         let col_b = vec3(0.8, 0.3, 0.1);
        
    // // (10.15 / (0.1 + lp*lp ))  // vec3(500.5,30.3,6.0)

    //         emission += d * dt * transmission * (1.0 - acc) * vec3(100.0,10.0,1.0);
            
    //         // emission += (1.0 - acc) * transmission  * d * dt * 2.0  ;
    //         transmission *= exp(-d*vec3(1.0,6.0,8.0)*dt * 2.1);
    //         acc *= exp(-d*vec3(0.75,4.0,8.0)*1.00*dt);
    //     }
    // }
    // return vec4(emission, length(transmission));
    // return vec4((0.0 + 55.0 * nfbm(ro*40.0))*vec3(1.0,0.7,0.1) / pow2(length(ro*3.0)),1.0);
}


fn stars(ro: vec3f, rd: vec3f) -> vec3f {
    var color = vec3(0.0);
    for (var s = 40.0; s < 1500.0; s *= 1.1) {
        let fl = rd * s + vec3(3.31231, 18.7872, -2.22134);
        let ori = round(fl);
        var di = 10e10;
        var dcol = vec3(0.0);
        let brightness = abs(rd.y) * 0.15 + 0.25 * nfbm(rd * 4.0) + 0.75;
        for (var x = 0.0; x < 1.5; x += 1.0) {
            for (var y = 0.0; y < 1.5; y += 1.0) {
                for (var z = 0.0; z < 1.5; z += 1.0) {
                    let disp = vec3(x, y, z);
                    let o = ori + disp;
                    let threshold = hash13(o);
                    if threshold < brightness { continue; }
                    let dl = hash33(o) * - 0.5;
                    let nd = distance(o + dl, fl);
                    if nd < di {
                        di = nd;
                        dcol = vec3(0.8, 0.7, 0.4) + vec3(0.2, 0.3, 0.6) * hash33(o + vec3(3.31231, 18.7872, -2.22134)) ;
                    }
                }
            }
        }
        color += 5.0 * vec3(smoothstep(0.1, 0.0, di)) * 0.3 * dcol ;
    }
    return color * abs(rd);
    // return abs(cos(rd*50.0));
}

    
const bh_rad = 0.2;
const start: f32 = 0.25;
const height: f32 = 0.015;
const end: f32 = 1.5;
const mid: f32 = (start + end) * 0.5;


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
        Face(
            vec3(-1.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            vec3(0.0, 0.0, 1.0),
        ),
        // FACES +Y
        Face(
            vec3(0.0, -1.0, 0.0),
            vec3(0.0, 0.0, 1.0),
            vec3(1.0, 0.0, 0.0),
        ),
        // FACES -Y
        Face(
            vec3(0.0, 1.0, 0.0),
            vec3(0.0, 0.0, -1.0),
            vec3(1.0, 0.0, 0.0),
        ),
        // FACES +Z
        Face(
            vec3(0.0, 0.0, 1.0),
            vec3(0.0, 1.0, 0.0),
            vec3(1.0, 0.0, 0.0),
        ),
        // FACES -Z
        Face(
            vec3(0.0, 0.0, -1.0),
            vec3(0.0, 1.0, 0.0),
            vec3(-1.0, 0.0, 0.0),
        ),
    );

    // Get texture coords relative to cubemap face
    let dst_dimensions = vec2<f32>(textureDimensions(dst));
    let cube_uv = vec2<f32>(gid.xy) / dst_dimensions * 2.0 - 1.0;
    let d_cube_uv = vec2<f32>(gid.xy + vec2(1u,1u)) / dst_dimensions * 2.0 - 1.0;

    // Get spherical coordinate from cube_uv
    let face = FACES[gid.z];
    let spherical = normalize(face.forward + face.right * cube_uv.x + face.up * cube_uv.y);
    let d_spherical = normalize(face.forward + face.right * d_cube_uv.x + face.up * d_cube_uv.y);

    let drd = d_spherical - spherical;

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



    let ro = vec3(0.9, -height, 0.0);
    let to_center = -normalize(ro);
    let tcdrd = dot(to_center, rd);
    var col = vec3(0.0);

    // let black_hole_intersection = sphIntersect(ro, rd, vec3(0.0), bh_rad);
    // col = mix(col, vec3(20.0),
    //     exp(-pow((tcdrd - 0.799) * 5000.0, 2.0)));


    let thickness = 0.1;
    var emsission = vec3(0.0);
    var transmission = 1.0;

        {
        let dt = 0.05;

        var d = rd;
        var prev_y = ro.y;
        var prev_l = length(ro);
        var black_hole_suck = 1.0;
        var photon_shere_light = 0.0;
        var t = 0.0;
        var p = ro;
        for (var i = 0; i < 1300; i++) {
            t += dt;
            let pd = d;
            let pp = p;
            p = ro + d * t;

            let length_p = length(p);

            // let ph_r = bh_rad + 0.05;
            // if ( length_p - ph_r) * (prev_l - ph_r) < 0.0 {
            // let bf = zero_at(prev_l,length_p);
            // let ps_d = mix(prev_l,length_p,bf);
            // photon_shere_light += exp(-1.0 * pow2(ps_d)) * 0.;
            // }

            if length_p < bh_rad {black_hole_suck = 0.0; break;}
            d = warp(p, length_p, d, dt * 0.6);
            // p += d * dt;
            // let p_disp = p.y - nfbm(p * 10.0)*0.15 + 0.10;
            // let pp_disp = pp.y - nfbm(pp * 10.0)*0.15 + 0.10;
            if (p.y * pp.y) < 0.0 {
                let blend = zero_at(pp.y, p.y);
                let sp = mix(pp, p, blend);
                let sd = mix(pd, d, blend);
                let lp = length(sp.xz);
                if lp > start && lp < end {
                    let gas = 
                    // vec4(drd * t * 100.0,1.0);
                    march_acretion_disk(sp, sd, thickness);
                    emsission += gas.rgb * transmission;
                    transmission *= gas.a;
                }
            }

            prev_y = p.y;
            prev_l = length_p;
        }

        col += stars(ro + rd * t, d) * black_hole_suck;
        col = col * transmission + emsission;
        // col += photon_shere_light;
    }
    

    // col *= abs(rd);
    // let color = c + 0.01;
    // let color = max(0.0, dot(rd, vec3(1.0,0.0,0.0))) * vec3(1.0);

    // let col = mix(vec3(2.6,0.4,0.123), vec3(0.02,0.23,0.3), exp(-m*0.5)) * sin(m*100.0) * 10.0 * 1.;

    textureStore(dst, gid.xy, gid.z, vec4(col, 1.0));
}
