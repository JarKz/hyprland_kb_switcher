use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use hyprland::{
    ctl::switch_xkb_layout,
    data::Devices,
    keyword::{Keyword, OptionValue},
    shared::HyprData,
    Result,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{future::Future, io::Write, path::PathBuf, time::UNIX_EPOCH};

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
    Init { devices: Vec<String> },

    /// Updates layouts in the data file without other actions.
    ///
    /// Uses, when you change the layout set in hyprland.conf.
    UpdateLayouts,

    /// Switches the keyboard layouts like MacOS.
    ///
    /// Switches the layouts for all devices, which you added in
    /// 'init' or 'add-device' command.
    Switch,

    /// Adds a device into the data file.
    ///
    /// Note: the device name must be correct, otherwise it won't add's into file.
    /// You can get the name using command 'hyprctl devices'.
    AddDevice { device_name: String },

    /// Removes matching device from the data file.
    ///
    /// You get the device name using command 'list-devices'.
    RemoveDevice { device_name: String },

    /// Prints all stored device names.
    ListDevices,

    /// Generate shell completion script;
    Completion { shell: Option<Shell> },
}

impl KbSwitcherCmd {
    pub async fn process(&self) -> Result<()> {
        match self {
            KbSwitcherCmd::Init { devices } => init(devices).await,
            KbSwitcherCmd::UpdateLayouts => update_layouts().await,
            KbSwitcherCmd::Switch => switch().await,
            KbSwitcherCmd::AddDevice { device_name } => add_device(device_name).await,
            KbSwitcherCmd::RemoveDevice { device_name } => remove_device(device_name),
            KbSwitcherCmd::ListDevices => list_devices(),
            KbSwitcherCmd::Completion { shell } => {
                print_completion(shell);
                Ok(())
            }
        }
    }
}

async fn init(devices: &[String]) -> Result<()> {
    let future_layouts = Keyword::get_async("input:kb_layout");
    let time = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("UNIX epoch must be earlier than current time!")
        .as_secs_f64();
    std::fs::create_dir_all(&*DATA_PATH)?;

    let layouts = load_layouts_from_hyprconf(future_layouts).await?;

    let data = Data {
        devices: devices.to_owned(),
        last_time: time,
        layouts: (0..layouts.len()).collect(),
        cur_freq: 0,
        cur_all: 0,
        sum_time: 0.0,
        counter: 0,
    };

    dump_data(data)?;
    Ok(())
}

async fn update_layouts() -> Result<()> {
    let future_layouts = Keyword::get_async("input:kb_layout");
    let mut data = load_data()?;

    let layouts = load_layouts_from_hyprconf(future_layouts).await?;
    data.layouts = (0..layouts.len()).collect();
    dump_data(data)?;
    Ok(())
}

async fn switch() -> Result<()> {
    let press_time = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("UNIX epoch must be earlier than current time!")
        .as_secs_f64();
    let mut data = load_data()?;
    compute_time_and_counter(press_time, &mut data);
    handle_press(&mut data);

    let layout_id = data.layouts[data.cur_freq];
    let mut processes = vec![];
    for keyboard in Devices::get()?
        .keyboards
        .into_iter()
        .filter(|keyboard| data.devices.contains(&keyboard.name))
    {
        let data = switch_xkb_layout::SwitchXKBLayoutCmdTypes::Id(layout_id as u8);
        processes.push(switch_xkb_layout::call_async(keyboard.name, data));
    }

    dump_data(data)?;

    for process in processes {
        process.await?;
    }
    Ok(())
}

async fn add_device(device_name: &String) -> Result<()> {
    let future_devices = Devices::get_async();
    let mut data = load_data()?;

    let available_keyboards = future_devices.await?.keyboards;

    if !available_keyboards
        .iter()
        .any(|keyboard| keyboard.name == *device_name)
    {
        eprintln!(
            "The given keyboard name is incorrect! Available keyboards: {}",
            available_keyboards
                .iter()
                .map(|keyboard| "\n- ".to_string() + &keyboard.name)
                .collect::<String>()
        );
        std::process::exit(1);
    }

    data.devices.push(device_name.clone());
    dump_data(data)?;
    Ok(())
}

fn remove_device(device_name: &String) -> Result<()> {
    let mut data = load_data()?;

    if let Some((i, _)) = data
        .devices
        .iter()
        .enumerate()
        .find(|(_, dev)| *dev == device_name)
    {
        data.devices.remove(i);
        dump_data(data)?;
    }
    Ok(())
}

fn list_devices() -> Result<()> {
    let data = load_data()?;
    println!(
        "Current stored devices:{}",
        data.devices
            .iter()
            .map(|device| "\n - ".to_string() + device)
            .collect::<String>()
    );
    Ok(())
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

async fn load_layouts_from_hyprconf(
    future_layouts: impl Future<Output = Result<Keyword>>,
) -> Result<Vec<String>> {
    match future_layouts.await?.value {
        OptionValue::String(s) => Ok(s.split(',').map(|layout| layout.to_string()).collect()),
        _ => {
            eprintln!("Something went wrong during getting option input:kb_layout. The given value is another than String type. Please check your config and report it to developer.");
            std::process::exit(1);
        }
    }
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
