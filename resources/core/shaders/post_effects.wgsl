struct Uniform {
    depth_threshold: f32,
}

@group(0) @binding(0)
var<uniform> ubo: Uniform;

@group(0) @binding(1)
var frame_texture: texture_2d<f32>;
@group(0) @binding(2)
var frame_sampler: sampler;

@group(0) @binding(3)
var normal_texture: texture_2d<f32>;
@group(0) @binding(4)
var normal_sampler: sampler;

@group(0) @binding(5)
var depth_texture: texture_2d<f32>;
@group(0) @binding(6)
var depth_sampler: sampler;


struct VertexInput {
    @builtin(vertex_index) idx: u32,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    let uv = vec2(
         f32((in.idx << 1u) & 2u),
         f32(in.idx & 2u)
     );

    out.pos = vec4(uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2(uv.x, 1.0 - uv.y);

    return out;
}

const SOBEL_X = mat3x3<f32>(
    vec3<f32>( 1.0,  2.0,  1.0),
    vec3<f32>( 0.0,  0.0,  0.0),
    vec3<f32>(-1.0, -2.0, -1.0),
);

const SOBEL_Y = mat3x3<f32>(
    vec3<f32>( 1.0,  0.0, -1.0),
    vec3<f32>( 2.0,  0.0, -2.0),
    vec3<f32>( 1.0,  0.0, -1.0),
);

fn sobel(uv: vec2<f32>) -> f32 {
    let texel_size = 1.0 / vec2<f32>(textureDimensions(frame_texture));

    let c  = length(textureSample(frame_texture, frame_sampler, uv).rgb);
    let n  = length(textureSample(frame_texture, frame_sampler, uv + texel_size * vec2<f32>( 0.0,  1.0)).rgb);
    let e  = length(textureSample(frame_texture, frame_sampler, uv + texel_size * vec2<f32>( 1.0,  0.0)).rgb);
    let s  = length(textureSample(frame_texture, frame_sampler, uv + texel_size * vec2<f32>( 0.0, -1.0)).rgb);
    let w  = length(textureSample(frame_texture, frame_sampler, uv + texel_size * vec2<f32>(-1.0,  0.0)).rgb);
    let ne = length(textureSample(frame_texture, frame_sampler, uv + texel_size * vec2<f32>( 1.0,  1.0)).rgb);
    let nw = length(textureSample(frame_texture, frame_sampler, uv + texel_size * vec2<f32>(-1.0,  1.0)).rgb);
    let se = length(textureSample(frame_texture, frame_sampler, uv + texel_size * vec2<f32>( 1.0, -1.0)).rgb);
    let sw = length(textureSample(frame_texture, frame_sampler, uv + texel_size * vec2<f32>(-1.0, -1.0)).rgb);

    let m = mat3x3(
        vec3(sw,  s, se),
        vec3( w,  c,  e),
        vec3(nw,  n, ne),
    );

    let gx = dot(SOBEL_X[0], m[0]) + dot(SOBEL_X[1], m[1]) + dot(SOBEL_X[2], m[2]);
    let gy = dot(SOBEL_Y[0], m[0]) + dot(SOBEL_Y[1], m[1]) + dot(SOBEL_Y[2], m[2]);

    let g = length(vec2(gx, gy));

    return smoothstep(0.2, 0.8, sqrt(g));
}

fn normal_edge(uv: vec2<f32>) -> f32 {
    let texel_size = 1.0 / vec2<f32>(textureDimensions(normal_texture));

    let c  = textureSample(normal_texture, normal_sampler, uv).rgb;
    let n  = dot(c, textureSample(normal_texture, normal_sampler, uv + texel_size * vec2<f32>( 0.0,  1.0)).rgb);
    let e  = dot(c, textureSample(normal_texture, normal_sampler, uv + texel_size * vec2<f32>( 1.0,  0.0)).rgb);
    let s  = dot(c, textureSample(normal_texture, normal_sampler, uv + texel_size * vec2<f32>( 0.0, -1.0)).rgb);
    let w  = dot(c, textureSample(normal_texture, normal_sampler, uv + texel_size * vec2<f32>(-1.0,  0.0)).rgb);

    let m = mat3x3(
        vec3(1.0,   s, 1.0),
        vec3(  w, 1.0,   e),
        vec3(1.0,   n, 1.0),
    );

    let gx = dot(SOBEL_X[0], m[0]) + dot(SOBEL_X[1], m[1]) + dot(SOBEL_X[2], m[2]);
    let gy = dot(SOBEL_Y[0], m[0]) + dot(SOBEL_Y[1], m[1]) + dot(SOBEL_Y[2], m[2]);

    let g = length(vec2(gx, gy));

    return step(0.2, sqrt(g));
}

fn rgb2hsl(c: vec3<f32>) -> vec3<f32> {
    let K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
    let q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));

    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;

    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

fn hsl2rgb(c: vec3<f32>) -> vec3<f32> {
  let K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
  let p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);

  return c.z * mix(K.xxx, clamp(p - K.xxx, vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0)), c.y);
}

fn normal_edge_color(color: vec4<f32>, normal_edge_r: f32) -> vec4<f32> {
    if (normal_edge_r > 0.0) {
        var hsl = rgb2hsl(color.rgb);
        hsl.z = 0.3;

        return vec4(hsl2rgb(hsl), color.a);
    } else {
        return color;
    }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(frame_texture, frame_sampler, in.uv);

    let sobel_r = sobel(in.uv);
    let sobel_c = color * vec4(vec3(sobel_r), 0.0);

    let normal_edge_r = normal_edge(in.uv);
    let normal_edge_c = normal_edge_color(color, normal_edge_r);

    return normal_edge_c + sobel_c * 0.5;
}