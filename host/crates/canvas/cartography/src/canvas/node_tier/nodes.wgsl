// Node bodies and links, instanced. Positions come from a storage buffer by
// slot, so a link reads both endpoints from the same buffer the bodies do and
// stays attached by construction. Output is premultiplied alpha.

struct Globals {
    offset: vec2<f32>,
    viewport: vec2<f32>,
    zoom: f32,
    tilt: f32,
    sin_yaw: f32,
    cos_yaw: f32,
    scale: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
    selection: vec4<f32>,
    focus: vec4<f32>,
    pin: vec4<f32>,
}

struct Attr {
    half_side: f32,
    lift: f32,
    color: u32,
    flagword: u32,
}

struct Link {
    end_a: u32,
    end_b: u32,
    color: u32,
    width: f32,
}

@group(0) @binding(0) var<uniform> g: Globals;
@group(0) @binding(1) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> attrs: array<Attr>;
@group(0) @binding(3) var<storage, read> body_order: array<u32>;
@group(0) @binding(4) var<storage, read> links: array<Link>;

const MARGIN: f32 = 8.0;
const FLAG_VISIBLE: u32 = 1u;
const FLAG_SELECTED: u32 = 4u;
const FLAG_HOVERED: u32 = 8u;
const FLAG_FOCUSED: u32 = 16u;
const FLAG_PINNED: u32 = 32u;

// Camera::to_screen, with the node's lift (world px) raising it on screen.
fn project(p: vec2<f32>, lift: f32) -> vec2<f32> {
    let rx = p.x * g.cos_yaw - p.y * g.sin_yaw;
    let ry = p.x * g.sin_yaw + p.y * g.cos_yaw;
    return vec2<f32>(
        rx * g.zoom + g.offset.x,
        ry * g.tilt * g.zoom + g.offset.y - lift * g.zoom,
    );
}

fn to_clip(px: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(px.x / g.viewport.x * 2.0 - 1.0, 1.0 - px.y / g.viewport.y * 2.0, 0.0, 1.0);
}

fn unpack(c: u32) -> vec4<f32> {
    return vec4<f32>(
        f32(c & 255u),
        f32((c >> 8u) & 255u),
        f32((c >> 16u) & 255u),
        f32(c >> 24u),
    ) / 255.0;
}

// Pixel coverage of a signed distance in layout px (negative inside).
fn cover(sd: f32) -> f32 {
    return clamp(0.5 - sd * g.scale, 0.0, 1.0);
}

// `rgb` at alpha `a` over a premultiplied destination.
fn over(dst: vec4<f32>, rgb: vec3<f32>, a: f32) -> vec4<f32> {
    return vec4<f32>(rgb * a + dst.rgb * (1.0 - a), a + dst.a * (1.0 - a));
}

struct NodeOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) color: vec4<f32>,
    @location(2) @interpolate(flat) half_px: f32,
    @location(3) @interpolate(flat) flagword: u32,
}

@vertex
fn vs_node(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> NodeOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
    );
    let slot = body_order[ii];
    let a = attrs[slot];
    let center = project(positions[slot].xy, a.lift);
    let half_px = a.half_side * g.zoom;
    let local = corners[vi] * (half_px + MARGIN);
    var out: NodeOut;
    out.pos = to_clip(center + local);
    out.local = local;
    out.color = unpack(a.color);
    out.half_px = half_px;
    out.flagword = a.flagword;
    return out;
}

// Shape ids: 0 rounded (radius is a fraction of the half side), 1 diamond,
// 2 hexagon (the outlines `Silhouette::outline` gives, pointy-top).
fn body_distance(p: vec2<f32>, half_px: f32, shape: u32, radius_fraction: f32) -> f32 {
    if (shape == 1u) {
        return (abs(p.x) + abs(p.y) - half_px) * 0.70710678;
    }
    if (shape == 2u) {
        return max(abs(p.x) - half_px, (abs(p.y) + 0.5 * abs(p.x) - half_px) * 0.89442719);
    }
    let r = radius_fraction * half_px;
    let q = abs(p) - vec2<f32>(half_px - r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

// The band `inner..inner + width` outside the body.
fn ring(sd: f32, inner: f32, width: f32) -> f32 {
    return cover(sd - inner - width) * (1.0 - cover(sd - inner));
}

@fragment
fn fs_node(in: NodeOut) -> @location(0) vec4<f32> {
    let shape = in.flagword & 3u;
    let bits = (in.flagword >> 2u) & 255u;
    let radius_fraction = f32((in.flagword >> 16u) & 255u) / 255.0;
    let sd = body_distance(in.local, in.half_px, shape, radius_fraction);
    var rgb = in.color.rgb;
    if ((bits & FLAG_HOVERED) != 0u) {
        rgb = mix(rgb, vec3<f32>(1.0), 0.18);
    }
    var out = vec4<f32>(0.0);
    if ((bits & FLAG_FOCUSED) != 0u) {
        out = over(out, g.focus.rgb, g.focus.a * ring(sd, 3.5, 1.5));
    }
    if ((bits & FLAG_SELECTED) != 0u) {
        out = over(out, g.selection.rgb, g.selection.a * ring(sd, 0.0, 2.5));
    }
    out = over(out, rgb, cover(sd));
    if ((bits & FLAG_PINNED) != 0u) {
        let pin_sd = length(in.local - vec2<f32>(in.half_px, -in.half_px) * 0.62) - 2.5;
        out = over(out, g.pin.rgb, g.pin.a * cover(pin_sd) * cover(sd));
    }
    return out;
}

struct LinkOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) across: f32,
    @location(1) @interpolate(flat) color: vec4<f32>,
    @location(2) @interpolate(flat) half_width: f32,
}

@vertex
fn vs_link(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> LinkOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
    );
    let l = links[ii];
    let fa = attrs[l.end_a];
    let fb = attrs[l.end_b];
    var out: LinkOut;
    out.color = unpack(l.color);
    out.half_width = l.width * 0.5;
    out.across = 0.0;
    let seen = (((fa.flagword >> 2u) & FLAG_VISIBLE) != 0u) && (((fb.flagword >> 2u) & FLAG_VISIBLE) != 0u);
    if (!seen) {
        // Outside the clip volume: dropped.
        out.pos = vec4<f32>(2.0, 2.0, 2.0, 1.0);
        return out;
    }
    let a = project(positions[l.end_a].xy, fa.lift);
    let b = project(positions[l.end_b].xy, fb.lift);
    let d = b - a;
    let dir = d / max(length(d), 0.0001);
    let normal = vec2<f32>(-dir.y, dir.x);
    let reach = out.half_width + 1.0 / g.scale;
    let c = corners[vi];
    out.across = c.y * reach;
    out.pos = to_clip(mix(a, b, c.x) + normal * c.y * reach);
    return out;
}

@fragment
fn fs_link(in: LinkOut) -> @location(0) vec4<f32> {
    let a = in.color.a * cover(abs(in.across) - in.half_width);
    return vec4<f32>(in.color.rgb * a, a);
}
