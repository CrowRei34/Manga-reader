use std::env;
use std::fs;
use std::path::PathBuf;

pub struct Xdg;

impl Xdg {
    pub fn home() -> String {
        env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
    }

    fn dir_or_default(var: &str, default: PathBuf) -> PathBuf {
        match env::var(var) {
            Ok(v) if !v.is_empty() => PathBuf::from(v),
            _ => default,
        }
    }

    pub fn data_home() -> PathBuf {
        Self::dir_or_default("XDG_DATA_HOME", PathBuf::from(format!("{}/.local/share", Self::home())))
    }
    pub fn config_home() -> PathBuf {
        Self::dir_or_default("XDG_CONFIG_HOME", PathBuf::from(format!("{}/.config", Self::home())))
    }
    pub fn cache_home() -> PathBuf {
        Self::dir_or_default("XDG_CACHE_HOME", PathBuf::from(format!("{}/.cache", Self::home())))
    }
    pub fn runtime_dir() -> PathBuf {
        Self::dir_or_default("XDG_RUNTIME_DIR", PathBuf::from(format!("/tmp/bakeneko-{}", Self::uid())))
    }

    pub fn data_root() -> PathBuf { Self::data_home().join("bakeneko") }
    pub fn config_root() -> PathBuf { Self::config_home().join("bakeneko") }
    pub fn cache_root() -> PathBuf { Self::cache_home().join("bakeneko") }
    pub fn daemon_socket() -> PathBuf { Self::runtime_dir().join("bakeneko").join("daemon.sock") }
    pub fn downloads_root() -> PathBuf { Self::data_root().join("downloads") }

    pub fn uid() -> i64 {
        // Espejo del Dart: lee /proc/self/status Uid:, fallback 1000.
        let Ok(contents) = fs::read_to_string("/proc/self/status") else {
            return 1000;
        };
        for line in contents.lines() {
            if let Some(rest) = line.strip_prefix("Uid:") {
                if let Some(first) = rest.split_whitespace().next() {
                    return first.parse().unwrap_or(1000);
                }
            }
        }
        1000
    }

    pub fn ensure_dirs() -> std::io::Result<()> {
        for d in [Self::data_root(), Self::config_root(), Self::cache_root(), Self::downloads_root()] {
            fs::create_dir_all(d)?;
        }
        fs::create_dir_all(Self::runtime_dir().join("bakeneko"))?;
        Ok(())
    }
}
