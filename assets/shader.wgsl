struct Uniforms {
    transform: mat4x4<f32>, // matrice orthographique 2D
};

@group(0) @binding(0)
var<uniform> uniforms : Uniforms;

@group(1) @binding(0)
var my_texture: texture_2d<f32>;
@group(1) @binding(1)
var my_sampler: sampler;

struct VSOut {
    @builtin(position) Position: vec4<f32>,
    @location(0) fragUV: vec2<f32>,
};

@vertex
fn vs_main(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>) -> VSOut {
    var out: VSOut;
    out.Position = uniforms.transform * vec4<f32>(position, 0.0, 1.0);
    out.fragUV = uv;
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    return textureSample(my_texture, my_sampler, in.fragUV);
}
