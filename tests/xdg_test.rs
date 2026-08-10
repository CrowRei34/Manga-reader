// tests/xdg_test.rs
use std::path::PathBuf;
use bakeneko::core::xdg::Xdg;

#[test]
fn data_home_uses_env() {
    temp_env::with_var("XDG_DATA_HOME", Some("/tmp/xdgtest/data"), || {
        assert_eq!(Xdg::data_home(), PathBuf::from("/tmp/xdgtest/data"));
    });
}

#[test]
fn data_home_defaults_to_local_share() {
    temp_env::with_vars(
        [
            ("XDG_DATA_HOME", Some("")),
            ("HOME", Some("/tmp/homedir")),
        ],
        || {
            assert_eq!(Xdg::data_home(), PathBuf::from("/tmp/homedir/.local/share"));
        },
    );
}

#[test]
fn daemon_socket_under_runtime_bakeneko() {
    temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/1000"), || {
        assert_eq!(
            Xdg::daemon_socket(),
            PathBuf::from("/run/user/1000/bakeneko/daemon.sock")
        );
    });
}

#[test]
fn data_root_appends_bakeneko() {
    temp_env::with_var("XDG_DATA_HOME", Some("/tmp/xdgtest/data"), || {
        assert_eq!(Xdg::data_root(), PathBuf::from("/tmp/xdgtest/data/bakeneko"));
    });
}
