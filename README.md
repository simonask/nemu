# Nemu Launcher

Nemu is a launcher app for Wayland compositors in the style of `dmenu`,
`fuzzel`, `krunner`, etc., but the difference is that it works how I want it to
work.

Nemu is written in Rust and uses Relm4/GTK4 under the hood. It should work with
any compositor that supports the `wlr-layer-shell-unstable-v1` protocol, such as
Hyprland, Niri, Sway, Cosmic, etc.

## Features
 
- **App launcher:** Live search of installed apps, sorted by best match. Shows
  you the command line that will be executed when you press Enter.
- **Emoji picker:** Search and pick emojis. The result is printed to stdout, so
  you can pass it to `wl-copy`. The Emoji picker can be launched directly by
  running `nemu emoji`. (TODO: Skin-tone selector)
- **dmenu mode:** Read items from stdin, write the selected item to stdout.
- TODO: **Calculator:** Parse simple mathematical expressions, and perform unit
  conversions.
- **Stylable:** Reads GTK stylesheet from `~/.config/nemu/style.css`.
- **Lightweight:** No daemons, no state.
