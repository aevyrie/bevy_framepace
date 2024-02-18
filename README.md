<div align="center">

# bevy_framepace⏱️

**Framepacing and framelimiting for Bevy**

[![crates.io](https://img.shields.io/crates/v/bevy_framepace)](https://crates.io/crates/bevy_framepace)
[![docs.rs](https://docs.rs/bevy_framepace/badge.svg)](https://docs.rs/bevy_framepace)
[![CI](https://github.com/aevyrie/bevy_framepace/workflows/CI/badge.svg?branch=main)](https://github.com/aevyrie/bevy_framepace/actions?query=workflow%3A%22CI%22+branch%3Amain)
[![Bevy](https://img.shields.io/badge/Bevy%20tracking-main-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)

<video src = "https://user-images.githubusercontent.com/2632925/211992979-1892195b-b98f-424e-ae91-3fc4d6924b5e.mp4"></video>

</div>

### Usage

It's as simple as adding the plugin to your app:

```rs
app.add_plugins(bevy_framepace::FramepacePlugin);
```

By default, the plugin will automatically measure your framerate and use this for framepacing.

You can adjust the framerate limit at runtime by modifying the`FramepaceSettings` resource. For
example, to set the framerate limit to 30fps:

```rs
settings.limiter = Limiter::from_framerate(30.0),
```

See `demo.rs` in the examples folder, or run with:

```console
cargo run --release --example demo
```

## How it works

The plugin works by recording how long it takes to render each frame, and sleeping the main thread
until the desired frametime is reached. This ensures the next frame isn't started until the very
last moment, delaying the event loop from restarting. By delaying the event loop, and thus input
collection, this reduces motion-to-photon latency by moving reading input closer to  rendering the
frame.

The `spin_sleep` dependency is needed for precise sleep times. The sleep function in the standard
library is not accurate enough for this application, especially on Windows.

## Bevy Version Support

I intend to track the `main` branch of Bevy. PRs supporting this are welcome!

| bevy | bevy_framepace            |
| ---- | -------------------       |
| 0.13 | 0.15                      |
| 0.12 | 0.14                      |
| 0.11 | 0.13                      |
| 0.10 | 0.12                      |
| 0.9  | 0.7, 0.8, 0.9, 0.10, 0.11 |
| 0.8  | 0.5, 0.6                  |
| 0.7  | 0.4                       |
| 0.6  | 0.3                       |

## License

`bevy_framepace` is free, open source and permissively licensed! Except where noted (below and/or in
individual files), all code in this repository is dual-licensed under either:

* MIT License ([LICENSE-MIT](LICENSE-MIT) or
  [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option. This means you can select the license you prefer! This dual-licensing approach is
the de-facto standard in the Rust ecosystem and there are very good reasons to include both.

### Your contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
