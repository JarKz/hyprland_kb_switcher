# About

The implementation of `kb_switcher` for Hyprland users, written on Rust.

## Compilation

Firstly, download rustup and install Rust toolchain. If you already have it, make sure that they're updated to last version. Then, run command:

```bash
cargo build --release
```

It will compile binary file at `target/release/kb_switcher`. You can move this binary file to any place, where you like to see.

## Usage

You can see all commands:

```bash
./kb_switcher help
```

Currently, only supported only two commands: `init` and `switch`.

- `init` – initializes the datafile, which placed at `$XDG_DATA_HOME/layout_switcher/data` or `$HOME/.local/share/layout_switcher/data`, if the environment variable `$XDG_DATA_HOME` is not present.
- `switch <dev_name>` – switches the layout for specific device. Currently it's not possible to dynamically detect active keyboards and run without this option.

## Contribution

If you found any issue or want to make code better, the Issue and PR section are open for you!
