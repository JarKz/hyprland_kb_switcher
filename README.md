# The keyboard switcher for Hyprland users

If you use more than two languages, you know, that is not very comfort way. Especially if you use mostly two languages, thirt at times.

It script will help you, because it based on concept, which uses on MacOS: it switches only two last languages if key combination presses only one time, but when combination presses more than one, then switches between all languages.

## Restricts

Because of Hyprland implementation, you can't switch language layout for all devices. I know, you can, but in needs to make a lot of calls, so I don't want make program so slow. So, there are two stages of script:

- Initialization
- Switching

The Initialization means loading default layout, which you can get, using command:

```bash
hyprctl getoption input:kb_layout
```

> __NOTE__
> I loads the layouts not for getting names, but count. The `hyprctl switchxkblayout` provide switching using indices and I use it.

And it stores as data in JSON format to file at `$XDG_DATA_HOME/layout_switcher/data` or `$HOME/.local/share/layout_switcher/data`, if the environment variable `$XDG_DATA_HOME` is not present. You can see it after processing this command via:

```bash
cat $XDG_DATA_HOME/layout_switcher/data
# or
cat $HOME/.local/share/layout_switcher/data
```

The Switching is a process, when layout switches to another. And the main restrict is need to use device name for it. Unfourtenately, Hyprland doesn't provide global layout switching, so need **force** set device name, when you switches.

For example:

```bash
./kb_switcher switch keychron-keychron-k3
```

> __NOTE__
> Also it's mean that you must not to set the custom layout set for specific devicee. It's guaranteed to cause UB (Undefined Behavior).

I know about batching, which `hyprctl` provides, but it very-very expensive call for all devices, so I decline it.

## Contribution

If you have some issues, or have better implementation, that I have, I'm open to contribution! Open issue or make PR, if you sure about problem.

## License

[MIT](/LICENSE)
