use clap::{Parser, Subcommand};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{error::Error, io::Write, path::PathBuf, process::Command, time::UNIX_EPOCH};

static DATA_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let mut data_path = match std::env::var("XDG_DATA_HOME") {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            let mut other_path =
                PathBuf::from(std::env::var("HOME").expect("Must be HOME env variable!"));
            other_path.push(".local");
            other_path.push("share");
            other_path
        }
    };
    data_path.push("kb_switcher");
    data_path
});

static DATA_STORAGE: Lazy<PathBuf> = Lazy::new(|| {
    let mut other_path = DATA_PATH.clone();
    other_path.push("data");
    other_path
});

const MAX_DURATION: f64 = 0.5;

#[derive(Serialize, Deserialize)]
struct Data {
    devices: Vec<String>,
    last_time: f64,
    layouts: Vec<usize>,
    cur_freq: usize,
    cur_all: usize,
    sum_time: f64,
    counter: u8,
}

/// Simple program, which can switch keyboard layout more comfotrable in Hyprland, like on MacOS.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct KbSwitcherCmd {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Initial command of kb_switcher
    ///
    /// Loads layouts from Hyprland config, which user uses, and stores
    /// in data. Also initializes dump file at $XDG_DATA_HOME/layout_switcher/data
    /// or $HOME/.local/share/layout_switcher/data.
    ///
    /// Must be called before `switch` command!
    Init { devices: Vec<String> },

    /// Switches the keyboard layouts like MacOS
    ///
    /// For correct work, please run firstly `init` command and do not delete the dump file!
    Switch,
}

impl KbSwitcherCmd {
    pub fn process(&self) -> Result<(), Box<dyn Error>> {
        match self.cmd {
            Cmd::Init { ref devices } => init(devices),
            Cmd::Switch => switch(),
        }
    }
}

fn init(devices: &Vec<String>) -> Result<(), Box<dyn Error>> {
    let layouts = load_layouts_from_hyprconf()?;
    let time = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs_f64();

    let data = Data {
        devices: devices.clone(),
        last_time: time,
        layouts: (0..layouts.len()).collect(),
        cur_freq: 0,
        cur_all: 0,
        sum_time: 0.0,
        counter: 0,
    };

    std::fs::create_dir_all(&*DATA_PATH)?;
    dump_data(data)
}

fn switch() -> Result<(), Box<dyn Error>> {
    let press_time = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs_f64();
    let mut data = load_data()?;
    compute_time_and_counter(press_time, &mut data);
    handle_press(&mut data);

    let layout_id = data.layouts[data.cur_freq];
    let mut childs = Vec::with_capacity(data.devices.len());
    for dev_name in &data.devices {
        childs.push(switch_layout_for(dev_name.clone(), layout_id));
    }

    dump_data(data)?;

    for child in childs {
        child?.wait()?;
    }
    Ok(())
}

fn compute_time_and_counter(press_time: f64, data: &mut Data) {
    let diff = press_time - data.last_time;
    data.last_time = press_time;

    data.sum_time += diff;

    if data.sum_time < MAX_DURATION {
        data.counter += 1;
    } else {
        data.sum_time = 0.0;
        data.counter = 1;
    }

    if data.counter >= 2 {
        data.sum_time = 0.0;
    }
}

fn handle_press(data: &mut Data) {
    if data.counter <= 1 {
        data.cur_freq = (data.cur_freq + 1) % 2;
    } else {
        data.cur_all = if data.counter > 2 {
            data.cur_all + 1
        } else {
            2
        };
        data.cur_all %= data.layouts.len();

        if data.cur_all == data.cur_freq {
            data.cur_all += 1;
        }

        (data.layouts[data.cur_all], data.layouts[data.cur_freq]) =
            (data.layouts[data.cur_freq], data.layouts[data.cur_all]);
    }
}

fn switch_layout_for(
    device: String,
    layout_id: usize,
) -> Result<std::process::Child, std::io::Error> {
    Command::new("hyprctl")
        .args(["switchxkblayout", &device, &layout_id.to_string()])
        .spawn()
}

fn load_layouts_from_hyprconf() -> Result<Vec<String>, Box<dyn Error>> {
    let output = Command::new("hyprctl")
        .args(["getoption", "input:kb_layout", "-j"])
        .output()?
        .stdout;
    let data: Value = serde_json::from_slice(&output).expect("Must be captured output!");
    Ok(data["str"]
        .as_str()
        .expect("The keyboard layouts must be available!")
        .split(',')
        .map(|s| s.to_string())
        .collect())
}

fn dump_data(data: Data) -> Result<(), Box<dyn Error>> {
    let mut file = std::fs::File::create(&*DATA_STORAGE)?;
    Ok(file.write_all(
        serde_json::to_string(&data)
            .expect("Something wrong happened when serializes from Data to string")
            .as_bytes(),
    )?)
}

fn load_data() -> Result<Data, Box<dyn Error>> {
    let file = std::fs::File::open(&*DATA_STORAGE)?;
    let reader = std::io::BufReader::new(file);
    Ok(serde_json::from_reader(reader)?)
}
