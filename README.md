<div align="center">

# bevy_framepace⏱️

**Framepacing and framelimiting for Bevy**

[![crates.io](https://img.shields.io/crates/v/bevy_framepace)](https://crates.io/crates/bevy_framepace)
[![docs.rs](https://docs.rs/bevy_framepace/badge.svg)](https://docs.rs/bevy_framepace)
[![CI](https://github.com/aevyrie/bevy_framepace/workflows/CI/badge.svg?branch=main)](https://github.com/aevyrie/bevy_framepace/actions?query=workflow%3A%22CI%22+branch%3Amain)
[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-main-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)

<video src = "https://user-images.githubusercontent.com/2632925/164378172-faa136d5-e78f-4328-9962-afbf410753ce.mp4"></video>

</div>

### Usage

It's as simple as adding the plugin to your app:

```rs
app.add_plugin(bevy_framepace::FramepacePlugin::default())
```

By default, the plugin will automatically measure your framerate and use this for framepacing.

You can adjust the framerate limit when adding the plugin, or at runtime by modifying the`FramepacePlugin` resource. For example, to set the framerate limit to 30fps:

```rs
settings.framerate_limit = FramerateLimit::Manual(30),
```

See `demo.rs` in the examples folder, or run with:
```console
cargo run --release --example demo
```

## How it works

![image](https://user-images.githubusercontent.com/2632925/148489293-180b28e2-de49-4450-a1db-221d50b29a00.png)

The plugin works by recording how long it takes to render each frame, and sleeping the main thread until the desired frametime is reached.

The `spin_sleep` dependency is needed for precise sleep times. The sleep function in the standard library is not accurate enough for this application, especially on Windows.


## Bevy Version Support

I intend to track the `main` branch of Bevy. PRs supporting this are welcome!

|bevy|bevy_framepace|
|---|---|
|0.7|0.4|
|0.6|0.3|


## License

Bevy_framepace is free and open source! All code in this repository is dual-licensed under either:

* MIT License (LICENSE-MIT or http://opensource.org/licenses/MIT)
* Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)

at your option. This means you can select the license you prefer! This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are very good reasons to include both.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
