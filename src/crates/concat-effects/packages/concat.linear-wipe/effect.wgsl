struct Params { direction: f32, softness: f32 }

fn transition(uv: vec2<f32>, progress: f32) -> vec4<f32> {
    let p = smoothstep(0.0, 1.0, progress);
    var coord: f32;
    if (params.direction < 0.5) {
        coord = uv.x;
    } else if (params.direction < 1.5) {
        coord = 1.0 - uv.x;
    } else if (params.direction < 2.5) {
        coord = 1.0 - uv.y;
    } else {
        coord = uv.y;
    }
    let soft = max(params.softness * 0.003, 0.001);
    // The edge travels from just off one side of the frame to just off
    // the other, so that the cut completes: with the edge at the frame's
    // edge the soft band straddles it, and a strip of the outgoing
    // picture stayed in the last frame of every wipe - up to a third of
    // the frame at the softest setting. The other wipes here overshoot
    // for the same reason.
    let edge = mix(-soft, 1.0 + soft, p);
    let m = smoothstep(edge - soft, edge + soft, coord);
    return mix(to_at(uv), from_at(uv), m);
}
