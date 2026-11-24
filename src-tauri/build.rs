fn main() {
    for name in [
        "APPTERISK_UPDATER_PUBKEY",
        "APPTERISK_STABLE_ENDPOINT",
        "APPTERISK_BETA_ENDPOINT",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "open_probe",
            "set_probe_bounds",
            "set_probe_visibility",
            "reload_probe",
            "capture_profile_metadata",
            "resume_probe_on_startup",
            "account_snapshot",
            "account_add",
            "account_update",
            "account_reorder",
            "account_switch",
            "account_layout",
            "account_reload",
            "account_remove",
            "settings_save",
            "restart_mode",
            "accounts_initialize",
            "desktop_snapshot",
            "desktop_save",
            "desktop_lock",
            "desktop_unlock",
            "desktop_password",
            "desktop_download_directory",
            "desktop_download_cancel",
            "desktop_permission",
            "desktop_permission_reply",
            "update_snapshot",
            "update_preferences",
            "update_check",
            "update_download",
            "update_cancel",
            "update_install",
        ]),
    ))
    .expect("Tauri permission generation failed")
}
