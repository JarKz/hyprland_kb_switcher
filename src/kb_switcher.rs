use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{io::Write, path::PathBuf, process::Command, time::UNIX_EPOCH};

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

/// Simple program, which can switch keyboard layout more comfotrable
/// in Hyprland, like on MacOS.
#[derive(Parser, Debug)]
#[command(version, about, name = env!("CARGO_PKG_NAME"))]
pub enum KbSwitcherCmd {
    /// Initializes storage data with device names.
    ///
    /// Also captures current time, loads layouts from hyprland.conf,
    /// and stores in file named "data", which is placed at
    /// $XDG_DATA_HOME/kb_switcher/data or $HOME/.local/share/kb_switcher/data.
    ///
    /// Must be called before all the other commands!
    Init {
        devices: Vec<String>,
    },

    /// Updates layouts in the data file without other actions.
    ///
    /// Uses, when you change the layout set in hyprland.conf.
    UpdateLayouts,

    /// Switches the keyboard layouts like MacOS.
    ///
    /// Switches the layouts for all devices, which you added in
    /// 'init' or 'add-device' command.
    Switch,

    /// Adds a device into data file.
    ///
    /// Note: the device name must be correct. You can get the name
    /// using command 'hyprctl devices'.
    AddDevice {
        device_name: String,
    },

    /// Removes matching device from data file.
    ///
    /// Note: the device must be correct. You can get the name from
    /// file, which is placed at $XDG_DATA_HOME/kb_switcher/data
    /// or $HOME/.local/share/kb_switcher/data.
    RemoveDevice {
        device_name: String,
    },

    /// Generate shell completion script
    Completion {
        shell: Option<Shell>,
    },
}

impl KbSwitcherCmd {
    pub fn process(&self) -> std::io::Result<()> {
        match self {
            KbSwitcherCmd::Init { devices } => init(devices),
            KbSwitcherCmd::UpdateLayouts => update_layouts(),
            KbSwitcherCmd::Switch => switch(),
            KbSwitcherCmd::AddDevice { device_name } => add_device(device_name),
            KbSwitcherCmd::RemoveDevice { device_name } => remove_device(device_name),
            KbSwitcherCmd::Completion { shell } => {
                print_completion(shell);
                Ok(())
            }
        }
    }
}

fn init(devices: &[String]) -> std::io::Result<()> {
    let layouts = load_layouts_from_hyprconf()?;
    let time = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("UNIX epoch must be earlier than current time!")
        .as_secs_f64();

    let data = Data {
        devices: devices.to_owned(),
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

fn update_layouts() -> std::io::Result<()> {
    let layouts = load_layouts_from_hyprconf()?;
    let mut data = load_data()?;
    data.layouts = (0..layouts.len()).collect();
    dump_data(data)
}

fn switch() -> std::io::Result<()> {
    let press_time = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("UNIX epoch must be earlier than current time!")
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

fn add_device(device_name: &String) -> std::io::Result<()> {
    let mut data = load_data()?;
    let available_keyboards = load_keyboards()?;

    if !available_keyboards.contains(device_name) {
        eprintln!(
            "The given keyboard name is incorrect! Available keyboards: {}",
            available_keyboards
                .iter()
                .map(|keyboard| "\n- ".to_string() + keyboard)
                .collect::<String>()
        );
        std::process::exit(1);
    }

    data.devices.push(device_name.clone());
    dump_data(data)
}

fn remove_device(device_name: &String) -> std::io::Result<()> {
    let mut data = load_data()?;

    if let Some((i, _)) = data
        .devices
        .iter()
        .enumerate()
        .find(|(_, dev)| *dev == device_name)
    {
        data.devices.remove(i);
    }

    dump_data(data)
}

fn print_completion(shell: &Option<Shell>) {
    let mut cmd = KbSwitcherCmd::command();
    let name = cmd.get_name().to_string();
    generate(
        shell.unwrap_or(Shell::Bash),
        &mut cmd,
        name,
        &mut std::io::stdout(),
    );
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

fn load_layouts_from_hyprconf() -> std::io::Result<Vec<String>> {
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

fn load_keyboards() -> std::io::Result<Vec<String>> {
    let output = Command::new("hyprctl")
        .args(["devices", "-j"])
        .output()?
        .stdout;
    let data: Value = serde_json::from_slice(&output).expect("Must be captured output!");
    let keyboards: Vec<String> = data["keyboards"]
        .as_array()
        .expect("Must be array of keyboards!")
        .iter()
        .map(|keyboard| {
            keyboard["name"]
                .as_str()
                .expect("The keyboard name must be string!")
                .to_string()
        })
        .collect();
    Ok(keyboards)
}

fn dump_data(data: Data) -> std::io::Result<()> {
    let mut file = std::fs::File::create(&*DATA_STORAGE)?;
    file.write_all(
        serde_json::to_string(&data)
            .expect("Something wrong happened when serializes from Data to string")
            .as_bytes(),
    )
}

fn load_data() -> std::io::Result<Data> {
    let file = match std::fs::File::open(&*DATA_STORAGE) {
        Ok(file) => file,
        Err(error) => match error.kind() {
            std::io::ErrorKind::NotFound => {
                eprintln!(
                    "File at {} doesn't exists!\nMaybe you need to initialize data using command 'init'.",
                    DATA_STORAGE.to_string_lossy()
                );
                std::process::exit(1);
            }
            _ => return Err(error),
        },
    };
    let reader = std::io::BufReader::new(file);
    Ok(serde_json::from_reader(reader)?)
}
