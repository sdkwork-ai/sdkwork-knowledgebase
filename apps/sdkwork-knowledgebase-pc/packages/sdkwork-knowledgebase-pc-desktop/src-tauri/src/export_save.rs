use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tokio::sync::Semaphore;

pub const MAX_EXPORT_FILE_BYTES: usize = 64 * 1024 * 1024;
const MAX_CONCURRENT_EXPORT_SAVES: usize = 1;
static EXPORT_SAVE_LIMIT: LazyLock<Semaphore> =
    LazyLock::new(|| Semaphore::new(MAX_CONCURRENT_EXPORT_SAVES));

pub fn decode_bounded_base64(
    payload: &str,
    max_decoded_bytes: usize,
    label: &str,
) -> Result<Vec<u8>, String> {
    let max_encoded_bytes = max_decoded_bytes
        .checked_add(2)
        .and_then(|value| value.checked_div(3))
        .and_then(|value| value.checked_mul(4))
        .ok_or_else(|| format!("{label} size limit is invalid"))?;
    if payload.len() > max_encoded_bytes {
        return Err(format!("{label} exceeds the maximum allowed size"));
    }
    let bytes = STANDARD
        .decode(payload.as_bytes())
        .map_err(|error| format!("invalid {label}: {error}"))?;
    if bytes.len() > max_decoded_bytes {
        return Err(format!("{label} exceeds the maximum allowed size"));
    }
    Ok(bytes)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportSaveMode {
    Downloads,
    SaveAs,
}

impl ExportSaveMode {
    fn from_str(raw: &str) -> Result<Self, String> {
        match raw {
            "downloads" => Ok(Self::Downloads),
            "saveAs" => Ok(Self::SaveAs),
            _ => Err(format!("unsupported export save mode: {raw}")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Downloads => "downloads",
            Self::SaveAs => "saveAs",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveExportFileRequest {
    suggested_name: String,
    data_base64: String,
    mode: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveExportFileResponse {
    pub saved: bool,
    pub cancelled: bool,
    pub path: Option<String>,
    pub mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevealExportFileRequest {
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocateExportFileRequest {
    file_name: String,
}

pub fn sanitize_file_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "document.bin".to_string();
    }

    let mut sanitized: String = trimmed
        .chars()
        .map(|ch| {
            if matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || ch.is_control()
            {
                '_'
            } else {
                ch
            }
        })
        .collect();

    // Windows: trailing dots/spaces are dropped by the filesystem, and device
    // names (CON.pdf, NUL.txt, COM1.pdf) map to devices. A bare `..` would
    // walk up a directory when joined.
    while sanitized.ends_with('.') || sanitized.ends_with(' ') {
        sanitized.pop();
    }
    let upper = sanitized.to_ascii_uppercase();
    let stem = upper.split('.').next().unwrap_or("");
    let is_reserved_device = matches!(
        stem,
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );
    if sanitized.is_empty() || sanitized == ".." || is_reserved_device {
        sanitized = "document.bin".to_string();
    }

    sanitized
}

pub fn resolve_downloads_dir() -> Result<PathBuf, String> {
    if let Some(dir) = dirs::download_dir() {
        return Ok(dir);
    }

    if let Ok(xdg_downloads) = std::env::var("XDG_DOWNLOAD_DIR") {
        let trimmed = xdg_downloads.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    if let Some(home) = dirs::home_dir() {
        let downloads = home.join("Downloads");
        return Ok(downloads);
    }

    if let Some(documents) = dirs::document_dir() {
        return Ok(documents);
    }

    Err("downloads directory is unavailable on this system".to_string())
}

pub fn unique_file_path(directory: &Path, file_name: &str) -> PathBuf {
    let safe_name = sanitize_file_name(file_name);
    let mut candidate = directory.join(&safe_name);
    if !candidate.exists() {
        return candidate;
    }

    let path = Path::new(&safe_name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("document");
    let extension = path.extension().and_then(|value| value.to_str());

    for index in 1..=999 {
        let next_name = match extension {
            Some(ext) if !ext.is_empty() => format!("{stem} ({index}).{ext}"),
            _ => format!("{stem} ({index})"),
        };
        candidate = directory.join(next_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    let fallback = match extension {
        Some(ext) if !ext.is_empty() => format!("{stem}-{}.{}", chrono_like_suffix(), ext),
        _ => format!("{stem}-{}", chrono_like_suffix()),
    };
    directory.join(fallback)
}

fn chrono_like_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn save_bytes_to_export_path(
    bytes: Vec<u8>,
    suggested_name: &str,
    mode: ExportSaveMode,
) -> Result<SaveExportFileResponse, String> {
    let safe_name = sanitize_file_name(suggested_name);

    match mode {
        ExportSaveMode::Downloads => {
            let downloads_dir = resolve_downloads_dir()?;
            std::fs::create_dir_all(&downloads_dir).map_err(map_io_error)?;
            let target_path = unique_file_path(&downloads_dir, &safe_name);
            std::fs::write(&target_path, bytes).map_err(map_io_error)?;
            track_issued_export_path(&target_path);
            Ok(SaveExportFileResponse {
                saved: true,
                cancelled: false,
                path: Some(target_path.to_string_lossy().into_owned()),
                mode: mode.as_str().to_string(),
            })
        }
        ExportSaveMode::SaveAs => {
            let mut dialog = rfd::FileDialog::new().set_file_name(&safe_name);
            if let Ok(downloads_dir) = resolve_downloads_dir() {
                dialog = dialog.set_directory(downloads_dir);
            }

            let Some(path) = dialog.save_file() else {
                return Ok(SaveExportFileResponse {
                    saved: false,
                    cancelled: true,
                    path: None,
                    mode: mode.as_str().to_string(),
                });
            };

            std::fs::write(&path, bytes).map_err(map_io_error)?;
            track_issued_export_path(&path);
            Ok(SaveExportFileResponse {
                saved: true,
                cancelled: false,
                path: Some(path.to_string_lossy().into_owned()),
                mode: mode.as_str().to_string(),
            })
        }
    }
}

pub async fn save_base64_to_export_path_async(
    payload: String,
    max_decoded_bytes: usize,
    label: &'static str,
    suggested_name: String,
    mode: ExportSaveMode,
) -> Result<SaveExportFileResponse, String> {
    let _permit = EXPORT_SAVE_LIMIT
        .acquire()
        .await
        .map_err(|_| "export save limiter is unavailable".to_string())?;
    tokio::task::spawn_blocking(move || {
        let bytes = decode_bounded_base64(&payload, max_decoded_bytes, label)?;
        save_bytes_to_export_path(bytes, &suggested_name, mode)
    })
    .await
    .map_err(|error| format!("export save task failed: {error}"))?
}

pub fn reveal_export_in_folder(raw_path: &str) -> Result<(), String> {
    let path = validate_export_file_path(raw_path)?;
    ensure_issued_export_path(&path)?;

    #[cfg(windows)]
    {
        use std::process::Command;
        Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn()
            .map_err(|error| format!("failed to reveal export file: {error}"))?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
            .map_err(|error| format!("failed to reveal export file: {error}"))?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let folder = path
            .parent()
            .ok_or_else(|| "export file has no parent directory".to_string())?;
        open::that(folder).map_err(|error| format!("failed to open export folder: {error}"))?;
        return Ok(());
    }

    #[cfg(not(any(windows, target_os = "macos", unix)))]
    {
        Err("reveal export file is not supported on this platform".to_string())
    }
}

/// Export paths this application issued via `save_export_file`. `open` /
/// `reveal` accept ONLY these: a compromised renderer must not be able to use
/// them as an arbitrary program-execution (ShellExecute "open") or
/// file-reveal primitive against renderer-chosen absolute paths.
static ISSUED_EXPORT_PATHS: LazyLock<std::sync::Mutex<std::collections::HashSet<PathBuf>>> =
    LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

fn track_issued_export_path(path: &Path) {
    if let Ok(mut issued) = ISSUED_EXPORT_PATHS.lock() {
        issued.insert(path.to_path_buf());
    }
}

fn ensure_issued_export_path(path: &Path) -> Result<(), String> {
    let issued = ISSUED_EXPORT_PATHS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if issued.contains(path) {
        return Ok(());
    }
    Err(format!(
        "export file was not issued by this session: {}",
        path.display()
    ))
}

/// Document extensions the desktop shell will `ShellExecute("open")`. The
/// allowlist is the last line of defense: the renderer supplies both the
/// suggested filename and the payload bytes for an export, so without it a
/// compromised renderer could mint `evil.bat`/`evil.hta` and execute it via
/// the open command. New export formats extend this list deliberately.
// `.doc` (Word-HTML export) and `.html` are live save formats from the
// document-export UI; keep this list in step with documentExportRender.
const OPENABLE_EXPORT_EXTENSIONS: &[&str] = &[
    "pdf", "png", "jpg", "jpeg", "webp", "txt", "md", "doc", "html",
];

fn openable_export_extension(path: &Path) -> Result<String, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .ok_or_else(|| "export file has no extension".to_string())?;
    if OPENABLE_EXPORT_EXTENSIONS.contains(&extension.as_str()) {
        Ok(extension)
    } else {
        Err(format!(
            "export file extension is not openable: .{extension}"
        ))
    }
}

pub fn launch_export_file(raw_path: &str) -> Result<(), String> {
    let path = validate_export_file_path(raw_path)?;
    ensure_issued_export_path(&path)?;
    openable_export_extension(&path)?;
    open::that(&path).map_err(|error| format!("failed to open export file: {error}"))
}

pub fn locate_export_in_downloads(file_name: &str) -> Result<PathBuf, String> {
    let dir = resolve_downloads_dir()?;
    let safe = sanitize_file_name(file_name);
    let direct = dir.join(&safe);
    if direct.is_file() {
        return Ok(direct);
    }

    let path = Path::new(&safe);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("document");
    let extension = path.extension().and_then(|value| value.to_str());

    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    for entry in std::fs::read_dir(&dir).map_err(map_io_error)? {
        let entry = entry.map_err(map_io_error)?;
        let file_path = entry.path();
        if !file_path.is_file() {
            continue;
        }

        let name = file_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let entry_path = Path::new(name);
        let file_stem = entry_path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let file_ext = entry_path.extension().and_then(|value| value.to_str());

        let stem_matches = file_stem == stem
            || file_stem
                .strip_prefix(&format!("{stem} ("))
                .and_then(|suffix| suffix.strip_suffix(')'))
                .and_then(|inner| inner.parse::<u32>().ok())
                .is_some();
        let ext_matches = extension.is_none()
            || extension.map(|value| value.to_ascii_lowercase())
                == file_ext.map(|value| value.to_ascii_lowercase());
        if !stem_matches || !ext_matches {
            continue;
        }

        let modified = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        if best.as_ref().is_none_or(|(_, current)| modified > *current) {
            best = Some((file_path, modified));
        }
    }

    best.map(|(path, _)| path)
        .ok_or_else(|| format!("export file not found in downloads folder: {safe}"))
}

fn validate_export_file_path(raw_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw_path.trim());
    if !path.is_absolute() {
        return Err("only absolute export paths can be opened".to_string());
    }
    if !path.exists() {
        return Err(format!("export file not found: {}", path.display()));
    }
    Ok(path)
}

fn map_io_error(error: std::io::Error) -> String {
    format!("export save failed: {error}")
}

#[tauri::command]
pub async fn save_export_file(
    request: SaveExportFileRequest,
) -> Result<SaveExportFileResponse, String> {
    let mode = ExportSaveMode::from_str(&request.mode)?;
    save_base64_to_export_path_async(
        request.data_base64,
        MAX_EXPORT_FILE_BYTES,
        "export payload",
        request.suggested_name,
        mode,
    )
    .await
}

#[cfg(test)]
mod openable_extension_tests {
    use super::*;

    #[test]
    fn allows_document_extensions_case_insensitively() {
        assert!(openable_export_extension(Path::new("report.PDF")).is_ok());
        assert!(openable_export_extension(Path::new("shot.Jpeg")).is_ok());
        assert!(openable_export_extension(Path::new("notes.md")).is_ok());
    }

    #[test]
    fn rejects_executable_and_script_extensions() {
        for name in [
            "evil.bat", "evil.cmd", "evil.exe", "evil.hta", "evil.lnk", "evil.ps1",
        ] {
            assert!(
                openable_export_extension(Path::new(name)).is_err(),
                "{name} must be rejected"
            );
        }
    }

    #[test]
    fn rejects_missing_extensions_and_dotfiles() {
        assert!(openable_export_extension(Path::new("noext")).is_err());
        assert!(openable_export_extension(Path::new(".gitignore")).is_err());
    }
}

#[cfg(test)]
mod payload_tests {
    use super::*;

    #[test]
    fn bounded_base64_rejects_encoded_input_before_decode() {
        let oversized = "A".repeat(9);
        assert!(decode_bounded_base64(&oversized, 3, "payload").is_err());
    }

    #[test]
    fn bounded_base64_accepts_payload_at_limit() {
        let payload = STANDARD.encode([1_u8, 2, 3]);
        assert_eq!(
            decode_bounded_base64(&payload, 3, "payload").expect("bounded payload"),
            vec![1, 2, 3]
        );
    }
}

#[tauri::command]
pub fn reveal_export_file(request: RevealExportFileRequest) -> Result<(), String> {
    reveal_export_in_folder(&request.path)
}

#[tauri::command]
pub fn open_export_file(request: RevealExportFileRequest) -> Result<(), String> {
    launch_export_file(&request.path)
}

#[tauri::command]
pub fn locate_export_file(request: LocateExportFileRequest) -> Result<String, String> {
    let path = locate_export_in_downloads(&request.file_name)?;
    // Located exports are app-produced Downloads files (strict stem/extension
    // match above); tracking them lets the subsequent open/reveal succeed.
    track_issued_export_path(&path);
    Ok(path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn unique_file_path_appends_suffix_for_duplicates() {
        let temp_dir =
            std::env::temp_dir().join(format!("kb-export-test-{}", chrono_like_suffix()));
        fs::create_dir_all(&temp_dir).expect("temp dir should exist");
        let first = unique_file_path(&temp_dir, "note.pdf");
        fs::write(&first, b"first").expect("write first");
        let second = unique_file_path(&temp_dir, "note.pdf");
        assert_ne!(first, second);
        assert!(second.to_string_lossy().contains("(1)"));
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn sanitize_file_name_replaces_invalid_chars() {
        assert_eq!(sanitize_file_name("bad:name?.pdf"), "bad_name_.pdf");
    }
}
