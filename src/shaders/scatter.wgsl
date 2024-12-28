struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct GameInfo {
    resolution: vec2<u32>,
    time: f32,
    delta_time: f32,
};

@group(0) @binding(0)
var<uniform> game_info: GameInfo;

const eye_position: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4(in.position, 1.0);
    out.uv = in.uv;
    return out;
}

fn sample_point(p: vec3<f32>) -> f32 {
    let amplitude = 0.1;
    let scale = 0.1;
    let sn = sin(p.x * scale);
    let cs = cos(p.y * scale);
    let scs = sin(p.z * scale);

    return 1.0 - clamp(sn + cs + scs, 0.0, 1.0) * amplitude;
}

fn gather_light(ro: vec3<f32>, rd: vec3<f32>) -> f32 {
    let step_count = 16;
    let max_distance = 100.0;
    let step_size = max_distance / f32(step_count);

    var t = 0.0;
    var sum = 1.0;
    while(t <= max_distance) {
        let p = ro + rd * t;
        sum *= sample_point(p);
        t += step_size;
    }

    return sum;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = in.uv * 2.0 - 1.0;
    uv.x *= f32(game_info.resolution.x) / f32(game_info.resolution.y);

    let ro = eye_position + vec3(game_info.time * 10.0, 0.0, 0.0);
    let rd = normalize(vec3(uv, 1.0));

    let light = gather_light(ro, rd);

    return vec4(vec3(light), 1.0);
}
