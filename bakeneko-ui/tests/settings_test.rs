// tests/settings_test.rs
use bakeneko::core::settings::{load, save, Settings, DISCORD_APPLICATION_ID};
use std::path::PathBuf;

#[test]
fn roundtrip_preserves_values() {
    temp_env::with_var("XDG_CONFIG_HOME", Some("/tmp/settest/config"), || {
        let s = Settings {
            theme: "light".into(), accent: "#ff0000".into(),
            default_source: Some("MANGADEX".into()), download_concurrency: 4,
            library_view: "list".into(), discord_client_id: "123456".into(),
            reader_mode: "paginated".into(), reader_filter: "sepia".into(),
            discord_presence_enabled: true, discord_show_adult: false,
        };
        save(&s).unwrap();
        let loaded = load();
        assert_eq!(loaded.theme, "light");
        assert_eq!(loaded.accent, "#ff0000");
        assert_eq!(loaded.default_source, Some("MANGADEX".to_string()));
        assert_eq!(loaded.download_concurrency, 4);
        assert_eq!(loaded.library_view, "list");
        assert_eq!(loaded.reader_mode, "paginated");
        assert_eq!(loaded.reader_filter, "sepia");
        assert_eq!(loaded.discord_client_id, DISCORD_APPLICATION_ID);
        assert!(loaded.discord_presence_enabled);
    });
}

#[test]
fn missing_file_yields_default() {
    temp_env::with_var("XDG_CONFIG_HOME", Some("/tmp/settest/empty"), || {
        let s = load();
        assert_eq!(s.theme, "dark");
        assert_eq!(s.download_concurrency, 2);
        assert_eq!(s.discord_client_id, DISCORD_APPLICATION_ID);
        assert!(s.discord_presence_enabled);
    });
}

#[test]
fn corrupt_file_yields_default() {
    temp_env::with_var("XDG_CONFIG_HOME", Some("/tmp/settest/corrupt"), || {
        std::fs::create_dir_all(PathBuf::from("/tmp/settest/corrupt/bakeneko")).unwrap();
        std::fs::write(PathBuf::from("/tmp/settest/corrupt/bakeneko/settings.json"), "{not json").unwrap();
        let s = load();
        assert_eq!(s.theme, "dark");
    });
}
