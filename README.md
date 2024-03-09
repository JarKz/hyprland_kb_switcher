# The keyboard switcher for Hyprland users

If you use more than two languages, you know, that is not very comfort way. Especially if you use mostly two languages, thirt at times.

It script will help you, because it based on concept, which uses on MacOS: it switches only two last languages if key combination presses only one time, but when combination presses more than one, then switches between all languages.

## Restricts

Because of Hyprland implementation, you can't switch layouts for all devices together. It may be annoying, if you use laptop and external keyboard as well. So, this program takes it into account and provide possibility switching for all selected devices. But, only for these devices, which uses default option of `kb_layout`, which is defined in `input:kb_layout`. For example, in hyprland.conf:

```conf
input {
    kb_layout = us,ru,by
}
```

When your device (mostly keyboard) have specific option `kb_layout`, the program may be work not correct as you want.

Why? Because I get only the option using command:

```bash
hyprctl getoption input:kb_layout
```

Also, you can't use the same bind, which you already defined in `kb_option` in input section of hyprland.conf file. For example, in hyprland.conf:

```conf
input {
    kb_layout = us,ru,by
    kb_option = grp:win_space_toggle
}

# You must don't use it, when the bint is defined above.
# bind = SUPER,SPACE,exec,kb_switcher switch
```

## Usage

Firstly, make sure that you have last updated rust tools. You can download it from official site, or update to last version using command:

```bash
rustup update
```

And install application using command:

```bash
cargo install --path .
```

And enjoy with application!

> __NOTE__
> If you want set another name of application, you can change it in Cargo.toml file: rename the package name, which placet at second line from top.

You can see all command using `help`, and their documentation. For example:

```bash
kb_switcher help init
```

It'll print in output the documentation of this command. Please, read it before using commads.

## Contribution

If you have an issue or want to make this application more better, the issue and PR page is open for you!

## License

[MIT](/LICENSE)
