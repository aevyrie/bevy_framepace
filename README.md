# bevy_framepace

### Framepacing and framelimiting for bevy

As simple as adding the plugin to your app:

```rs
app.add_plugin(bevy_framepace::FramepacePlugin::default())
```

You can adjust the framerate limit and framepacing forward estimation safety margin when adding the
plugin, or at runtime by modifying the `FramepaceSettings` resource.

See `demo.rs` in the examples folder, or run with:
```console
cargo run --release --example demo
```