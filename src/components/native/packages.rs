//! Count native package databases without invoking package managers.
use super::Sources;
use std::{
    io::{BufRead, BufReader},
    path::Path,
};
fn directories(path: &Path, required: Option<&str>) -> Option<u64> {
    let mut count = 0;
    for entry in std::fs::read_dir(path).ok()? {
        let entry = entry.ok()?;
        if entry.file_type().ok()?.is_dir()
            && required.is_none_or(|name| entry.path().join(name).is_file())
        {
            count += 1;
        }
    }
    Some(count)
}
pub fn dpkg(reader: impl BufRead) -> Option<u64> {
    let mut count = 0;
    let mut packages = 0;
    let mut nonempty = false;
    for line in reader.lines() {
        let line = line.ok()?;
        nonempty |= !line.trim().is_empty();
        if line.starts_with("Package: ") {
            packages += 1;
        }
        if line.trim() == "Status: install ok installed" {
            count += 1;
        }
    }
    if nonempty && packages == 0 {
        None
    } else {
        Some(count)
    }
}
fn rpm(path: &Path) -> Option<u64> {
    if !path.is_file() {
        return None;
    }
    let db = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;
    db.busy_timeout(std::time::Duration::from_millis(50)).ok()?;
    db.query_row("SELECT COUNT(DISTINCT hnum) FROM Sigmd5", [], |row| {
        row.get::<_, u64>(0)
    })
    .or_else(|_| {
        db.query_row("SELECT COUNT(*) FROM Packages", [], |row| {
            row.get::<_, u64>(0)
        })
    })
    .ok()
}
fn flatpak(base: &Path) -> Option<u64> {
    std::fs::read_dir(base).ok()?;
    fn dirs(path: &Path) -> Option<Vec<std::path::PathBuf>> {
        let entries = match std::fs::read_dir(path) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Some(vec![]),
            Err(_) => return None,
        };
        let mut paths = vec![];
        for entry in entries {
            let entry = entry.ok()?;
            if entry.file_type().ok()?.is_dir()
                && !entry.file_name().to_string_lossy().starts_with('.')
            {
                paths.push(entry.path());
            }
        }
        Some(paths)
    }
    let mut count = dirs(&base.join("app"))?
        .into_iter()
        .filter(|app| app.join("current").exists())
        .count() as u64;
    for runtime in dirs(&base.join("runtime"))? {
        let name = runtime.file_name()?.to_string_lossy();
        if name.ends_with(".Locale") || name.ends_with(".Debug") {
            continue;
        }
        for arch in dirs(&runtime)? {
            count += dirs(&arch)?.len() as u64;
        }
    }
    Some(count)
}
pub fn collect(s: &Sources) -> Option<Vec<(String, u64)>> {
    let mut counts = vec![];
    let pacman = s
        .read(&s.path("etc/pacman.conf"))
        .map(|v| super::appearance::ini(&v))
        .unwrap_or_default();
    let db = pacman
        .get(&("options".into(), "DBPath".into()))
        .map(|p| s.path(p).join("local"))
        .or_else(|| {
            pacman
                .get(&("options".into(), "RootDir".into()))
                .map(|p| s.path(p).join("var/lib/pacman/local"))
        })
        .unwrap_or_else(|| s.path("var/lib/pacman/local"));
    if let Some(n) = directories(&db, Some("desc")) {
        counts.push(("pacman".into(), n));
    }
    if let Ok(file) = std::fs::File::open(s.path("var/lib/dpkg/status"))
        && let Some(n) = dpkg(BufReader::new(file))
    {
        counts.push(("dpkg".into(), n));
    }
    if let Some(n) = [
        "usr/lib/sysimage/rpm/rpmdb.sqlite",
        "var/lib/rpm/rpmdb.sqlite",
    ]
    .into_iter()
    .find_map(|p| rpm(&s.path(p)))
    {
        counts.push(("rpm".into(), n));
    }
    let user_data = s
        .env("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| s.home.join(".local/share"));
    let flatpak = [s.path("var/lib/flatpak"), user_data.join("flatpak")]
        .iter()
        .filter_map(|p| flatpak(p))
        .collect::<Vec<_>>();
    if !flatpak.is_empty() {
        counts.push(("flatpak".into(), flatpak.iter().sum()));
    }
    if let Ok(entries) = std::fs::read_dir(s.path("var/lib/snapd/sequence")) {
        let mut count = 0;
        let mut valid = true;
        for entry in entries {
            let Ok(entry) = entry else {
                valid = false;
                break;
            };
            if entry.path().extension().is_some_and(|s| s == "json") {
                let sequence = s
                    .read(&entry.path())
                    .and_then(|v| serde_json::from_str::<serde_json::Value>(&v).ok());
                if sequence.as_ref().is_some_and(|v| {
                    v.get("current").is_some()
                        && v.get("sequence")
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(|a| !a.is_empty())
                }) {
                    count += 1;
                } else {
                    valid = false;
                    break;
                }
            }
        }
        if valid {
            counts.push(("snap".into(), count));
        }
    }
    (!counts.is_empty()).then_some(counts)
}
