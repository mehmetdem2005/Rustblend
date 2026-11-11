//! GPU-independent startup status, consumed by the Android Activity overlay.
//! Never touches project files. Atomic replacement avoids partially read messages.
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

static REPORT: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
const MAX_MESSAGE: usize = 8192;
pub fn init(directory: Option<&Path>) {
    let mut path = REPORT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    *path = directory.map(|dir| dir.join("startup-status.txt"));
    drop(path);
    stage("Rust motoru başlatılıyor");
}
pub fn stage(message: &str) {
    report("STARTING", message);
}
pub fn ready(message: &str) {
    report("READY", message);
}
pub fn error(message: &str) {
    report("ERROR", message);
}
fn report(state: &str, message: &str) {
    eprintln!("RUSTBLEND {state}: {message}");
    let path = REPORT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(path) = path.as_ref() {
        if let Err(error) = write_status(path, state, message) {
            eprintln!("RUSTBLEND startup status could not be saved: {error}");
        }
    }
}
fn write_status(path: &Path, state: &str, message: &str) -> std::io::Result<()> {
    let mut end = message.len().min(MAX_MESSAGE);
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, format!("{state}\n{}", &message[..end]))?;
    std::fs::rename(temp, path)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostics_replace_previous_state_and_bound_utf8_without_touching_projects() {
        let dir = std::env::temp_dir().join(format!("rustblend-startup-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sentinel = dir.join("project.rblend");
        std::fs::write(&sentinel, b"preserve").unwrap();
        let path = dir.join("startup-status.txt");
        write_status(&path, "ERROR", &"ş".repeat(6000)).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("ERROR\n"));
        assert!(text.len() <= MAX_MESSAGE + 6);
        write_status(&path, "READY", "GLES").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "READY\nGLES");
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"preserve");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
