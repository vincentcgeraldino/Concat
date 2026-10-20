// Halation: the red-orange halo film lays around its blown highlights. Light
// passes through the emulsion, scatters off the backing and comes back, so
// the halo belongs to the picture's brightest parts and is drawn a little
// wider than they are - never a glow over the whole frame.
//
// The passes carry light, not colour: `across` and `down` each take the
// picture's own light, weighted above the threshold and premultiplied by
// alpha, and blur it. The last pass reads that bloom's luminance and lays
// the tint on with it, graded in the display encoding, where the halo of a
// film burn is judged and where 1.0 is white.
//
// A flat picture survives a normalised convolution unchanged, so on flat
// input `down_at` is `source_at` - which is what `fixtures.toml` pins.

// A WGSL package declares two things and nothing else: the `Params` struct
// whose fields are its knobs, in the manifest's order, and `fn effect`. The
// host reads the struct back through naga for each field's offset, so the
// order here is the layout there.
struct Params { threshold: f32, radius: f32, amount: f32, tint: f32 }

/// How far the taps reach, in sigmas. The Gaussian's width is `params.radius`
/// as a share of the layer, so a halo reads the same on a phone and a 4K cut.
const TAU: f32 = 3.14159265;

/// Taps a side of the centre, per pass: a constant in the source, never
/// read from the picture. FXC will not unroll a loop whose count it cannot
/// see, and a `sample` - an implicit gradient - inside a loop it cannot
/// unroll is a compile error (X3570, then X3511 once the count is in the
/// hundreds), so the count lives here and the step carries the width.
const TAPS: i32 = 12;

fn weight(d: f32, s: f32) -> f32 {
    return exp(-d * d / (2.0 * s * s));
}

/// The picture's own light where it is bright, premultiplied by alpha so a
/// clear pixel adds none, with colour kept: the halo takes its hue later.
fn source_at(uv: vec2<f32>) -> vec4<f32> {
    let c = sample(uv);
    let d = to_display(max(c.rgb, vec3<f32>(0.0)));
    let level = luma(d) * max(c.a, 0.0);
    let th = params.threshold / 100.0;
    let passed = smoothstep(th, min(th + 0.25, 1.0), level);
    return vec4<f32>(c.rgb * passed * c.a, c.a);
}

/// `source_at`'s light weighed along its row about `uv`: `2 * TAPS` taps
/// from `centre - reach` to `centre + reach`, a step apart, the step
/// widening with `sigma` so one fixed count covers any width. The screen
/// distance, so the scaling trap either way is caught in the middle, where
/// `bloom_texel` is the picture's own pixel.
fn across(uv: vec2<f32>) -> vec4<f32> {
    let width = frame.size.x;
    if (width < 1.0) {
        return source_at(uv);
    }
    let screen = length((uv - vec2<f32>(0.5)) * vec2<f32>(2.0));
    let sigma = params.radius / 100.0 * screen * frame.size.y / TAU;
    let centre = uv.x * width;
    let reach = TAU * sigma;
    let step = max(reach, 0.5) / f32(TAPS);
    let first = centre - reach + step * 0.5;
    var sum = vec4<f32>(0.0);
    var total = 0.0;
    for (var i = 0; i < 2 * TAPS; i++) {
        let a = first + f32(i) * step;
        let w = weight(a - centre, sigma);
        if (w > 0.0) {
            sum += source_at(vec2<f32>(a / width, uv.y)) * w;
            total += w;
        }
    }
    if (total <= 0.0) {
        return source_at(uv);
    }
    return sum / total;
}

/// `across` weighed down its column about `uv`, in its own pixels, the same
/// fixed taps and widening step.
fn down(uv: vec2<f32>) -> vec4<f32> {
    let rows = 1.0 / across_texel().y;
    let screen = length((uv - vec2<f32>(0.5)) * vec2<f32>(2.0));
    let sigma = params.radius / 100.0 * screen * 2.0;
    let centre = uv.y * rows;
    let reach = TAU * sigma;
    let step = max(reach, 0.5) / f32(TAPS);
    let first = centre - reach + step * 0.5;
    var sum = vec4<f32>(0.0);
    var total = 0.0;
    for (var i = 0; i < 2 * TAPS; i++) {
        let a = first + f32(i) * step;
        let w = weight(a - centre, sigma);
        if (w > 0.0) {
            sum += across_at(vec2<f32>(uv.x, a / rows)) * w;
            total += w;
        }
    }
    if (total <= 0.0) {
        return across_at(uv);
    }
    return sum / total;
}

/// The halo's colour at a tint: negative the deep red of a dense negative,
/// positive the orange of a print, zero the red-orange of the stock.
fn halation_tint(t: f32) -> vec3<f32> {
    let stock = vec3<f32>(1.0, 0.42, 0.16);
    let u = clamp(t / 100.0, -1.0, 1.0);
    if (u < 0.0) {
        return mix(stock, vec3<f32>(1.0, 0.10, 0.0), -u);
    }
    return mix(stock, vec3<f32>(1.0, 0.72, 0.28), u);
}

fn effect(uv: vec2<f32>) -> vec4<f32> {
    let c = sample(uv);
    let base = to_display(c.rgb);
    let bloom = luma(down_at(uv).rgb);
    let lit = base + halation_tint(params.tint) * bloom * (params.amount / 100.0) * 1.6;
    return vec4<f32>(from_display(lit), c.a);
}