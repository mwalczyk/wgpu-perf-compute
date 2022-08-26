@group(0) @binding(0) var volume: texture_storage_3d<rgba8sint, write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var size = textureDimensions(volume);

    // Compute 3D UV coords in the range [0, 1] then convert to [-128, 127] so
    // that we can store it as an 8-bit signed integer
    var uv = vec3<f32>(global_id) / vec3<f32>(size);
    var to_signed_int = vec3<i32>(
        uv * 255.0 - 128.0
    );

    var write_index = vec3<i32>(global_id);
    var write_value = vec4<i32>(to_signed_int, 1);

    textureStore(volume, write_index, write_value);

    return;
}
