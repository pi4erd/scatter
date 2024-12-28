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

struct Camera {
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> game_info: GameInfo;
@group(0) @binding(1)
var<uniform> camera: Camera;

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4(in.position, 1.0);
    out.uv = in.uv;
    return out;
}

// Fragment shader
const SKY_LIGHT: vec3<f32> = vec3<f32>(0.05, 0.05, 0.1);

fn aabb_ray(min: vec3<f32>, max: vec3<f32>, ro: vec3<f32>, rd: vec3<f32>) -> vec2<f32> {
    let tMin = (min - ro) / rd;
    let tMax = (max - ro) / rd;

    let t1 = min(tMin, tMax);
    let t2 = max(tMin, tMax);

    let tNear = max(max(t1.x, t1.y), t1.z);
    let tFar = min(min(t2.x, t2.y), t2.z);

    if (tNear > tFar || tFar < 0.0) {
        return vec2(-1.0, -1.0); // No intersection
    }

    return vec2(tNear, tFar);
}

fn sample_point(p: vec3<f32>) -> f32 {
    let amplitude = 0.01;
    let scale = 0.5;
    let sn = pow(sin(p.x * scale), 2.0);
    let cs = pow(cos(p.y * scale), 2.0);
    let scs = sin(p.z * scale);

    let spherical = 1.0 - length(p) / 5.0;

    return clamp(sn, 0.0, 1.0) * amplitude;
}

fn gather_light(p0: vec3<f32>, p1: vec3<f32>) -> f32 {
    let step_count = 128;
    let h = p1 - p0;
    let d = normalize(h);
    let step_size = length(h) / f32(step_count);

    var t = 0.0;
    var sum = 1.0;
    var steps = 0;
    while(steps < step_count) {
        let p = p0 + d * t;
        sum *= 1.0 - sample_point(p);
        t += step_size;
        steps++;
    }

    return sum;
}

fn calculate_pixel(ro: vec3<f32>, rd: vec3<f32>) -> vec3<f32> {
    let intersection = aabb_ray(vec3(-10.0), vec3(10.0), ro, rd);

    if(intersection.x == -1.0 && intersection.y == -1.0) {
        // no intersection, return sky color
        return SKY_LIGHT;
    }

    var p0 = ro + rd * intersection.x;
    if(intersection.x < 0.0) {
        p0 = ro;
    }

    let p1 = ro + rd * intersection.y;

    let light = gather_light(p0, p1);
    return light * SKY_LIGHT;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let aspect = f32(game_info.resolution.x) / f32(game_info.resolution.y);
    var uv = in.uv * 2.0 - 1.0;
    uv.x *= aspect;

    let ro = (camera.inverse_view * vec4(0.0, 0.0, 0.0, 1.0)).xyz;
    let near_p = (camera.inverse_view * vec4(vec3(uv, 1.0), 1.0)).xyz;
    let rd = normalize(near_p - ro);

    let light = calculate_pixel(ro, rd);

    return vec4(light, 1.0);
}
