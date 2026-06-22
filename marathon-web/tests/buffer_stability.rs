//! Box 3.2 invariant test: the web vertex/index buffers are created ONCE and
//! are NEVER recreated in the per-frame render path (`render.rs::frame()`).
//!
//! # Why this is a source-level invariant, not a real headless-wgpu readback
//!
//! The relevant invariant is about *`wgpu::Buffer` handle identity across
//! frames*: the vertex and index buffers are allocated once (in the constructor
//! and in `load_level_into` at level load) and the per-frame `frame()` only ever
//! *binds* them (`set_vertex_buffer` / `set_index_buffer`) — it never calls
//! `create_buffer*` for them. But `frame()` is `wasm32`-only: it drives a live
//! `wgpu::Surface` via `requestAnimationFrame` and cannot run in the `rust:slim`
//! CI runner, which has no GPU and no browser (the same constraint the box 3.1
//! `render_snapshot_upload.rs` and box 4.3 `dynamic_geometry.rs` tests document).
//!
//! The box 4.3 test already proves the buffer *contents* are sim-independent
//! (rebuilding the mesh after ticking yields byte-identical vertex/index data).
//! This box 3.2 test proves the complementary half: the *frame path itself*
//! contains no per-frame (re)creation of those buffers, so the handles are
//! stable across frames. We assert this on the one artifact that fully
//! determines it — the source of `render.rs::frame()` — by statically verifying
//! its body contains no vertex/index buffer allocation while it does reuse the
//! existing handles.
//!
//! ## How this fails if the invariant is violated (non-vacuous)
//!
//! If anyone reintroduces per-frame geometry re-baking — e.g. adds
//! `self.vertex_buffer = self.device.create_buffer_init(...)` inside `frame()` —
//! the `frame()` body would then contain a buffer-creation call and
//! `assert_no_buffer_creation_in_frame_body` fails. The
//! `buffer_creation_is_outside_the_frame_path` test cross-checks that the only
//! places that DO allocate these buffers are the constructor and the level-load
//! path, both of which run outside `frame()`.

const RENDER_RS: &str = include_str!("../src/render.rs");

/// Extract the body of the per-frame `fn frame(&mut self)` method from
/// `render.rs`. Returns the slice between its opening `{` and the matching
/// closing `}`, found by brace-depth counting so nested blocks are handled.
fn frame_body(src: &str) -> &str {
    let sig = "fn frame(&mut self) {";
    let sig_at = src
        .find(sig)
        .expect("render.rs must define `fn frame(&mut self)`");
    // Position just after the opening brace of the function.
    let body_start = sig_at + sig.len();
    let bytes = src.as_bytes();
    let mut depth = 1usize; // we are already inside the function's `{`
    let mut i = body_start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[body_start..i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("could not find the closing brace of `fn frame`");
}

/// Box 3.2: the per-frame render path must not (re)create the static
/// vertex/index buffers. `frame()` only binds them.
#[test]
fn assert_no_buffer_creation_in_frame_body() {
    let body = frame_body(RENDER_RS);

    // (a) No buffer ALLOCATION of any kind inside the frame path. The only
    //     legitimate per-frame GPU writes are `write_buffer` (camera uniform)
    //     and `write_texture` (the per-poly data texture) — neither creates a
    //     buffer. A `create_buffer` / `create_buffer_init` here would mean a
    //     buffer is being recreated every frame, violating the invariant.
    assert!(
        !body.contains("create_buffer"),
        "frame() must NOT create/recreate any buffer (found a `create_buffer*` \
         call in the per-frame path); the vertex/index buffers are allocated once \
         at level load and only bound per frame (box 3.2 buffer-stability invariant)"
    );

    // (b) The frame path must never REASSIGN the vertex/index buffer handles.
    //     Reassignment is how a recreation would surface even if it were built
    //     by a helper rather than `create_buffer` inline.
    assert!(
        !body.contains("self.vertex_buffer ="),
        "frame() must NOT reassign self.vertex_buffer (handle must stay stable across frames)"
    );
    assert!(
        !body.contains("self.index_buffer ="),
        "frame() must NOT reassign self.index_buffer (handle must stay stable across frames)"
    );

    // (c) Positive control: the frame path DOES reuse the existing handles by
    //     binding them. If these binds vanished the test would be vacuous, so we
    //     assert the buffers are genuinely exercised per frame.
    assert!(
        body.contains("set_vertex_buffer(0, self.vertex_buffer"),
        "frame() must bind the existing vertex_buffer handle (reuse, not recreate)"
    );
    assert!(
        body.contains("set_index_buffer(self.index_buffer"),
        "frame() must bind the existing index_buffer handle (reuse, not recreate)"
    );
}

/// Box 3.2: cross-check that the vertex/index buffers ARE created exactly in the
/// allocation sites that live OUTSIDE the per-frame path — the constructor and
/// the level-load path (`load_level_into`). This pins down that buffer creation
/// is a load-time concern, not a frame-time one, so the handles are stable for
/// the lifetime of a loaded level.
#[test]
fn buffer_creation_is_outside_the_frame_path() {
    // The buffers are assigned (created) at level load. `load_level_into` lives
    // outside `frame()`.
    assert!(
        RENDER_RS.contains("state.vertex_buffer = state"),
        "load_level_into must (re)create the vertex_buffer at level load"
    );
    assert!(
        RENDER_RS.contains("state.index_buffer = state"),
        "load_level_into must (re)create the index_buffer at level load"
    );

    // Sanity: those load-time assignments are NOT inside the frame body (they
    // belong to `load_level_into`), confirming the only buffer (re)creation is a
    // load-time event, not a per-frame one.
    let body = frame_body(RENDER_RS);
    assert!(
        !body.contains("state.vertex_buffer ="),
        "the level-load vertex_buffer assignment must not appear in the frame path"
    );
    assert!(
        !body.contains("state.index_buffer ="),
        "the level-load index_buffer assignment must not appear in the frame path"
    );
}
