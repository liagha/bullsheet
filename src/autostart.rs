pub(crate) struct Autostart {
    executable: std::path::PathBuf,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    entry: std::path::PathBuf,
}

#[cfg(target_os = "windows")]
const AUTOSTART_VALUE: &str = "Calculator";

#[cfg(target_os = "windows")]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

#[cfg(target_os = "windows")]
impl Autostart {
    pub(crate) fn new() -> std::io::Result<Self> {
        Ok(Self {
            executable: std::env::current_exe()?,
        })
    }

    pub(crate) fn enabled(&self) -> bool {
        use windows_sys::Win32::Foundation::ERROR_SUCCESS;
        use windows_sys::Win32::System::Registry::{
            HKEY_CURRENT_USER, KEY_QUERY_VALUE, RegCloseKey, RegOpenKeyExW,
            RegQueryValueExW,
        };
        let subkey = Self::wide(RUN_KEY);
        let value = Self::wide(AUTOSTART_VALUE);
        let mut key = std::ptr::null_mut();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                0,
                KEY_QUERY_VALUE,
                &mut key,
            )
        };
        if status != ERROR_SUCCESS {
            return false;
        }
        let mut size = 0;
        let status = unsafe {
            RegQueryValueExW(
                key,
                value.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut size,
            )
        };
        unsafe {
            RegCloseKey(key);
        }
        status == ERROR_SUCCESS
    }

    pub(crate) fn enable(&self) -> std::io::Result<()> {
        use windows_sys::Win32::Foundation::ERROR_SUCCESS;
        use windows_sys::Win32::System::Registry::{
            HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegOpenKeyExW,
            RegSetValueExW,
        };
        let subkey = Self::wide(RUN_KEY);
        let value = Self::wide(AUTOSTART_VALUE);
        let command = format!("\"{}\" --hidden", self.executable.display());
        let data = Self::wide(&command);
        let mut key = std::ptr::null_mut();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                0,
                KEY_SET_VALUE,
                &mut key,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(std::io::Error::from_raw_os_error(status as i32));
        }
        let status = unsafe {
            RegSetValueExW(
                key,
                value.as_ptr(),
                0,
                REG_SZ,
                data.as_ptr() as *const u8,
                (data.len() * std::mem::size_of::<u16>()) as u32,
            )
        };
        unsafe {
            RegCloseKey(key);
        }
        if status != ERROR_SUCCESS {
            return Err(std::io::Error::from_raw_os_error(status as i32));
        }
        Ok(())
    }

    pub(crate) fn disable(&self) -> std::io::Result<()> {
        use windows_sys::Win32::Foundation::{
            ERROR_FILE_NOT_FOUND, ERROR_SUCCESS,
        };
        use windows_sys::Win32::System::Registry::{
            HKEY_CURRENT_USER, KEY_SET_VALUE, RegCloseKey, RegDeleteValueW,
            RegOpenKeyExW,
        };
        let subkey = Self::wide(RUN_KEY);
        let value = Self::wide(AUTOSTART_VALUE);
        let mut key = std::ptr::null_mut();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                0,
                KEY_SET_VALUE,
                &mut key,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(std::io::Error::from_raw_os_error(status as i32));
        }
        let status = unsafe { RegDeleteValueW(key, value.as_ptr()) };
        unsafe {
            RegCloseKey(key);
        }
        if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(std::io::Error::from_raw_os_error(status as i32))
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        std::ffi::OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }
}

#[cfg(target_os = "linux")]
impl Autostart {
    pub(crate) fn new() -> std::io::Result<Self> {
        let base = directories::BaseDirs::new().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no home directory",
            )
        })?;
        Ok(Self {
            executable: std::env::current_exe()?,
            entry: base
                .config_dir()
                .join("autostart")
                .join("calculator.desktop"),
        })
    }

    pub(crate) fn enabled(&self) -> bool {
        self.entry.exists()
    }

    pub(crate) fn enable(&self) -> std::io::Result<()> {
        if let Some(directory) = self.entry.parent() {
            std::fs::create_dir_all(directory)?;
        }
        let contents = format!(
            "[Desktop Entry]\nType=Application\nName=Calculator\nExec=\"{}\" --hidden\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
            self.executable.display()
        );
        std::fs::write(&self.entry, contents)
    }

    pub(crate) fn disable(&self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.entry) {
            Ok(()) => Ok(()),
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
}

#[cfg(target_os = "macos")]
impl Autostart {
    pub(crate) fn new() -> std::io::Result<Self> {
        let base = directories::BaseDirs::new().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no home directory",
            )
        })?;
        Ok(Self {
            executable: std::env::current_exe()?,
            entry: base
                .home_dir()
                .join("Library")
                .join("LaunchAgents")
                .join("calculator.plist"),
        })
    }

    pub(crate) fn enabled(&self) -> bool {
        self.entry.exists()
    }

    pub(crate) fn enable(&self) -> std::io::Result<()> {
        if let Some(directory) = self.entry.parent() {
            std::fs::create_dir_all(directory)?;
        }
        let executable = self
            .executable
            .display()
            .to_string()
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;");
        let contents = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n<key>Label</key>\n<string>com.calculator.app</string>\n<key>ProgramArguments</key>\n<array>\n<string>{executable}</string>\n<string>--hidden</string>\n</array>\n<key>RunAtLoad</key>\n<true/>\n</dict>\n</plist>\n"
        );
        std::fs::write(&self.entry, contents)
    }

    pub(crate) fn disable(&self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.entry) {
            Ok(()) => Ok(()),
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
}
