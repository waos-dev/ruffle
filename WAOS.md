# Ruffle for WaOS

The branch `waos` of `github.com/waos-dev/ruffle` is Ruffle as a [WaOS](https://github.com/waos-dev/waos) program: built for `wasm32-wasip3` and run as `dev.waos.ruffle`. It plays one movie in one window: the file its first argument names, or the one its user picks in WaOS's file dialog. What movies save (shared objects) lives in the program's own `/data/shared_objects`.

What is on top of upstream Ruffle:

- `waos/`: the crate `ruffle_waos`, the player put together from Ruffle's own parts on WaOS's: Ruffle's wgpu renderer on the window's view through `wasdk-wgpu`, Ruffle's mixer played through WaOS's audio graph, the view's pointer and keys as Ruffle's events, and the small backends the player asks its platform for. Each module's docs say what it does and what it does not do yet. On any target but WASI the crate is an empty binary, so the workspace's own checks pass over it.
- `rust-toolchain.toml`: WaOS's nightly, with `wasm32-wasip3`. Upstream names no toolchain, so on this branch its crates build with this one too.
- `Cargo.toml`: `waos` among the workspace's members.
- `.github/workflows/waos.yml`: a check that the port builds, formatted and without warnings, on each push to `waos`. Upstream's workflows run on `master` alone.

What of upstream's is changed, each as a commit of its own that could go to Ruffle itself:

- `frontend-utils/Cargo.toml`: `reqwest` comes with the `navigator` feature, its only user, where it was always built. Without it the crate builds for `wasm32-wasip3`, which has no sockets of its own for `reqwest` and `tokio` to stand on, so the port can use the preferences, bookmarks and bundles Ruffle's UI is built on. The desktop program asks for `navigator` and builds as before.
- `desktop/src`: the GUI, the player and its backends send the app's events through `RuffleEventSender` (`custom_event.rs`), where each held winit's `EventLoopProxy`. The app makes the sender from winit's proxy once, and nothing behaves differently. It is the first step of making the GUI usable with another event loop, WaOS's.

## The desktop program on WaOS

Ruffle's own desktop program (`desktop/`), with its menu bar, its dialogs and its preferences, also builds for `wasm32-wasip3` and runs as `dev.waos.ruffle.desktop`. It is the way forward for the port: `waos/` puts a player together by hand, and the desktop program is Ruffle's real one.

It runs because the libraries it is written against have WaOS backends, on forks under `github.com/waos-dev`, each on a branch `waos` with a `WAOS.md` of its own: winit, egui, arboard, rfd, webbrowser, dirs, sys-locale, fontdb and reqwest. The workspace's `Cargo.toml` takes them through `[patch.crates-io]`, with `wasdk` from WaOS's `main` until its next release.

What the desktop program itself chooses by target, each at one place:

- `desktop/src/task.rs`: starting a task and waiting for one. On WaOS a task runs on winit's event loop (`winit::platform::waos::spawn_local`) and a wait is `wasdk::block_on`. A task is never started with `wasdk::spawn` there: a wait on the app's thread takes such a task up and does not return until it ends.
- `desktop/src/backends.rs`: the player's sound, and the outputs the preferences list; `desktop/src/backends/waos/` holds WaOS's audio and video, as `waos/` has them.
- `frontend-utils/src/backends/navigator/`: the navigator is one for both, and what differs is in two modules. `http.rs`: a request is sent by reqwest, on a desktop handed to tokio's runtime, on WaOS awaited where it is asked for; and a proxy the app names is used on a desktop alone. `tcp.rs`: a movie's TCP connection is tokio's on a desktop (`tcp/desktop.rs`) and one of WASI's sockets on WaOS (`tcp/waos.rs`), where its life runs as one of the navigator's own tasks.
- `desktop/Cargo.toml` and `frontend-utils/Cargo.toml`: cpal, the log file's library, tokio's runtime and sockets, async-io and egui-wgpu's own GPU backends are named for a desktop alone.
- The GPU instance is WaOS's one (`gui/controller.rs`), the log goes to stdout (`main.rs`), a picked file is read in place (`backends/ui.rs`), and no gamepad library starts (`app.rs`).

Built in `desktop/` with `waos run --release --no-default-features --features software_video,lzma`.

Checked in a headless WaOS on a Mac: the window opens with the menu bar and the open dialog; a path typed there, or given on the command line, starts a movie; H.264 video plays through WaOS's decoder; the File menu and the preferences open; and the system's fonts are found, each file read when first needed.

The network, checked in a headless WaOS on a Mac with Ruffle's own test movies and their expected traces: a movie opened by an `http://` address plays, and what it loads by a relative address is fetched there, by GET and by POST (`avm1/load_vars`); the same movie from an `https://` address on the internet; an XMLSocket and a `flash.net.Socket` connect, send, receive and close against a server playing the tests' `socket.json` (`avm1/xml_socket`, `avm2/socket_connect`), after Ruffle's own dialog asks whether the movie may connect; and a refused connection reaches the movie as a failed one. A server on this computer is behind WaOS's local network switch, off by default: `waos settings set local-network on`.

Checked in the WaOS app on a Mac: "Open File" shows the system's panel on the window, and the file picked there plays; full screen from the View menu and back with Escape; a link from the Help menu opens in the browser; text copied in a field reaches the system's clipboard.

What WaOS does not offer an app, so Ruffle does not have it there: a proxy of the app's own choosing (`--proxy` and the Open dialog's proxy are not used, and the log says so; WaOS sends requests through the system's proxy), and HTTP/2 (WaOS's client speaks HTTP/1.1).

Not there yet: gamepads; pasting what another app copied, which waits on WaOS (the paste works once WaOS itself has been in front since the copy). Not checked: Export, a link a movie opens (the Help menu's links open), and the dialogs a movie opens other than the one for a socket.

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
