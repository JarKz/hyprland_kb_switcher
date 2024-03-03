#!/bin/python3

import os
import subprocess
import json
import time
from typing import cast


def get_path() -> str:
    path = os.getenv("XDG_DATA_HOME")
    if path is None:
        path = cast(str, os.getenv("HOME")) + "/.config"

    data_path = path + "/layout_switcher"

    if not os.path.exists(data_path):
        os.makedirs(data_path)

    return data_path + "/data"


def get_layouts_from_hyprconf() -> str:
    data_about_layouts = subprocess.run(
        ["hyprctl", "getoption", "input:kb_layout", "-j"], capture_output=True)
    return json.loads(data_about_layouts.stdout)["str"].split(',')


def dump_data(data: dict) -> None:
    path = get_path()
    with open(path, "w") as datafile:
        datafile.writelines(json.dumps(data))


if __name__ == "__main__":
    layouts = get_layouts_from_hyprconf()

    data = {
        "last_time": time.time(),
        "layouts": list(range(len(layouts))),
        "cur_freq": 0,
        "cur_all": 0,
        "sum_time": 0,
        "counter": 0,
    }

    dump_data(data)
