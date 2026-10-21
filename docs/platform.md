# Window, display, frame pacing and saved settings

How the window's mode, vsync, the frame cap and `PlayerPrefs` work. Code: `iron_platform`
([display.rs](../crates/iron_platform/src/display.rs), [platform.rs](../crates/iron_platform/src/platform.rs)),
`iron_render` ([surface.rs](../crates/iron_render/src/surface.rs)), `iron_app`
([display.rs](../crates/iron_app/src/display.rs), [prefs.rs](../crates/iron_app/src/prefs.rs),
[logging.rs](../crates/iron_app/src/logging.rs),
[main.rs](../crates/iron_app/src/main.rs)) and `@iron/engine`
([screen.ts](../js/engine/src/screen.ts), [prefs.ts](../js/engine/src/prefs.ts),
[app.ts](../js/engine/src/app.ts), [quality.ts](../js/engine/src/quality.ts)).

## Window modes

`FullScreenMode` has Unity's four modes (scripts use the names):

| Mode | What the window does |
|---|---|
| `ExclusiveFullScreen` | The display switches to the chosen size and refresh rate |
| `FullScreenWindow` | A borderless window covering the monitor at its own resolution |
| `MaximizedWindow` | A normal window, maximized |
| `Windowed` | A normal window of the chosen size |

`DisplaySettings` is the mode plus width, height (physical pixels) and refresh rate (Hz, 0 means
"the monitor's rate"). The default is windowed, 1280x720. A width and height of 0 mean "no size asked
for"; `window_width` / `window_height` remember the last windowed size.

**Platform rules.** `supported_modes` lists what the platform offers, most fullscreen first:
all four on desktop, three on Wayland (compositors own the display mode, so no exclusive
fullscreen), and only `FullScreenWindow` on iOS and Android. `effective_mode` maps an unsupported
request to the nearest mode: fullscreen requests try `FullScreenWindow` then `MaximizedWindow`,
window requests try `MaximizedWindow` then `FullScreenWindow`. Scripts can read the list from
`Screen.supportedFullScreenModes` and show only those.

**`display::apply`** puts a window in a mode and returns what took effect:
- Exclusive: `exclusive_target` gives the size and rate asked for, or with no size the monitor's current
  resolution and rate (not the smallest mode). `closest_mode` then picks the video mode nearest to it
  (size first, then refresh rate; with no rate asked for, the fastest). The result carries that mode's
  real size and rate. If the monitor reports no modes it falls back to borderless.
- Windowed: `windowed_target` gives the size asked for, or with none the remembered window size. It is
  clamped between 320x240 and the monitor, then requested with `request_inner_size`. So leaving
  fullscreen goes back to the previous window size (`M5-DSP-04`).
- Borderless and maximized ignore the size.

`resolutions` lists the monitor's video modes, largest and fastest first, without repeats.
`current_resolution` is the monitor's size and rate now.

## How settings flow

1. At startup `main` reads the manifest's `iron.display` (`{ mode, width, height }`) as the defaults,
   then lets the saved `iron.display` pref override them field by field (`display::from_json`;
   unknown mode names and sizes below 1 are ignored). The window is created at the resulting size.
2. `__iron_info.display` is set before the bundle runs (`display::state`). It holds the settings
   plus `supportedModes`, `resolutions` and `currentResolution`; before the window exists the last
   two are empty/null and all four modes are listed. It is rebuilt and re-evaluated once the window
   is ready and after every `screen.set`.
3. `window_ready` applies a non-windowed mode **before the first frame**, so the window never
   flashes at the wrong size (it is created hidden and shown after its first frame).
4. At run time `Screen.setResolution(w, h, mode, refreshRate)` sends `screen.set mode width height
   refreshRate`. The `fullScreenMode` and `fullScreen` setters send a size of 0, so the app picks the
   monitor's resolution or the last window size (see above). The app applies it, writes what was asked for to the `iron.display` pref and
   re-evaluates `__iron_info.display` with what took effect.
5. **Screenshot runs** (`--screenshot`) stay windowed and never apply a mode, and their prefs are
   kept in memory, so renders are the same every time.

`Screen.width`/`height` (UI pixels) still come from `__iron_screen`; `Screen.windowSize` is the
physical size from `__iron_info.display`.

## Log file

`iron` is a Windows GUI-subsystem program, so it has no console of its own. `logging::init` sends the log
(`tracing`, default filter `info,wgpu_core=warn,wgpu_hal=error`, `RUST_LOG` overrides it) to `Player.log`
in the game's settings folder, next to `prefs.json`, with the same ANSI colours a terminal shows. The
previous run's file is kept as `Player-prev.log`. `IRON_LOG_FILE` names another file. Started from a
terminal, it also logs there: on Windows `attach_console` attaches to the parent's console when stderr
is not already redirected, and piped output (scripts, tests) works as before. A panic hook writes the
panic to the log (`M5-APP-04`).

## Vsync and the frame cap

- `QualitySettings.vSyncCount` (0 or 1) goes out as the seventh argument of `render.quality`. It
  fills `Quality.vsync` (default off: the engine tracks raw performance, so frames are shown as soon as they are drawn;
  the quality levels and `Quality::default()` have `vSyncCount` 0 and the examples have no vsync control; scripts may still set it).
  A saved quality choice never turns it back on (`apply_default_quality` and `_applyDefaultQuality` take vsync from the level). The app calls `WindowSurface::set_vsync` before each frame,
  which switches the present mode between `AutoVsync` and `AutoNoVsync` and reconfigures the
  surface only when the mode changes. wgpu picks what the platform has (relaxed or plain FIFO;
  mailbox or immediate).
- `Application.targetFrameRate` sends `app.targetFrameRate` (-1 means no cap). Like Unity's, it only
  counts with vsync off (the default), since vsync already paces frames. Screenshot runs ignore it.
- The cap is `PlatformApp::frame_interval`. After handling events, the loop in `about_to_wait`
  checks the time since the last frame started; if the next frame is not due it sets
  `ControlFlow::WaitUntil(due)` and does not request a redraw. Input events still wake the loop.
  Otherwise it sets `ControlFlow::Wait` and requests a redraw.
- The target frame rate is not saved; games keep it in `PlayerPrefs` if players choose it.
- `lifecycle::frame_cap(target_fps, vsync, screenshot)` makes the interval: `None` unless vsync is off,
  the rate is above 0 and it is not a screenshot run (`M5-APP-08`). The rate set by scripts goes into
  `__iron_info.app.targetFrameRate`, so a full script reload keeps it.

## Focus and pausing

[lifecycle.rs](../crates/iron_app/src/lifecycle.rs) holds `Lifecycle`, which decides whether the game is
in the background and whether that pauses it.

- **Background** means unfocused (`InputEvent::Focused(false)`), occluded (`InputEvent::Occluded`, sent on
  platforms that tell, such as macOS and Wayland) or sized to 0 x 0 (how Windows reports a minimised
  window). `App::input` feeds those three into `Lifecycle`.
- **Pausing** happens in the background unless `runInBackground` is on (`Application.runInBackground`,
  which sends `app.runInBackground`, or `runInBackground` in `project.json`; default off). Screenshot
  runs never pause. Switching `runInBackground` off while in the background pauses at once. The script
  value goes into `__iron_info.app.runInBackground`, which a full reload keeps.
- `Lifecycle::update` is called from `App::sync_lifecycle` (after each input event and op) and returns a
  `Change`: `paused`, `resumed` (with how long the pause lasted) and `report`, the new
  `(focused, paused)` for scripts when either changed.
- **While paused**, `App::tick` returns early after polling the hot reload watchers: no fixed ticks,
  updates, timers or animation. `Audio::set_muted(true)` fades the main track to silence over 0.1 s
  without losing the master volume scripts set (a mixer opened during a pause starts muted).
  `PlatformApp::paused()` is true, so the event loop requests no frames and waits; it calls
  `PlatformApp::idle()` every `IDLE_POLL` (250 ms), which polls script hot reload so rebuilds show on resume.
- **On resume**, `started` moves forward by the pause, so script time (timers, caret, UI time) skips it,
  and `FixedClock::restart` makes the first frame count as one tick instead of catching up. `FixedClock`
  turns frame times into fixed ticks (at most `MAX_TICKS_PER_FRAME`, dropping the rest after a hitch).
  The mixer unmutes.
- **Scripts.** `__iron_focus(focused, paused)` updates `Application.isFocused` and `isPaused`, then calls
  `onApplicationPause(paused)` on active components when pausing changed, then the `onFocusChanged`
  listeners and `onApplicationFocus(focused)` when focus changed. A throwing listener doesn't stop the
  rest. New scripts after a reload start believing they are focused and running
  (`scripts_restarted`), so the next update tells them the real state.

Tests: `M5-APP-05` (window events), `-06` (muting, in `iron_audio/tests/audio.rs`), `-07` (`Lifecycle` and
`FixedClock`), `-08` (the frame cap) and `-09` (manifest and `__iron_info`) in the app crate; `-10` (the JS
side) in `js/engine/test/app.test.ts`.

## PlayerPrefs

`PlayerPrefs` has `getInt`/`getFloat`/`getString` (with a fallback; a key of the wrong type gives the
fallback, ints are truncated), `setInt`/`setFloat`/`setString`, `hasKey`, `deleteKey`, `deleteAll`
and `save`. Values are numbers and strings.

- **File.** One JSON object per game, `prefs.json`, in the OS settings folder:
  - Windows: `%APPDATA%\<company>\<title>\`
  - macOS: `~/Library/Application Support/<identifier, or company.title>/`
  - Linux and others: `$XDG_CONFIG_HOME/<company>/<title>/` (or `~/.config`)

  Names come from the manifest and are made file-name safe. `IRON_PREFS_DIR` replaces the folder
  (tests, portable installs). With no settings folder, or on screenshot runs, prefs live in memory
  only.
- **Read.** The app loads the file at startup and puts it in `__iron_info.prefs`. `prefs.ts` copies
  it into a map when the engine loads, so reads never cross the boundary. A file that is not a JSON
  object is ignored with a warning.
- **Write.** Sets and deletes go to the app as `prefs.set key value`, `prefs.delete key`,
  `prefs.deleteAll` and `prefs.save`. The file is written on `save()` and when the `App` is dropped
  (so on exit). `Prefs::save` skips the write when nothing changed and writes through a temporary
  file and a rename, so a crash cannot leave half a file.
- **Engine keys** start with `iron.`: `iron.display` (the window mode and size, a JSON object) and
  `iron.quality` (the quality level and its current values, a JSON string). `deleteAll` removes
  them too.
- **Quality.** Every quality or shadow change saves `iron.quality`. `_applyDefaultQuality`
  restores it when the engine loads (level by name, then the saved values over that level), without
  writing the starting state back as if the player had chosen it.

## Limits

- One monitor: the window's current monitor is used, and there is no way to pick another.
- Exclusive fullscreen is untested across platforms.
- No HDR output modes.
- Phones and tablets: no safe area or orientation handling yet; only `FullScreenWindow`.
