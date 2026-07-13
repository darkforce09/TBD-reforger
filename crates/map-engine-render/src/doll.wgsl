// T-154 arsenal doll — instanced 3D primitives, lambert + ambient. params.x > 0.5 switches
// to flat (unlit) output for the byte-exact self-check probes.

struct DollUniforms {
  mvp: mat4x4<f32>,
  // x: flat flag (self-check), yzw: reserved.
  params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: DollUniforms;

struct VsIn {
  @location(0) pos: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) m0: vec4<f32>,
  @location(3) m1: vec4<f32>,
  @location(4) m2: vec4<f32>,
  @location(5) m3: vec4<f32>,
  @location(6) color: vec4<f32>,
};

struct VsOut {
  @builtin(position) clip: vec4<f32>,
  @location(0) normal: vec3<f32>,
  @location(1) color: vec4<f32>,
};

@vertex
fn vs_doll(in: VsIn) -> VsOut {
  let model = mat4x4<f32>(in.m0, in.m1, in.m2, in.m3);
  var out: VsOut;
  out.clip = u.mvp * model * vec4<f32>(in.pos, 1.0);
  // Axis-aligned normals + diagonal scale + Z-rotation: direction is exact after normalize.
  out.normal = normalize((model * vec4<f32>(in.normal, 0.0)).xyz);
  out.color = in.color;
  return out;
}

@fragment
fn fs_doll(in: VsOut) -> @location(0) vec4<f32> {
  if (u.params.x > 0.5) {
    return in.color;
  }
  let l = normalize(vec3<f32>(0.35, 0.8, 0.55));
  let shade = 0.42 + 0.58 * max(dot(normalize(in.normal), l), 0.0);
  return vec4<f32>(in.color.rgb * shade, 1.0);
}
