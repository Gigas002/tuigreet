use std::{
    env,
    error::Error,
    ffi::CStr,
    fs::{self},
    path::{Path, PathBuf},
    process::Command,
    sync::LazyLock,
};

use chrono::Local;
use ini::Ini;
use rustix::system::uname;

use crate::{
    Greeter,
    model::sessions::{Session, SessionType},
};

static XDG_DATA_DIRS: LazyLock<Vec<PathBuf>> = LazyLock::new(|| {
    let value = env::var("XDG_DATA_DIRS").unwrap_or("/usr/local/share:/usr/share".to_string());
    env::split_paths(&value)
        .filter(|p| p.is_absolute())
        .collect()
});

static DEFAULT_SESSION_PATHS: LazyLock<Vec<(PathBuf, SessionType)>> = LazyLock::new(|| {
    XDG_DATA_DIRS
        .iter()
        .map(|p| (p.join("wayland-sessions"), SessionType::Wayland))
        .chain(
            XDG_DATA_DIRS
                .iter()
                .map(|p| (p.join("xsessions"), SessionType::X11)),
        )
        .collect()
});

fn cstr_str(c: &CStr) -> &str {
    c.to_str().unwrap_or("")
}

pub fn get_hostname() -> String {
    cstr_str(uname().nodename()).to_string()
}

pub fn get_issue() -> Option<String> {
    let (date, time) = {
        let now = Local::now();

        (
            now.format("%a %b %_d %Y").to_string(),
            now.format("%H:%M:%S").to_string(),
        )
    };

    // `\U` in /etc/issue historically counts utmp logins; we no longer read utmp.
    let user_count = "0 user";

    let vtnr: usize = env::var("XDG_VTNR")
        .unwrap_or_else(|_| "0".to_string())
        .parse()
        .unwrap_or(0);
    let uts = uname();

    if let Ok(issue) = fs::read_to_string("/etc/issue") {
        let issue = issue
            .replace("\\S", "Linux")
            .replace("\\l", &format!("tty{vtnr}"))
            .replace("\\d", &date)
            .replace("\\t", &time)
            .replace("\\U", user_count)
            .replace("\\s", cstr_str(uts.sysname()))
            .replace("\\r", cstr_str(uts.release()))
            .replace("\\v", cstr_str(uts.version()))
            .replace("\\n", cstr_str(uts.nodename()))
            .replace("\\m", cstr_str(uts.machine()));

        #[cfg(target_os = "linux")]
        let issue = issue.replace("\\o", cstr_str(uts.domainname()));

        return Some(
            issue
                .replace("\\x1b", "\x1b")
                .replace("\\033", "\x1b")
                .replace("\\e", "\x1b")
                .replace(r"\\", r"\"),
        );
    }

    None
}

pub fn get_sessions(greeter: &Greeter) -> Result<Vec<Session>, Box<dyn Error>> {
    let paths = if greeter.session_paths.is_empty() {
        &*DEFAULT_SESSION_PATHS
    } else {
        &greeter.session_paths
    };

    let mut files = vec![];

    for (path, session_type) in paths.iter() {
        tracing::info!(
            "reading {:?} sessions from '{}'",
            session_type,
            path.display()
        );

        if let Ok(entries) = fs::read_dir(path) {
            files.extend(
                entries
                    .flat_map(|entry| {
                        entry.map(|entry| load_desktop_file(entry.path(), *session_type))
                    })
                    .flatten()
                    .flatten(),
            );
        }
    }

    files.sort_by(|a, b| a.name.cmp(&b.name));

    tracing::info!("found {} sessions", files.len());

    Ok(files)
}

fn load_desktop_file<P>(
    path: P,
    session_type: SessionType,
) -> Result<Option<Session>, Box<dyn Error>>
where
    P: AsRef<Path>,
{
    let desktop = Ini::load_from_file(path.as_ref())?;
    let section = desktop
        .section(Some("Desktop Entry"))
        .ok_or("no Desktop Entry section in desktop file")?;

    if let Some("true") = section.get("Hidden") {
        tracing::info!(
            "ignoring session in '{}': Hidden=true",
            path.as_ref().display()
        );
        return Ok(None);
    }
    if let Some("true") = section.get("NoDisplay") {
        tracing::info!(
            "ignoring session in '{}': NoDisplay=true",
            path.as_ref().display()
        );
        return Ok(None);
    }

    let slug = path
        .as_ref()
        .file_stem()
        .map(|slug| slug.to_string_lossy().to_string());
    let name = section
        .get("Name")
        .ok_or("no Name property in desktop file")?;
    let exec = section
        .get("Exec")
        .ok_or("no Exec property in desktop file")?;
    let xdg_desktop_names = section.get("DesktopNames").map(str::to_string);

    tracing::info!("got session '{}' in '{}'", name, path.as_ref().display());

    Ok(Some(Session {
        slug,
        name: name.to_string(),
        command: exec.to_string(),
        session_type,
        path: Some(path.as_ref().into()),
        xdg_desktop_names,
    }))
}

pub fn capslock_status() -> bool {
    let mut command = Command::new("kbdinfo");
    command.args(["gkbled", "capslock"]);

    match command.output() {
        Ok(output) => output.status.code() == Some(0),
        Err(_) => false,
    }
}
