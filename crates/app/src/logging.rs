use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

static LOG_PATH: Mutex<Option<String>> = Mutex::new(None);

pub fn init(dir: &Path) {
    *LOG_PATH.lock().unwrap() = Some(
        dir.join("ai-token-stats.log")
            .to_string_lossy()
            .into_owned(),
    );
    log_msg("log initialized");
}

pub fn log_msg(msg: &str) {
    let path = LOG_PATH.lock().unwrap().clone();
    let Some(path) = path else { return };
    let line = format!(
        "[{}] {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        msg
    );
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(line.as_bytes());
    }
}
