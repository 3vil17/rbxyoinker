// Copyright (C) 2026 3vil17. AGPL-3.0 with additional terms, see LICENSE and NOTICE.md.
use std::path::{Path, PathBuf};

#[cfg(windows)]
pub const UPDATE_SUFFIX: &str = ".exe";
#[cfg(target_os = "macos")]
pub const UPDATE_SUFFIX: &str = "-macos.zip";

#[cfg(windows)]
pub const CREDENTIAL_STORE: &str = "Windows Credential Manager";
#[cfg(target_os = "macos")]
pub const CREDENTIAL_STORE: &str = "the macOS Keychain";

pub fn sibling(exe: &Path, tag: &str) -> PathBuf {
    if cfg!(windows) {
        exe.with_extension(format!("{tag}.exe"))
    } else {
        let mut name = exe.as_os_str().to_owned();
        name.push(format!(".{tag}"));
        PathBuf::from(name)
    }
}

#[cfg(windows)]
pub fn unpack(bytes: Vec<u8>) -> Result<Vec<u8>, &'static str> {
    if bytes.starts_with(b"MZ") && bytes.len() >= 100_000 {
        Ok(bytes)
    } else {
        Err("the downloaded file doesn't look like a valid exe")
    }
}

#[cfg(target_os = "macos")]
pub fn unpack(bytes: Vec<u8>) -> Result<Vec<u8>, &'static str> {
    use std::io::Read;
    let bad = "the downloaded file doesn't look like a valid rbxyoinker build";
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| bad)?;
    let mut file = zip.by_name("rbxyoinker").map_err(|_| bad)?;
    if file.size() > 200_000_000 {
        return Err(bad);
    }
    let mut out = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut out).map_err(|_| bad)?;
    let magic = out.get(..4).unwrap_or_default();
    let mach = [[0xca, 0xfe, 0xba, 0xbe], [0xcf, 0xfa, 0xed, 0xfe]];
    if out.len() < 100_000 || !mach.iter().any(|m| m == magic) {
        return Err(bad);
    }
    Ok(out)
}

pub fn make_executable(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    let _ = path;
    Ok(())
}

pub fn relaunch(exe: &Path) -> std::io::Result<std::process::Child> {
    use std::process::{Command, Stdio};
    let mut cmd = Command::new(exe);
    cmd.arg("--updated").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd.spawn()
}

pub fn default_plugins_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let dir = dirs::data_local_dir().map(|d| d.join("Roblox").join("Plugins"));
    #[cfg(target_os = "macos")]
    let dir = dirs::document_dir().map(|d| d.join("Roblox").join("Plugins"));
    dir.filter(|d| d.is_dir())
}

pub fn normalize(path: &str) -> String {
    if cfg!(windows) {
        path.replace('/', "\\")
    } else {
        path.to_string()
    }
}

#[cfg(windows)]
pub fn open_dir(dir: &Path) -> std::io::Result<()> {
    unlock_foreground();
    std::process::Command::new("explorer").arg(dir).spawn()?;
    tokio::spawn(raise_explorer(dir.to_path_buf()));
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn open_dir(dir: &Path) -> std::io::Result<()> {
    std::process::Command::new("open").arg(dir).spawn()?;
    Ok(())
}

#[cfg(windows)]
pub fn reveal_file(file: &Path, dir: &Path) -> std::io::Result<()> {
    unlock_foreground();
    std::process::Command::new("explorer").arg(format!("/select,{}", file.display())).spawn()?;
    tokio::spawn(raise_explorer(dir.to_path_buf()));
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn reveal_file(file: &Path, _dir: &Path) -> std::io::Result<()> {
    std::process::Command::new("open").arg("-R").arg(file).spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn unlock_foreground() {}

#[cfg(windows)]
pub fn unlock_foreground() {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEINPUT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY};
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: 0, dwFlags: MOUSEEVENTF_MOVE, time: 0, dwExtraInfo: 0 },
        },
    };
    unsafe {
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
        AllowSetForegroundWindow(ASFW_ANY);
    }
}

#[cfg(windows)]
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
async fn raise_explorer(dir: PathBuf) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowExW, GetWindowTextW, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };
    let full = dir.to_string_lossy().to_lowercase();
    let name = dir.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let class = wide("CabinetWClass");
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let mut found = None;
        unsafe {
            let mut h = FindWindowExW(std::ptr::null_mut(), std::ptr::null_mut(), class.as_ptr(), std::ptr::null());
            while !h.is_null() {
                let mut buf = [0u16; 512];
                let n = GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32);
                let title = String::from_utf16_lossy(&buf[..n.max(0) as usize]).to_lowercase();
                let base = title.split(" - ").next().unwrap_or("");
                if base == name || base == full {
                    found = Some(h as usize);
                    break;
                }
                h = FindWindowExW(std::ptr::null_mut(), h, class.as_ptr(), std::ptr::null());
            }
        }
        if let Some(h) = found {
            unlock_foreground();
            unsafe {
                let h = h as windows_sys::Win32::Foundation::HWND;
                if IsIconic(h) != 0 {
                    ShowWindow(h, SW_RESTORE);
                }
                SetForegroundWindow(h);
            }
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sibling_names() {
        let exe = Path::new("dir").join(if cfg!(windows) { "rbxyoinker.exe" } else { "rbxyoinker" });
        let want = if cfg!(windows) { "rbxyoinker.old.exe" } else { "rbxyoinker.old" };
        assert_eq!(sibling(&exe, "old").file_name().unwrap(), want);
    }

    #[test]
    fn unpack_rejects_junk() {
        assert!(unpack(b"not a build".to_vec()).is_err());
    }
}
