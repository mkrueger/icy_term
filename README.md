# icy_term

IcyTERM is a BBS terminal program with allows you to connect to BBSes.
Visit https://www.telnetbbsguide.com/ for a start to enter the BBS world.

Features supported so far:
- Connection
  - [x] Telnet
  - [x] SSH
  - [x] Raw
  - [x] WebSockets (both secure and non-secure)
- Terminal encoding support
  - [x] ANSI
    - ANSI music
    - Sixel support
    - Loadable fonts
  - [x] Avatar
  - [x] Petscii
  - [x] Built in fonts, many DOS fonts, C64, Amiga & Atari font supported.
  - [x] Baud emulation
  - [x] ATASCII
  - [x] Viewdata
- File transfer protocols
  - [x] Xmodem, 1k & 1k-G (implemented but needs testing)
  - [x] Ymodem batch & Ymodem-G (implemented but needs testing)
  - [x] Zmodem/ZedZap (implemented but needs testing)
- Auto login
  - [x] IEMSI
  - [x] Terminate style auto login system
- Misc features
  - [x] Scrollback buffer (scrollwheel)
  - [x] Exporting buffer to disk & capture session
  - [x] IEMSI autologin
  - [x] Better rendering engine (maybe switching the UI to OpenGL)
  - [x] Copy & Paste
  - [x] Internationalization

# Get binaries

Get the latest release here:
https://github.com/mkrueger/icy_term/releases/latest

# Screenshots

Code page 437 (aka "DOS") support:

![DOS](assets/dos_bbs.png?raw=true "CP437 DOS")

Petscii screenshot:

![Petscii](assets/c64_bbs.png?raw=true "Petscii")

Atascii screenshot:

![Petscii](assets/atascii_bbs.png?raw=true "Atascii")

Viewdata screenshot:

![Viewdata](assets/viewdata_bbs.png?raw=true "Viewdata")

# History

I had an own BBS back in the 90'. When I started rust I searched a project and some days earlier I spoke with my wife about the good old days, PCBoard and then I got the idea to improve the PPL decompiler we used these days.
That was my first project and it was successful (https://github.com/mkrueger/PPLEngine).
Around that time I learned that there are still BBSes in the internet and I started to update my old ansi drawing tool (MysticDraw) however I lost a bit track because of the gtk4 bindings. It's very difficult to write even a mid sized UI application with these.

I tried to use druid & egui for that but none of these libraries felt that it was the way to go.

Now I want to make some small real world projects for each.

- I made a small prototype for the ansi drawer in druid (feels very comfortable to work with but the APP looks bad)
- Made a game cheating tool with egui - very nice tool but the APPs still have not the look & feel
- Now I made a terminal app with iced - PopOS! is switching to that library.

With all my ansi & buffer routines I've already written it makes sense to make a terminal. I need one program as well - most terminal programs are a bit too old school. Time to change that.

# Build instructions

* Install rust toolchain: https://www.rust-lang.org/tools/install
* Running: "cargo run"
* Building release version: "cargo build --release" - in target/release the icy_term executable is all you need

Building redistrib
# Bugs
Please file some - I'm sure the protocols still have issues
