#!/bin/python3

import os
import json
import subprocess
import time
from typing import cast

MAX_DURATION = 0.5
DEVICE = "keychron-keychron-k3"
PATH = os.getenv("XDG_DATA_HOME")
if PATH is None:
    PATH = cast(str, os.getenv("HOME")) + "/.config"

DATA_PATH = PATH + "/layout_switcher/data"


def swap(array: list[int], first: int, last: int) -> None:
    array[first], array[last] = array[last], array[first]


def get_data() -> dict:
    data: dict
    with open(DATA_PATH, "r") as datafile:
        data = json.loads(datafile.readline())
    return data


def dump_data(data: dict) -> None:
    with open(DATA_PATH, "w") as datafile:
        datafile.writelines(json.dumps(data))


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


def handle_press(data: dict) -> None:
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

    subprocess.run(["hyprctl", "switchxkblayout", DEVICE,
                   str(layouts[cur_freq])], capture_output=True)
    data["cur_freq"] = cur_freq


if __name__ == "__main__":
    press_time = time.time()
    data = get_data()
    compute_time_and_counter(press_time, data)
    handle_press(data)
    dump_data(data)
