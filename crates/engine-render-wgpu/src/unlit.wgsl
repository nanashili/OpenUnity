// Original opaque unlit shader for INFRA-RENDER-001.
//
// The CPU frame-preparation path supplies finite homogeneous clip positions.
// WGPU's native clip volume performs x/y clipping against [-w,w] and z
// clipping against [0,w]. No discard, culling, blending, or depth emulation
// is performed here.

struct VertexInput {
    @location(0) clip_position: vec4<f32>,
    @location(1) rgba8: vec4<u32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) rgba8: vec4<u32>,
}

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = input.clip_position;
    output.rgba8 = input.rgba8;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.rgba8) / 255.0;
}
