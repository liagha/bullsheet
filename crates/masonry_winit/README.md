# masonry_winit (vendored)

A copy of `masonry_winit` 0.4.0 from crates.io, with one change.

## Why

Masonry draws the sheet with Vello on the GPU. The first frame builds Vello's
whole pipeline set, and the graphics device is created with wgpu's default
memory hints, which ask the driver to reserve large pools up front. On a
machine with an older discrete card that costs several hundred megabytes of
resident memory for a window 640 by 440 pixels.

Changing the hints to `MemoryHints::MemoryUsage` asks the driver to allocate
for what is actually used. Measured on a Quadro K1100M, the cost of the first
frame falls from 474 MB to 274 MB private, and the resident working set from
199 MB to 78 MB.

## The change

`src/vello_util.rs`, in `RenderContext::new_device`:

```rust
-    memory_hints: MemoryHints::default(),
+    memory_hints: MemoryHints::MemoryUsage,
```

`masonry_winit` does not expose the device or the renderer options, so this
cannot be changed from the outside; hence the copy.

## Updating

Replace the contents of `src/` with the new release from crates.io and reapply
the change above. The workspace picks this copy up through the `[patch.crates-io]`
section of the root `Cargo.toml`.

Upstream: https://github.com/linebender/xilem — Apache-2.0.
