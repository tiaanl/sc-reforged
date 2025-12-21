@group(0) @binding(0) var t_color: texture_2d<f32>;
@group(0) @binding(1) var oit_accumulation: texture_2d<f32>;
@group(0) @binding(2) var oit_revealage: texture_2d<f32>;

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    let clip_position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);

    return clip_position;
}

@fragment
fn fragment(@builtin(position) clip_position: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = textureDimensions(t_color, 0);
    let x = clamp(i32(clip_position.x), 0, i32(dims.x) - 1);
    let y = clamp(i32(clip_position.y), 0, i32(dims.y) - 1);
    let pixel = vec2<i32>(x, y);

    let base_color = textureLoad(t_color, pixel, 0);

    // OIT resolve inputs
    let accum = textureLoad(oit_accumulation, pixel, 0);   // rgb=sum(color*alpha), a=sum(alpha)
    let reveal = clamp(textureLoad(oit_revealage, pixel, 0).r, 0.0, 1.0); // Π(1 - alpha)

    // Reconstruct average translucent color and its effective alpha
    let epsilon = 1e-6;
    let translucent_alpha = 1.0 - reveal;
    let translucent_rgb = accum.rgb / max(accum.a, epsilon);

    // Composite translucent over base using premultiplied-style mix:
    // final = base * reveal + translucent * translucent_alpha
    let final_rgb = base_color.rgb * reveal + translucent_rgb * translucent_alpha;

    // Present as opaque (typical for swapchain). If you need downstream alpha, use
    // `translucent_alpha` or combine with base alpha as appropriate for your pipeline.
    return vec4<f32>(final_rgb, 1.0);
}
