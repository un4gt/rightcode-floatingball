#[cfg(target_os = "macos")]
use std::path::PathBuf;

#[cfg(windows)]
const WINDOWS_RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const WINDOWS_VALUE_NAME: &str = "RightCodeFloatingBall";

#[cfg(target_os = "macos")]
const MACOS_LAUNCH_AGENT_LABEL: &str = "codes.rightcode.floatingball";

pub fn is_supported() -> bool {
    cfg!(any(windows, target_os = "macos"))
}

pub fn is_enabled() -> Result<bool, String> {
    #[cfg(windows)]
    return windows_is_enabled();

    #[cfg(target_os = "macos")]
    return macos_is_enabled();

    #[cfg(not(any(windows, target_os = "macos")))]
    return Ok(false);
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    return windows_set_enabled(enabled);

    #[cfg(target_os = "macos")]
    return macos_set_enabled(enabled);

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = enabled;
        Ok(())
    }
}

#[cfg(windows)]
fn windows_set_enabled(enabled: bool) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ, RegCloseKey,
        RegDeleteValueW, RegOpenKeyExW, RegSetValueExW,
    };

    fn wide_null(value: &str) -> Vec<u16> {
        OsStr::new(value)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    let subkey = wide_null(WINDOWS_RUN_KEY);
    let value_name = wide_null(WINDOWS_VALUE_NAME);

    unsafe {
        let mut key: HKEY = std::ptr::null_mut();
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_SET_VALUE | KEY_QUERY_VALUE,
            &mut key,
        );

        if status != ERROR_SUCCESS {
            return Err(format!("RegOpenKeyExW failed: {status}"));
        }

        let result = if enabled {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let command = format!("\"{}\"", exe.display());
            let data = wide_null(&command);

            let set_status = RegSetValueExW(
                key,
                value_name.as_ptr(),
                0,
                REG_SZ,
                data.as_ptr().cast(),
                (data.len() * 2) as u32,
            );

            if set_status != ERROR_SUCCESS {
                Err(format!("RegSetValueExW failed: {set_status}"))
            } else {
                Ok(())
            }
        } else {
            let delete_status = RegDeleteValueW(key, value_name.as_ptr());
            if delete_status == ERROR_SUCCESS || delete_status == ERROR_FILE_NOT_FOUND {
                Ok(())
            } else {
                Err(format!("RegDeleteValueW failed: {delete_status}"))
            }
        };

        let _ = RegCloseKey(key);
        result
    }
}

#[cfg(windows)]
fn windows_is_enabled() -> Result<bool, String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
    };

    fn wide_null(value: &str) -> Vec<u16> {
        OsStr::new(value)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    let subkey = wide_null(WINDOWS_RUN_KEY);
    let value_name = wide_null(WINDOWS_VALUE_NAME);

    unsafe {
        let mut key: HKEY = std::ptr::null_mut();
        let open_status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut key,
        );

        if open_status != ERROR_SUCCESS {
            return Ok(false);
        }

        let mut size: u32 = 0;
        let query_status = RegQueryValueExW(
            key,
            value_name.as_ptr(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut size,
        );

        let _ = RegCloseKey(key);

        Ok(query_status == ERROR_SUCCESS && size > 0)
    }
}

#[cfg(target_os = "macos")]
fn macos_launch_agent_path() -> Result<PathBuf, String> {
    let base = directories::BaseDirs::new().ok_or("unable to resolve home directory")?;
    Ok(base
        .home_dir()
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{MACOS_LAUNCH_AGENT_LABEL}.plist")))
}

#[cfg(target_os = "macos")]
fn macos_is_enabled() -> Result<bool, String> {
    Ok(macos_launch_agent_path()?.exists())
}

#[cfg(target_os = "macos")]
fn macos_set_enabled(enabled: bool) -> Result<(), String> {
    let path = macos_launch_agent_path()?;

    if !enabled {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_str().ok_or("current exe path is not valid utf-8")?;

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{MACOS_LAUNCH_AGENT_LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
</dict>
</plist>
"#
    );

    std::fs::write(&path, plist).map_err(|e| e.to_string())?;
    Ok(())
}
