//! How this copy of SimpleWorship was installed, and so who updates it.
//!
//! Copies from GitHub (the NSIS installer, the macOS and Linux bundles)
//! update themselves through the updater plugin. A copy from the Microsoft
//! Store must not: the Store updates it, Store policy forbids apps that
//! update themselves from elsewhere, and running the NSIS installer from a
//! Store copy would put a second, separate SimpleWorship on the computer
//! (Store apps keep their settings in their own package folder).
//!
//! Detection also includes sideloaded MSIX packages. The repository has no
//! supported MSIX update path, so all packaged copies disable the app's
//! updater and show neutral guidance to update through their install source.

use std::path::Path;
use std::sync::OnceLock;

/// Whether this copy is packaged as MSIX, including sideloaded packages.
/// The name is historical: package detection cannot establish Store origin.
/// The app's own updater stays off for all packaged copies. Always `false`
/// outside Windows. Worked out once, then remembered.
pub fn installed_from_store() -> bool {
    static FROM_STORE: OnceLock<bool> = OnceLock::new();
    *FROM_STORE.get_or_init(|| {
        let from_store = cfg!(windows)
            && std::env::current_exe()
                .map(|exe| is_packaged_exe(&exe))
                .unwrap_or(false);
        if from_store {
            log::info!("MSIX package detected: update through the original install source");
        }
        from_store
    })
}

/// Whether `exe` runs from inside an MSIX package.
///
/// Windows puts packaged apps under `…\WindowsApps\<package>\`, and every
/// package has its `AppxManifest.xml` at the root, next to our executable
/// (see `scripts/package-msix.ps1`). Either sign is enough. This avoids
/// calling `GetCurrentPackageFullName`, which would need `unsafe` code.
fn is_packaged_exe(exe: &Path) -> bool {
    let in_windows_apps = exe
        .components()
        .any(|part| part.as_os_str().eq_ignore_ascii_case("WindowsApps"));
    let has_manifest = exe
        .parent()
        .is_some_and(|folder| folder.join("AppxManifest.xml").is_file());
    in_windows_apps || has_manifest
}

/// Whether this copy is packaged, for the web UI: it hides Check for Updates
/// and the automatic-update setting, and shows install-source guidance.
#[tauri::command]
pub fn updates_from_store() -> bool {
    installed_from_store()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_inside_windows_apps_is_packaged() {
        let exe = Path::new(
            "C:/Program Files/WindowsApps/SimpleWorship_0.5.3.0_x64__abc/SimpleWorship.exe",
        );
        assert!(is_packaged_exe(exe));
    }

    #[test]
    fn ordinary_install_is_not_packaged() {
        let exe = Path::new("C:/Program Files/SimpleWorship/SimpleWorship.exe");
        assert!(!is_packaged_exe(exe));
    }

    #[test]
    fn exe_next_to_a_package_manifest_is_packaged() {
        let folder = std::env::temp_dir().join(format!("sw-msix-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("AppxManifest.xml"), "<Package/>").unwrap();
        let packaged = is_packaged_exe(&folder.join("SimpleWorship.exe"));
        std::fs::remove_dir_all(&folder).unwrap();
        assert!(packaged);
    }
}
