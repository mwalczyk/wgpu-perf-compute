// Vertex Shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
    @location(0) position: vec4<f32>,
    @location(1) uv: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = position;
    out.uv = uv;
    return out;
}

// Fragment Shader
struct ApplicationData {
    time: f32,
};

@group(0) @binding(0) var<uniform> application_data: ApplicationData;
@group(0) @binding(1) var volume: texture_storage_3d<rgba8sint, read>;

@fragment
fn fs_main(
    in: VertexOutput
) -> @location(0) vec4<f32> {

    var size = textureDimensions(volume);

    // Display one "slice" of the volume data
    var data = textureLoad(volume, vec3<i32>(
        i32(in.uv.x * f32(size.x)),
        i32(in.uv.y * f32(size.y)),

        // Middle "slice" hardcoded for now, just for visualization purposes
        i32(0.5 * f32(size.z)),
    ));

    // Convert 8-bit signed integer format to 8-bit u-norm
    return (vec4<f32>(data) + 128.0) / 255.0;
}
