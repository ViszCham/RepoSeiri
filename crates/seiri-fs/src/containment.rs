use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryPathRejectReason {
    Absolute,
    EscapesRepository,
    InvalidPercentEncoding,
    InvalidUtf8,
    NonPortableSeparator,
    NonPortablePrefix,
    SymlinkEscape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryPathUnknownReason {
    RootUnavailable,
    ProbeFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryPathResolution {
    Present(String),
    Missing(String),
    Rejected(RepositoryPathRejectReason),
    Unknown(RepositoryPathUnknownReason),
}

pub fn resolve_repository_path(root: &Path, raw: &str) -> RepositoryPathResolution {
    let canonical_root = match fs::canonicalize(root) {
        Ok(root) if root.is_dir() => root,
        _ => {
            return RepositoryPathResolution::Unknown(RepositoryPathUnknownReason::RootUnavailable)
        }
    };
    let relative = match normalize_repository_path(raw) {
        Ok(relative) => relative,
        Err(reason) => return RepositoryPathResolution::Rejected(reason),
    };
    let candidate = canonical_root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));

    match nearest_existing_ancestor(&candidate) {
        Ok(ancestor) => match fs::canonicalize(&ancestor) {
            Ok(canonical) if canonical.starts_with(&canonical_root) => {}
            Ok(_) => {
                return RepositoryPathResolution::Rejected(
                    RepositoryPathRejectReason::SymlinkEscape,
                )
            }
            Err(_) => {
                return RepositoryPathResolution::Unknown(RepositoryPathUnknownReason::ProbeFailed)
            }
        },
        Err(reason) => return RepositoryPathResolution::Unknown(reason),
    }

    match exact_case_exists(&canonical_root, &relative) {
        Ok(true) => RepositoryPathResolution::Present(relative),
        Ok(false) => RepositoryPathResolution::Missing(relative),
        Err(reason) => RepositoryPathResolution::Unknown(reason),
    }
}

fn normalize_repository_path(raw: &str) -> Result<String, RepositoryPathRejectReason> {
    let raw = raw.trim();
    if raw.is_empty() || raw.starts_with('/') || raw.starts_with("//") {
        return Err(RepositoryPathRejectReason::Absolute);
    }
    if raw.contains('\\') {
        return Err(RepositoryPathRejectReason::NonPortableSeparator);
    }
    let decoded = percent_decode(raw)?;
    if decoded.starts_with('/') || decoded.contains('\\') {
        return Err(RepositoryPathRejectReason::NonPortableSeparator);
    }
    if decoded.contains(':') || decoded.as_bytes().contains(&0) {
        return Err(RepositoryPathRejectReason::NonPortablePrefix);
    }

    let mut segments = Vec::new();
    for segment in decoded.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if segments.pop().is_none() {
                    return Err(RepositoryPathRejectReason::EscapesRepository);
                }
            }
            value => segments.push(value),
        }
    }
    if segments.is_empty() {
        return Ok(".".to_string());
    }
    Ok(segments.join("/"))
}

fn percent_decode(raw: &str) -> Result<String, RepositoryPathRejectReason> {
    let bytes = raw.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] != b'%' {
            decoded.push(bytes[cursor]);
            cursor += 1;
            continue;
        }
        let Some(pair) = bytes.get(cursor + 1..cursor + 3) else {
            return Err(RepositoryPathRejectReason::InvalidPercentEncoding);
        };
        let high = decode_hex(pair[0])?;
        let low = decode_hex(pair[1])?;
        decoded.push((high << 4) | low);
        cursor += 3;
    }
    String::from_utf8(decoded).map_err(|_| RepositoryPathRejectReason::InvalidUtf8)
}

fn decode_hex(value: u8) -> Result<u8, RepositoryPathRejectReason> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(RepositoryPathRejectReason::InvalidPercentEncoding),
    }
}

fn nearest_existing_ancestor(path: &Path) -> Result<PathBuf, RepositoryPathUnknownReason> {
    let mut cursor = path;
    loop {
        match fs::symlink_metadata(cursor) {
            Ok(_) => return Ok(cursor.to_path_buf()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                cursor = cursor
                    .parent()
                    .ok_or(RepositoryPathUnknownReason::ProbeFailed)?;
            }
            Err(_) => return Err(RepositoryPathUnknownReason::ProbeFailed),
        }
    }
}

fn exact_case_exists(root: &Path, relative: &str) -> Result<bool, RepositoryPathUnknownReason> {
    if relative == "." {
        return Ok(true);
    }
    let mut current = root.to_path_buf();
    for segment in relative.split('/') {
        let expected = OsStr::new(segment);
        let entries =
            fs::read_dir(&current).map_err(|_| RepositoryPathUnknownReason::ProbeFailed)?;
        let mut matched = None;
        for entry in entries {
            let entry = entry.map_err(|_| RepositoryPathUnknownReason::ProbeFailed)?;
            if entry.file_name() == expected {
                matched = Some(entry.path());
                break;
            }
        }
        let Some(path) = matched else {
            return Ok(false);
        };
        current = path;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn lexical_normalization_rejects_repository_escape() {
        assert_eq!(
            normalize_repository_path("../SECURITY.md"),
            Err(RepositoryPathRejectReason::EscapesRepository)
        );
        assert_eq!(
            normalize_repository_path("%2e%2e/SECURITY.md"),
            Err(RepositoryPathRejectReason::EscapesRepository)
        );
        assert_eq!(
            normalize_repository_path("docs/../SECURITY.md"),
            Ok("SECURITY.md".to_string())
        );
    }

    #[test]
    fn exact_case_is_required_even_on_case_insensitive_filesystems() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("reposeiri-containment-{nonce}"));
        fs::create_dir_all(root.join("Docs")).expect("fixture directory");
        fs::write(root.join("Docs/Guide.md"), "# Guide\n").expect("fixture file");
        assert_eq!(
            resolve_repository_path(&root, "Docs/Guide.md"),
            RepositoryPathResolution::Present("Docs/Guide.md".to_string())
        );
        assert_eq!(
            resolve_repository_path(&root, "docs/guide.md"),
            RepositoryPathResolution::Missing("docs/guide.md".to_string())
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
