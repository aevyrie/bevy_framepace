# bevy_framepace

### Framepacing and framelimiting for bevy

As simple as adding the plugin to your app:

```rs
app.add_plugin(bevy_framepace::FramepacePlugin::default())
```

You can adjust the framerate limit and framepacing forward estimation safety margin when adding the
plugin, or at runtime by modifying the `FramepacePlugin` resource.

See `demo.rs` in the examples folder, or run with:
```console
cargo run --release --example demo
```

## How it works

![image](https://user-images.githubusercontent.com/2632925/148489293-180b28e2-de49-4450-a1db-221d50b29a00.png)

The plugin works by recording how long it takes to render each frame, it then uses this to estimate how long the next frame will take, and sleeps at the end of the frame until just before it needs to start rendering the next one (blue annotation above). Because this system has to estime how long to sleep for, there is a small safety margin that is subtracted from the sleep time to prevent frame drops, in case the frame takes longer to render than expected. 

A second system then runs right before the frame is presented to the gpu, and sleeps until the desired frametime limit has been reached (red annotation above). This makes up for any error in forward estimation, and ensures frame time is exactly correct.

The `spin_sleep` dependency is needed for precise sleep times. The sleep function in the standard library is not accurate enough for this application, especially on Windows.
