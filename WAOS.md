# Ruffle for WaOS

The branch `waos` of `github.com/waos-dev/ruffle` is Ruffle as a [WaOS](https://github.com/waos-dev/waos) app: Ruffle's own desktop program (`desktop/`), with its menu bar, its dialogs and its preferences, built for `wasm32-wasip3` and run as `dev.waos.ruffle.desktop`. Its preferences and recent movies are kept in the app's own `/data/ruffle`.

It runs because the libraries it is written against have WaOS backends, on forks under `github.com/waos-dev`, each on a branch `waos` with a `WAOS.md` of its own: winit, egui, arboard, rfd, webbrowser, dirs, sys-locale, fontdb and reqwest.

What is on top of upstream Ruffle:

- `rust-toolchain.toml`: WaOS's nightly, with `wasm32-wasip3`. Upstream names no toolchain, so on this branch its crates build with this one too.
- `Cargo.toml`: the forks through `[patch.crates-io]`, with `wasdk` from WaOS's `main` until its next release.
- `.github/workflows/waos.yml`: a check that the desktop program builds for WaOS, formatted and without warnings, on each push to `waos`, and that the shared front-end crate does too. Upstream's workflows run on `master` alone.

What the desktop program chooses by target, each at one place:

- `desktop/src/task.rs`: starting a task and waiting for one. On WaOS a task runs on winit's event loop (`winit::platform::waos::spawn_local`) and a wait is `wasdk::block_on`. A task is never started with `wasdk::spawn` there: a wait on the app's thread takes such a task up and does not return until it ends.
- `desktop/src/backends.rs`: the player's sound, and the outputs the preferences list; `desktop/src/backends/waos/` holds WaOS's audio and video.
- `frontend-utils/src/backends/navigator/`: the navigator is one for both, and what differs is in two modules. `http.rs`: a request is sent by reqwest, on a desktop handed to tokio's runtime, on WaOS awaited where it is asked for; and a proxy the app names is used on a desktop alone. `tcp.rs`: a movie's TCP connection is tokio's on a desktop (`tcp/desktop.rs`) and one of WASI's sockets on WaOS (`tcp/waos.rs`), where its life runs as one of the navigator's own tasks.
- `desktop/src/open_requests.rs`: a movie the user opens with Ruffle, from WaOS or the system, arrives as an open request on WaOS, the first one too, where a desktop has the command line's argument at the start only.
- `desktop/Cargo.toml` and `frontend-utils/Cargo.toml`: cpal, the log file's library, tokio's runtime and sockets, async-io and egui-wgpu's own GPU backends are named for a desktop alone.
- The GPU instance is WaOS's one (`gui/controller.rs`), the log goes to stdout and there is no log file (`main.rs`, `log.rs`), a picked file is read in place (`backends/ui.rs`), and no gamepad library starts (`app.rs`).

What of upstream's is changed, each as a commit of its own that could go to Ruffle itself:

- `frontend-utils/Cargo.toml`: `reqwest` comes with the `navigator` feature, its only user, where it was always built, so the crate builds without it.
- `desktop/src`: the GUI, the player and its backends send the app's events through `RuffleEventSender` (`custom_event.rs`), where each held winit's `EventLoopProxy`. The app makes the sender from winit's proxy once, and nothing behaves differently.

## What works

Checked in a headless WaOS on a Mac:

- The window opens with the menu bar and the open dialog; a path typed there, or given on the command line, starts a movie; the File menu and the preferences open.
- H.264 video plays through WaOS's video decoder, which is the system's (VideoToolbox on a Mac).
- The system's fonts are found, each file read when first needed.
- The network, with Ruffle's own test movies and their expected traces: a movie opened by an `http://` address plays, and what it loads by a relative address is fetched there, by GET and by POST (`avm1/load_vars`); the same movie from an `https://` address on the internet; an XMLSocket and a `flash.net.Socket` connect, send, receive and close against a server playing the tests' `socket.json` (`avm1/xml_socket`, `avm2/socket_connect`), after Ruffle's own dialog asks whether the movie may connect; and a refused connection reaches the movie as a failed one. A server on this computer is behind WaOS's local network switch, off by default: `waos settings set local-network on`.
- Typing: text from WaOS's text input reaches Ruffle as winit's input method events, with Ruffle's "Input Method" preference on: `waos ui type` puts Japanese and accented text into the Open dialog's field and into a movie's input field.

Checked in the WaOS app on a Mac:

- "Open File" shows the system's panel on the window, and the file picked there plays.
- Full screen from the View menu, and back with Escape.
- A link from the Help menu opens in the browser.
- Text copied in a field reaches the system's clipboard, and text copied in another app pastes into one.
- A `.swf` opened with Ruffle (`waos open <file> --program <id>`) starts it with that movie, and one opened while it runs replaces the movie playing.

Not checked with the desktop program: that its sound is heard (it goes to WaOS's audio graph), H.263, VP6 and Screen Video (Ruffle's software decoders), a composition with a real input method, Export, a link a movie opens, and the dialogs a movie opens other than the one for a socket.

What WaOS does not offer an app, so Ruffle does not have it there: a proxy of the app's own choosing (`--proxy` and the Open dialog's proxy are not used, and the log says so; WaOS sends requests through the system's proxy), and HTTP/2 (WaOS's client speaks HTTP/1.1).

Not there yet: gamepads; dragging a `.swf` onto the window, and Ruffle offered by the system for `.swf` files, which WaOS has no way to declare yet; a font for Chinese, Japanese and Korean in Ruffle's own menus and dialogs, which name fonts a Mac does not have.

## Building

Needed:

- Rust through rustup: `rustup toolchain install` in the checkout installs `rust-toolchain.toml`'s.
- Java, which Ruffle's build compiles its ActionScript library with. OpenJDK 25 works.
- `waos`, and a WaOS to run it in, built from WaOS's `main`, which `wasdk` comes from: no release of WaOS carries it yet.

Then, in `desktop/`:

```sh
waos install --release --no-default-features --features software_video,lzma   # installs it in the WaOS on this computer
waos run --release --no-default-features --features software_video,lzma       # installs it, starts it, and shows what it writes
waos run --release --no-default-features --features software_video,lzma -- /data/movies/a.swf
```

A movie named as an argument is a path among the app's own files, not this computer's: WaOS mounts what the user picks in the dialog or opens with Ruffle, and `/data` is the app's own folder.

## Keeping up with upstream Ruffle

`master` follows `ruffle-rs/ruffle`; the port lives on `waos`.

```sh
git fetch upstream
git merge upstream/master     # on waos; `git diff upstream/master...waos` is the whole port
```

Where upstream changes what the port builds on, `cargo clippy -p ruffle_desktop --target wasm32-wasip3 --no-default-features --features software_video,lzma -- -D warnings` finds what must follow.
