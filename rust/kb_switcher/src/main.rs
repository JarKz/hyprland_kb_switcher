use clap::Parser;
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
    data_path.push("layout_switcher");
    data_path
});

static DATA_STORAGE: Lazy<PathBuf> = Lazy::new(|| {
    let mut other_path = DATA_PATH.clone();
    other_path.push("data");
    other_path
});

#[derive(Serialize, Deserialize)]
struct Data {
    last_time: f64,
    layouts: Vec<usize>,
    cur_freq: usize,
    cur_all: usize,
    sum_time: f64,
    counter: u8,
}

#[derive(Parser, Debug)]
#[command(version, about)]
struct KbSwitcherCmd {
    name: Option<String>,
    device_name: Option<String>,
}

impl KbSwitcherCmd {
    fn process(&mut self) -> Result<(), Box<dyn Error>> {
        match self.name.as_ref().map(|s| s.as_str()) {
            Some("init") => return init(),
            Some("switch") => {}
            None => {}
            other => {}
        }
        Ok(())
    }
}

fn init() -> Result<(), Box<dyn Error>> {
    let layouts = load_layouts_from_hyprconf()?;
    let time = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs_f64();

    let data = Data {
        last_time: time,
        layouts: (0..layouts.len()).collect(),
        cur_freq: 0,
        cur_all: 0,
        sum_time: 0.0,
        counter: 0,
    };

    init_path()?;
    let mut file = std::fs::File::create(&*DATA_STORAGE)?;
    file.write_all(
        serde_json::to_string(&data)
            .expect("Something wrong happened when serializes from Data to string")
            .as_bytes(),
    )?;

    Ok(())
}

fn load_layouts_from_hyprconf() -> Result<Vec<String>, Box<dyn Error>> {
    let output = Command::new("hyprctl")
        .args(&["getoption", "input:kb_layout", "-j"])
        .output()?
        .stdout;
    let data: Value = serde_json::from_slice(&output).expect("Must be captured output!");
    Ok(data["str"]
        .as_str()
        .expect("The keyboard layouts must be available!")
        .split(",")
        .map(|s| s.to_string())
        .collect())
}

fn init_path() -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(&*DATA_PATH)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut command = KbSwitcherCmd::parse();
    command.process()?;
    Ok(())
}
