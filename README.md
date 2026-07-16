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

## Installation

Check out the repository and run `cargo install --path crates/nemu-bin`. This will
put the executable at `~/.cargo/bin/nemu`. Nemu is a single self-contained executable,
and may be freely copied anywhere on your system.

### Dependencies

- Rust toolchain
- GTK 4.12+

## Hyprland Integration

Add a keybinding for running `nemu` somewhere in your Hyprland config:

```lua
hl.bind("SUPER + space", hl.dsp.exec_cmd("nemu"))
-- Separate keybinding for only the emoji picker, if you want:
hl.bind("SUPER + period", hl.dsp.exec_cmd("nemu emoji"))
```

Enable blur for the background of the Nemu window:

```lua
hl.layer_rule({
    name = "nemu",
    match = {
        namespace = "nemu",
    },
    no_anim = true,
    ignore_alpha = 0.5,
    blur = true,
    blur_popups = true,
})
```
