use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

/// ISO-8601 UTC timestamp with second precision.
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Lexical normalisation that also resolves existing symlinks when possible.
pub fn norm(p: &Path) -> PathBuf {
    if let Ok(c) = p.canonicalize() {
        return c;
    }
    // Resolve `..` lexically for paths that do not exist yet.
    let abs = std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    let mut out = PathBuf::new();
    for comp in abs.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// True when `child` is the same as, or nested inside, `parent`.
pub fn is_within(parent: &Path, child: &Path) -> bool {
    let p = norm(parent);
    let c = norm(child);
    c == p || c.starts_with(&p)
}

pub fn sha256_file(path: &Path) -> Option<String> {
    let mut f = File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(format!("{:x}", hasher.finalize()))
}

pub fn slugify(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.len() > 48 {
        trimmed[..48].trim_end_matches('-').to_string()
    } else if trimmed.is_empty() {
        "untitled".into()
    } else {
        trimmed
    }
}

pub fn count_words(text: &str) -> i64 {
    text.split_whitespace().count() as i64
}

/// Narration estimate at ~145 spoken words per minute.
pub fn estimate_duration_sec(words: i64) -> i64 {
    ((words as f64 / 145.0) * 60.0).round() as i64
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.1} {}", v, UNITS[i])
    }
}

/// Reject path strings that are obviously malformed before touching the filesystem.
pub fn validate_path_string(raw: &str) -> Result<(), String> {
    if raw.trim().is_empty() {
        return Err("Path is empty".into());
    }
    if raw.contains('\0') {
        return Err("Path contains a null byte".into());
    }
    if raw.len() > 32_000 {
        return Err("Path is too long".into());
    }
    Ok(())
}

pub fn friendly_err<E: std::fmt::Display>(context: &str, e: E) -> String {
    format!("{}: {}", context, e)
}
