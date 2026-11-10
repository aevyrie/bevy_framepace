# 0.21.1 - 2026-11-10

- Fix limited frames alternating between long and short. A sleep that ran long, which on macOS is up to half again the time asked for, cut the next frame's sleep by the whole overrun. The limiter now waits for each frame's deadline to within microseconds, and a frame that runs long no longer shortens the next one.
- Fix the limiter skipping a frame, or missing a settings change, when the main thread held a lock at the same moment.

# 0.20.1 - 2025-12-08

- Fix non-send usage of winit.