use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Deserialize, Serialize)]
struct ExtensionImportRequest {
    id: Option<String>,
    source_url: Option<String>,
    doi: Option<String>,
    arxiv_id: Option<String>,
    title: Option<String>,
    authors: Option<Vec<String>>,
    year: Option<u16>,
    publication: Option<String>,
    pdf_path: Option<String>,
    suggested_category: Option<String>,
    tags: Option<Vec<String>>,
    import_warnings: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct NativeMessage {
    action: String,
    request: ExtensionImportRequest,
}

#[derive(Debug, Serialize)]
struct NativeResponse {
    ok: bool,
    message: String,
    request_path: Option<String>,
}

fn app_root_dir() -> Result<PathBuf, String> {
    let exe_path = std::env::current_exe().map_err(|error| error.to_string())?;
    exe_path
        .parent()
        .map(|path| path.to_path_buf())
        .ok_or_else(|| "Could not resolve native host directory.".to_string())
}

fn pending_dir() -> Result<PathBuf, String> {
    Ok(app_root_dir()?
        .join("setting")
        .join("extension-imports")
        .join("pending"))
}

fn now_millis() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| error.to_string())
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn write_import_request(mut request: ExtensionImportRequest) -> Result<PathBuf, String> {
    let dir = pending_dir()?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let id = request
        .id
        .clone()
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| format!("chrome-import-{}", now_millis().unwrap_or(0)));
    request.id = Some(id.clone());
    let filename = format!("{}.json", safe_id(&id));
    let path = dir.join(filename);
    let json = serde_json::to_string_pretty(&request).map_err(|error| error.to_string())?;
    fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(path)
}

fn read_native_message() -> Result<NativeMessage, String> {
    let mut length_bytes = [0_u8; 4];
    io::stdin()
        .read_exact(&mut length_bytes)
        .map_err(|error| error.to_string())?;
    let length = u32::from_ne_bytes(length_bytes) as usize;
    let mut buffer = vec![0_u8; length];
    io::stdin()
        .read_exact(&mut buffer)
        .map_err(|error| error.to_string())?;
    serde_json::from_slice(&buffer).map_err(|error| error.to_string())
}

fn write_native_response(response: &NativeResponse) -> Result<(), String> {
    let json = serde_json::to_vec(response).map_err(|error| error.to_string())?;
    let length = (json.len() as u32).to_ne_bytes();
    let mut stdout = io::stdout();
    stdout
        .write_all(&length)
        .and_then(|_| stdout.write_all(&json))
        .and_then(|_| stdout.flush())
        .map_err(|error| error.to_string())
}

fn run() -> Result<NativeResponse, String> {
    let message = read_native_message()?;
    if message.action != "import_paper" {
        return Err(format!("Unsupported action: {}", message.action));
    }

    let path = write_import_request(message.request)?;
    Ok(NativeResponse {
        ok: true,
        message: "Import request queued for paper-manager.".to_string(),
        request_path: Some(path.to_string_lossy().into_owned()),
    })
}

fn main() {
    let response = match run() {
        Ok(response) => response,
        Err(error) => NativeResponse {
            ok: false,
            message: error,
            request_path: None,
        },
    };

    if let Err(error) = write_native_response(&response) {
        eprintln!("Could not write native response: {error}");
    }
}
