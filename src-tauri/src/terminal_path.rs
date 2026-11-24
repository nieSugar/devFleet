use std::{env, ffi::OsString, io};
use windows::{
    core::w,
    Win32::{
        Foundation::{LPARAM, WPARAM},
        UI::WindowsAndMessaging::{
            SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
        },
    },
};
use winreg::{
    enums::{HKEY_CURRENT_USER, REG_EXPAND_SZ, REG_SZ},
    types::{FromRegValue, ToRegValue},
    RegKey,
};

pub fn installer_command() -> Option<i32> {
    let mut args = env::args_os().skip(1);
    let install = match args.next()?.to_str()? {
        "--install-terminal-path" => true,
        "--uninstall-terminal-path" => false,
        _ => return None,
    };
    if args.next().is_some() {
        return None;
    }

    Some(match update_user_path(install) {
        Ok(false) if install => 2, // The installer does not own an existing entry.
        Ok(_) => 0,
        Err(error) => {
            eprintln!("[devfleet] failed to update terminal command PATH: {error}");
            1
        }
    })
}

fn updated_path(current: &str, dir: &str, install: bool) -> Option<String> {
    let matches = |entry: &str| {
        entry
            .trim_matches('"')
            .trim_end_matches(['\\', '/'])
            .eq_ignore_ascii_case(dir.trim_end_matches(['\\', '/']))
    };
    let present = current.split(';').any(matches);
    match (install, present) {
        (true, false) if current.is_empty() => Some(dir.to_owned()),
        (true, false) => Some(format!("{current};{dir}")),
        (false, true) => Some(
            current
                .split(';')
                .filter(|entry| !matches(entry))
                .collect::<Vec<_>>()
                .join(";"),
        ),
        _ => None,
    }
}

fn update_user_path(install: bool) -> io::Result<bool> {
    let bin_dir = env::current_exe()?.with_file_name("bin");
    let dir = bin_dir
        .to_str()
        .filter(|path| !path.contains(';'))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid install directory"))?;

    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey("Environment")?;
    let (current, value_type) = match key.get_raw_value("Path") {
        Ok(raw) if matches!(raw.vtype, REG_SZ | REG_EXPAND_SZ) => {
            let current = OsString::from_reg_value(&raw)?.into_string().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid text in user PATH")
            })?;
            (current, raw.vtype)
        }
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid user PATH type",
            ))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => (String::new(), REG_SZ),
        Err(error) => return Err(error),
    };
    let Some(updated) = updated_path(&current, dir, install) else {
        return Ok(false);
    };
    let mut value = updated.to_reg_value();
    value.vtype = value_type;
    key.set_raw_value("Path", &value)?;

    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(w!("Environment").as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            100,
            None,
        );
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::updated_path;

    #[test]
    fn terminal_path_is_added_once_and_removed_without_changing_other_entries() {
        let dir = r"C:\Dev Tools\DevFleet\bin";
        let original = r"%USERPROFILE%\bin;C:\Other Tools\bin;C:\Dev Tools\DevFleet";
        let added = updated_path(original, dir, true).unwrap();
        assert_eq!(added, format!("{original};{dir}"));
        assert_eq!(updated_path(&added, dir, true), None);
        assert_eq!(updated_path(&added, dir, false).as_deref(), Some(original));
        assert_eq!(
            updated_path(r#""c:\dev tools\devfleet\bin\""#, dir, true),
            None
        );
        assert_eq!(updated_path("", dir, true).as_deref(), Some(dir));
        assert_eq!(updated_path(dir, dir, false).as_deref(), Some(""));
    }
}
