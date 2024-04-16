use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use hyprland::{
    ctl::switch_xkb_layout,
    data::Devices,
    keyword::{Keyword, OptionValue},
    shared::HyprData,
    Result,
};

use serde::{Deserialize, Serialize};
use std::{future::Future, time::UNIX_EPOCH};

mod data;

#[derive(Serialize, Deserialize)]
struct Data {
    devices: Vec<String>,
    last_time: f64,
    layouts: Vec<usize>,
    cur_freq: usize,
    cur_all: usize,
    sum_time: f64,
    counter: u8,

    #[serde(default)]
    max_duration: Duration,
}

#[derive(Serialize, Deserialize)]
struct Duration(f64);

impl Duration {
    const DEFAULT_MAX_DURATION: f64 = 0.4;
    const MIN: f64 = 0.2;
    const MAX: f64 = 1.0;

    fn satisfies(&self, time: f64) -> bool {
        time < self.0
    }

    fn valid(time: f64) -> bool {
        (Self::MIN..=Self::MAX).contains(&time)
    }
}

impl Default for Duration {
    fn default() -> Self {
        Duration(Self::DEFAULT_MAX_DURATION)
    }
}

impl std::fmt::Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
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

    /// Subcommand for managing devices.
    #[command(subcommand)]
    Device(DeviceCmd),

    /// Updates layouts in the data file without other actions.
    ///
    /// Uses, when you change the layout set in hyprland.conf.
    UpdateLayouts,

    /// Switches the keyboard layouts like MacOS.
    ///
    /// Switches the layouts for all devices, which you added in
    /// 'init' or 'device add' command.
    Switch,

    /// The keypress duration between two presses for activating 'switch'.
    ///
    /// 'Between two presses' means from first press and third press, after which turning to
    /// another (not last used) layout.
    ///
    /// To set use a value from range [0.2; 1.0] (in seconds), where brackets means inclusive range.
    /// Incorrect value will be immediately declined.
    ///
    /// To print just call command without values.
    KeypressDuration {
        duration: Option<f64>,
    },

    /// Generate shell completion script;
    Completion { shell: Option<Shell> },
}

/// Subcommand for managing devices.
#[derive(Subcommand, Debug)]
pub enum DeviceCmd {

    /// Prints all stored device names.
    List,

    /// Adds a device into the data file.
    ///
    /// Note: the device name must be correct, otherwise it won't add's into file.
    /// You can get the name using command 'hyprctl devices'.
    Add {
        device_name: String
    },

    /// Removes matching device from the data file.
    ///
    /// You get the device name using command 'devices list'.
    Remove {
        device_name: String
    }
}

impl DeviceCmd {
    pub async fn handle(&self) -> Result<()> {
        match self {
            DeviceCmd::List => list_devices(),
            DeviceCmd::Add { device_name } => add_device(device_name).await,
            DeviceCmd::Remove { device_name } => remove_device(device_name),
        }
    }
}

impl KbSwitcherCmd {
    pub async fn handle(&self) -> Result<()> {
        match self {
            KbSwitcherCmd::Init { devices } => init(devices).await,
            KbSwitcherCmd::UpdateLayouts => update_layouts().await,
            KbSwitcherCmd::Switch => switch().await,
            KbSwitcherCmd::Device(cmd) => cmd.handle().await,
            KbSwitcherCmd::KeypressDuration { duration } => handle_keypress_duration(duration),
            KbSwitcherCmd::Completion { shell } => {
                print_completion(shell);
                Ok(())
            }
        }
    }
}

async fn init(devices: &[String]) -> Result<()> {
    let future_layouts = Keyword::get_async("input:kb_layout");
    let available_devices = Devices::get_async();
    let time = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("UNIX epoch must be earlier than current time!")
        .as_secs_f64();
    data::init()?;

    let layouts = load_layouts_from_hyprconf(future_layouts).await?;

    let available_keyboards: std::collections::HashSet<String> = available_devices
        .await?
        .keyboards
        .into_iter()
        .map(|kb| kb.name)
        .collect();

    let mut used_devices = vec![];
    for device in devices {
        if available_keyboards.contains(device) {
            used_devices.push(device.to_owned());
            continue;
        }

        eprintln!("The keyboard name is invalid: {} (skipped).\n", device);
    }

    let data = Data {
        devices: used_devices,
        last_time: time,
        layouts: (0..layouts.len()).collect(),
        cur_freq: 0,
        cur_all: 0,
        sum_time: 0.0,
        counter: 0,
        max_duration: Default::default(),
    };

    data::dump(data)?;
    Ok(())
}

async fn update_layouts() -> Result<()> {
    let future_layouts = Keyword::get_async("input:kb_layout");
    let mut data = data::load()?;

    let layouts = load_layouts_from_hyprconf(future_layouts).await?;
    data.layouts = (0..layouts.len()).collect();
    data::dump(data)?;
    Ok(())
}

async fn switch() -> Result<()> {
    let future_devices = Devices::get_async();

    let press_time = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("UNIX epoch must be earlier than current time!")
        .as_secs_f64();
    let mut data = data::load()?;
    compute_time_and_counter(press_time, &mut data);
    handle_press(&mut data);

    let layout_id = data.layouts[data.cur_freq];
    let mut processes = vec![];
    for keyboard in future_devices
        .await?
        .keyboards
        .into_iter()
        .filter(|keyboard| data.devices.contains(&keyboard.name))
    {
        let data = switch_xkb_layout::SwitchXKBLayoutCmdTypes::Id(layout_id as u8);
        processes.push(switch_xkb_layout::call_async(keyboard.name, data));
    }

    data::dump(data)?;

    for process in processes {
        process.await?;
    }
    Ok(())
}

async fn add_device(device_name: &String) -> Result<()> {
    let future_devices = Devices::get_async();
    let mut data = data::load()?;

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
    data::dump(data)?;
    Ok(())
}

fn remove_device(device_name: &String) -> Result<()> {
    let mut data = data::load()?;

    if let Some((i, _)) = data
        .devices
        .iter()
        .enumerate()
        .find(|(_, dev)| *dev == device_name)
    {
        data.devices.remove(i);
        data::dump(data)?;
    }
    Ok(())
}

fn list_devices() -> Result<()> {
    let data = data::load()?;
    println!(
        "Current stored devices:{}",
        data.devices
            .iter()
            .map(|device| "\n - ".to_string() + device)
            .collect::<String>()
    );
    Ok(())
}

fn handle_keypress_duration(duration: &Option<f64>) -> Result<()> {
    match duration {
        Some(duration) => set_keypress_duration(duration),
        None => print_keypress_duration(),
    }
}

fn set_keypress_duration(&duration: &f64) -> Result<()> {
    if !Duration::valid(duration) {
        eprintln!("The selected keypress duration is too strange! Please, set a number from range [0.2, 1.0].\nYour selected duration: {}", duration);
        std::process::exit(1);
    }
    let mut data = data::load()?;
    data.max_duration = Duration(duration);
    Ok(data::dump(data)?)
}

fn print_keypress_duration() -> Result<()> {
    let data = data::load()?;
    println!("The current max keypress duration: {}", data.max_duration);
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

    if data.max_duration.satisfies(data.sum_time) {
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
