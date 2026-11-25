# Ruffle for WaOS

The branch `waos` of `github.com/waos-dev/ruffle` is Ruffle as a [WaOS](https://github.com/waos-dev/waos) program: built for `wasm32-wasip3` and run as `dev.waos.ruffle`. It plays one movie in one window: the file its first argument names, or the one its user picks in WaOS's file dialog. What movies save (shared objects) lives in the program's own `/data/shared_objects`.

What is on top of upstream Ruffle:

- `waos/`: the crate `ruffle_waos`, the player put together from Ruffle's own parts on WaOS's: Ruffle's wgpu renderer on the window's view through `wasdk-wgpu`, Ruffle's mixer played through WaOS's audio graph, the view's pointer and keys as Ruffle's events, and the small backends the player asks its platform for. Each module's docs say what it does and what it does not do yet. On any target but WASI the crate is an empty binary, so the workspace's own checks pass over it.
- `rust-toolchain.toml`: WaOS's nightly, with `wasm32-wasip3`. Upstream names no toolchain, so on this branch its crates build with this one too.
- `Cargo.toml`: `waos` among the workspace's members.
- `.github/workflows/waos.yml`: a check that the port builds, formatted and without warnings, on each push to `waos`. Upstream's workflows run on `master` alone.

What of upstream's is changed, each as a commit of its own that could go to Ruffle itself:

- `frontend-utils/Cargo.toml`: `reqwest` comes with the `navigator` feature, its only user, where it was always built. Without it the crate builds for `wasm32-wasip3`, which has no sockets of its own for `reqwest` and `tokio` to stand on, so the port can use the preferences, bookmarks and bundles Ruffle's UI is built on. The desktop program asks for `navigator` and builds as before.

## What works, and what does not yet

Checked in a headless WaOS on a Mac, with movies from Ruffle's own tests:

- A movie is drawn and animates, and its sound is mixed and written to WaOS's output.
- Keys reach a movie as key events and as typed text, and the pointer as mouse events.
- A bar of controls over the movie, drawn with egui: pause and play, and the volume. A click on the bar is not a click in the movie. It is the first piece of Ruffle's own UI here, through `wasdk-adapter-egui`; the menu bar and the dialogs of Ruffle's desktop program are not here yet.
- H.264 video plays through WaOS's video decoder, which is the system's (VideoToolbox on a Mac): Ruffle's own test movie `visual/video/h264`, whose pictures are stored out of the order they are shown in, shows them in order. H.263, VP6 and Screen Video play through Ruffle's software decoders. On a system where WaOS has no decoder yet, H.264 is not shown and the log says so.

Not yet, each answered as a platform without it and logged where a movie asks:

- Fetching over the network, opening a link, and sockets. A movie reads only files beside it.
- The clipboard, full screen, a system's fonts (text in a device font is drawn in the font Ruffle carries), file dialogs a movie opens, and an on-screen keyboard.

## Building

Needed:

- Rust through rustup: `rustup toolchain install` in the checkout installs `rust-toolchain.toml`'s.
- Java, which Ruffle's build compiles its ActionScript library with. OpenJDK 25 works.
- `waos`, and a WaOS to run it in, built from the revision of WaOS that `waos/Cargo.toml` names for `wasdk`, or newer: what the port was checked on. No release of WaOS carries that revision yet.

Then, in `waos/`:

```sh
waos run --release                          # in the WaOS app on this computer; asks for a movie
waos run --release -- /data/movies/a.swf    # a movie among the program's own files
waos package                                # target/waos/dev.waos.ruffle-<version>.wapk and index.json
```

A movie named as an argument is a path among the program's own files, not this computer's: WaOS mounts what the user picks in the dialog, and `/data` is the program's own folder.

## Keeping up with upstream Ruffle

`master` follows `ruffle-rs/ruffle`; the port lives on `waos`.

```sh
git fetch upstream
git merge upstream/master     # on waos; `git diff upstream/master...waos` is the whole port
```

Where upstream changes a backend's trait, `cargo check -p ruffle_waos --target wasm32-wasip3` finds what `waos/` must follow.
