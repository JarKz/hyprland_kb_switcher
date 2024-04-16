use once_cell::sync::Lazy;
use std::{path::PathBuf, io::Write};

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

pub fn init() -> std::io::Result<()> {
    std::fs::create_dir_all(&*DATA_PATH)
}

pub fn dump(data: super::Data) -> std::io::Result<()> {
    let mut file = std::fs::File::create(&*DATA_STORAGE)?;
    file.write_all(
        serde_json::to_string(&data)
            .expect("Something wrong happened when serializes from Data to string")
            .as_bytes(),
    )
}

pub fn load() -> std::io::Result<super::Data> {
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
