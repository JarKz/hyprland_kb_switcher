#!/usr/bin/python3

import os
import sys
import subprocess
import json
import time
from typing import cast

MAX_DURATION = 0.5

path = os.getenv("XDG_DATA_HOME")
if path is None:
    path = cast(str, os.getenv("HOME")) + "/.config"

DATA_PATH = path + "/layout_switcher"
DATA_STORAGE = DATA_PATH + "/data"


def init_path() -> None:
    if not os.path.exists(DATA_PATH):
        os.makedirs(DATA_PATH)


def load_layouts_from_hyprconf() -> str:
    data_about_layouts = subprocess.run(
        ["hyprctl", "getoption", "input:kb_layout", "-j"], capture_output=True)
    return json.loads(data_about_layouts.stdout)["str"].split(',')


def load_data() -> dict:
    with open(DATA_STORAGE, "r") as datafile:
        return json.loads(datafile.readline())


def dump_data(data: dict) -> None:
    with open(DATA_STORAGE, "w") as datafile:
        datafile.writelines(json.dumps(data))


def init():
    init_path()
    layouts = load_layouts_from_hyprconf()
    data = {
        "last_time": time.time(),
        "layouts": list(range(len(layouts))),
        "cur_freq": 0,
        "cur_all": 0,
        "sum_time": 0,
        "counter": 0,
    }

    dump_data(data)


def swap(array: list[int], first: int, last: int) -> None:
    array[first], array[last] = array[last], array[first]


def compute_time_and_counter(press_time: float, data: dict) -> None:
    diff = press_time - data["last_time"]
    data["last_time"] = press_time

    data["sum_time"] += diff

    if data["sum_time"] < MAX_DURATION:
        data["counter"] += 1
    else:
        data["sum_time"] = 0
        data["counter"] = 1

    if data["counter"] >= 2:
        data["sum_time"] = 0


def handle_press(data: dict, device: str) -> None:
    layouts = data["layouts"]
    cur_freq = data["cur_freq"]
    counter = data["counter"]

    if counter <= 1:
        cur_freq = (cur_freq + 1) % 2
    else:
        cur_all = data["cur_all"] + 1 if counter > 2 else 2
        cur_all %= len(layouts)

        if cur_all == cur_freq:
            cur_all += 1

        data["cur_all"] = cur_all
        swap(layouts, cur_freq, cur_all)

    subprocess.run(["hyprctl", "switchxkblayout", device,
                   str(layouts[cur_freq])], capture_output=True)
    data["cur_freq"] = cur_freq


def switch(device: str):
    press_time = time.time()
    data = load_data()
    compute_time_and_counter(press_time, data)
    handle_press(data, device)
    dump_data(data)


def unknown_command():
    print("""Unknown command! Currently possible only two commands:
        init – initializes storage file with current layouts.
        switch <dev_name> – switches keyboard layout (languages) for specific the <dev_name> device name aka keyboard.""")
    exit(1)


if __name__ == "__main__":
    command, *args = sys.argv[1:]
    print(command)

    match command:
        case "init": init()
        case "switch":
            if len(args) == 0:
                print("Expected device name!")
                exit(1)
            switch(args[0])
        case _: unknown_command()
