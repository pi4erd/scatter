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
const SKY_LIGHT: vec3<f32> = vec3<f32>(0.0);
const AABB_MIN: vec3<f32> = vec3<f32>(-40.0);
const AABB_MAX: vec3<f32> = vec3<f32>(40.0);
const SUN_DIRECTION: vec3<f32> = vec3<f32>(0.5773502691896258, 0.5773502691896258, 0.5773502691896258);
const PI: f32 = 3.141592653589;
const WAVELENGTHS: vec3<f32> = vec3<f32>(0.7, 0.9, 0.80);

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
    let ln = length(p);
    if(ln < 18.0) {
        return 0.0;
    }

    let h = length(p) / 20.0;

    return exp(-h / 0.35);
}

// Compute Rayleigh scattering coefficient
fn rayleighScattering(wavelength: vec3<f32>) -> vec3<f32> {
    const rayleigh_intensity: f32 = 0.1; // Adjust for intensity scaling
    return rayleigh_intensity / pow(wavelength, vec3(4.0)); // 1 / λ^4
}

// Compute Mie scattering coefficient
fn mieScattering(wavelength: vec3<f32>) -> vec3<f32> {
    const mie_intensity: f32 = 0.001; // Adjust for intensity scaling
    return mie_intensity / wavelength; // 1 / λ (approximation)
}

// Rayleigh phase function
fn rayleighPhase(cos_theta: f32) -> f32 {
    return 3.0 / 4.0 + pow(cos_theta, 2.0);
}

// Mie phase function (Henyey-Greenstein)
fn miePhase(cos_theta: f32, g: f32) -> f32 {
    let left = 3.0 * (1.0 - pow(g, 2.0)) / (2.0 * (2.0 + pow(g, 2.0)));
    let right = (1 + pow(cos_theta, 2.0)) / pow(1 + pow(g, 2.0) - 2 * g * cos_theta, 1.5);

    return left * right;
}

fn ray_sky(rd: vec3<f32>) -> vec3<f32> {
    let sun = pow(clamp(dot(rd, SUN_DIRECTION), 0.0, 1.0), 128.0);

    return mix(SKY_LIGHT, vec3<f32>(1.0), sun);
}

fn out_scattering(p0: vec3<f32>, p1: vec3<f32>) -> vec3<f32> {
    let rayleigh = rayleighScattering(WAVELENGTHS);
    let mie = mieScattering(WAVELENGTHS);

    let step_count = 8;
    let h = p1 - p0;
    let d = normalize(h);
    let step_size = length(h) / f32(step_count);

    var accumulated_scattering = vec3<f32>(0.0);
    var t = 0.0;

    var steps = 0;

    let sky = ray_sky(d);

    while(steps < step_count) {
        let p = p0 + t * d;
        let density = sample_point(p);

        accumulated_scattering += density * step_size;

        t += step_size;
        steps++;
    }

    return 4.0 * PI * rayleigh * accumulated_scattering;
}

fn in_scattering(p0: vec3<f32>, p1: vec3<f32>) -> vec3<f32> {
    let rayleigh = rayleighScattering(WAVELENGTHS);
    let mie = mieScattering(WAVELENGTHS);

    let step_count = 256;
    let h = p1 - p0;
    let d = normalize(h);
    let step_size = length(h) / f32(step_count);

    var accumulated_scattering = vec3<f32>(0.0);
    var t = 0.0;

    let cos_theta = dot(d, SUN_DIRECTION);
    let rayleigh_phase = rayleighPhase(cos_theta);

    var steps = 0;
    let sky = ray_sky(d);

    while(steps < step_count) {
        let p = p0 + t * d;
        let density = sample_point(p);

        let sun_dir_intersection = aabb_ray(AABB_MIN, AABB_MAX, p, SUN_DIRECTION);
        let cam_dir_intersection = aabb_ray(AABB_MIN, AABB_MAX, p, -d);
        let out_scatter_sun = out_scattering(p, p + SUN_DIRECTION * (sun_dir_intersection.y + 0.1));
        let out_scatter_camera = out_scattering(p, p + (-d * cam_dir_intersection.y));

        let sun_camera_scatter = exp(-out_scatter_sun);

        accumulated_scattering += density * sun_camera_scatter * step_size;

        t += step_size;
        steps++;
    }

    // Is(l) * K(l) * F(theta, g)
    return vec3(1.0) * rayleigh * accumulated_scattering;
}

fn scatter(p0: vec3<f32>, p1: vec3<f32>) -> vec3<f32> {
    return in_scattering(p0, p1);
}

fn blend_with_sky(rd: vec3<f32>, scattered: vec3<f32>) -> vec3<f32> {
    // Example: Use an exponential decay based on scattering intensity
    let scattering_factor = 1.0 - exp(-length(scattered));
    return mix(ray_sky(rd), scattered, scattering_factor);
}

fn calculate_pixel(ro: vec3<f32>, rd: vec3<f32>) -> vec3<f32> {
    let intersection = aabb_ray(AABB_MIN, AABB_MAX, ro, rd);

    if(intersection.x == -1.0 && intersection.y == -1.0) {
        // no intersection, return sky color
        return ray_sky(rd);
    }

    var p0 = ro + rd * intersection.x;
    if(intersection.x < 0.0) {
        p0 = ro; // we're inside the box
    }

    let p1 = ro + rd * intersection.y;

    let scattered = scatter(p0, p1);
    return blend_with_sky(rd, scattered);
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
