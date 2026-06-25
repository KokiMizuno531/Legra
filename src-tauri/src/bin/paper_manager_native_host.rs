use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[allow(dead_code)]
#[path = "../platform.rs"]
mod platform;

const MANAGED_LIBRARY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
struct Paper {
    folder_category: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Settings {
    managed_directory: Option<String>,
    workspace_root: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AppData {
    papers: Vec<Paper>,
    settings: Settings,
}

#[derive(Debug, Deserialize)]
struct ManagedLibraryFile {
    schema_version: u32,
    revision: u64,
    data: AppData,
}

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
    abstract_text: Option<String>,
    pdf_path: Option<String>,
    suggested_category: Option<String>,
    tags: Option<Vec<String>>,
    import_warnings: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct NativeMessage {
    action: String,
    request: Option<ExtensionImportRequest>,
}

#[derive(Debug, Serialize)]
struct NativeResponse {
    ok: bool,
    message: String,
    request_path: Option<String>,
    categories: Option<Vec<String>>,
}

fn app_root_dir() -> Result<PathBuf, String> {
    let exe_path = std::env::current_exe().map_err(|error| error.to_string())?;
    exe_path
        .parent()
        .map(|path| path.to_path_buf())
        .ok_or_else(|| "Could not resolve native host directory.".to_string())
}

fn default_setting_dir() -> Result<PathBuf, String> {
    platform::default_setting_dir()
}

fn legacy_setting_dir() -> Result<PathBuf, String> {
    Ok(app_root_dir()?.join("setting"))
}

fn setting_dir() -> Result<PathBuf, String> {
    let default_dir = default_setting_dir()?;
    let legacy_dir = legacy_setting_dir()?;
    let default_data = default_dir.join("app-data.json");
    let legacy_data = legacy_dir.join("app-data.json");
    if legacy_data.exists() && !default_data.exists() {
        return Ok(legacy_dir);
    }

    Ok(default_dir)
}

fn pending_dir() -> Result<PathBuf, String> {
    Ok(setting_dir()?.join("extension-imports").join("pending"))
}

fn data_file_path() -> Result<PathBuf, String> {
    Ok(setting_dir()?.join("app-data.json"))
}

fn workspace_data_file(root: &Path) -> PathBuf {
    root.join("paper-manager-workspace.json")
}

fn workspace_papers_dir(root: &Path) -> PathBuf {
    root.join("papers")
}

fn managed_library_data_file(root: &Path) -> PathBuf {
    root.join(".legra").join("library.json")
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

fn read_managed_library(root: &Path) -> Result<AppData, String> {
    let json =
        fs::read_to_string(managed_library_data_file(root)).map_err(|error| error.to_string())?;
    let library: ManagedLibraryFile =
        serde_json::from_str(&json).map_err(|error| error.to_string())?;
    if library.schema_version > MANAGED_LIBRARY_SCHEMA_VERSION {
        return Err("Managed library was created by a newer Legra version.".to_string());
    }

    let _revision = library.revision;
    let mut data = library.data;
    data.settings.managed_directory = Some(root.to_string_lossy().into_owned());
    data.settings.workspace_root = None;
    Ok(data)
}

fn read_app_data() -> Result<Option<AppData>, String> {
    let data_file = data_file_path()?;
    if !data_file.exists() {
        return Ok(None);
    }

    let json = fs::read_to_string(&data_file).map_err(|error| error.to_string())?;
    let local_data: AppData = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    let workspace_root = local_data
        .settings
        .workspace_root
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if let Some(workspace_root) = workspace_root {
        let workspace_file = workspace_data_file(Path::new(workspace_root));
        if !workspace_file.exists() {
            return Ok(Some(local_data));
        }

        let workspace_json =
            fs::read_to_string(&workspace_file).map_err(|error| error.to_string())?;
        let mut workspace_data: AppData =
            serde_json::from_str(&workspace_json).map_err(|error| error.to_string())?;
        workspace_data.settings.workspace_root = Some(workspace_root.to_string());
        workspace_data.settings.managed_directory = Some(
            workspace_papers_dir(Path::new(workspace_root))
                .to_string_lossy()
                .into_owned(),
        );
        return Ok(Some(workspace_data));
    }

    if let Some(root) = local_data
        .settings
        .managed_directory
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        if managed_library_data_file(&root).exists() {
            return Ok(Some(read_managed_library(&root)?));
        }
    }

    Ok(Some(local_data))
}

fn add_category_with_ancestors(categories: &mut BTreeSet<String>, value: &str) {
    let parts = value
        .split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    for index in 0..parts.len() {
        categories.insert(parts[..=index].join("/"));
    }
}

fn collect_directory_categories(
    categories: &mut BTreeSet<String>,
    root: &Path,
    current: &Path,
) -> Result<(), String> {
    if !current.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(current).map_err(|error| error.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if !file_type.is_dir() {
            continue;
        }
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "notes" || name == ".legra")
        {
            continue;
        }

        if let Ok(relative) = path.strip_prefix(root) {
            let category = relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            add_category_with_ancestors(categories, &category);
        }
        collect_directory_categories(categories, root, &path)?;
    }

    Ok(())
}

fn list_categories() -> Result<Vec<String>, String> {
    let Some(data) = read_app_data()? else {
        return Ok(Vec::new());
    };

    let mut categories = BTreeSet::new();
    for paper in &data.papers {
        if let Some(category) = paper.folder_category.as_deref() {
            add_category_with_ancestors(&mut categories, category);
        }
    }

    let category_root = data
        .settings
        .workspace_root
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|root| workspace_papers_dir(Path::new(root)))
        .or_else(|| {
            data.settings
                .managed_directory
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        });

    if let Some(root) = category_root {
        collect_directory_categories(&mut categories, &root, &root)?;
    }

    Ok(categories.into_iter().collect())
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

    if message.action == "list_categories" {
        return Ok(NativeResponse {
            ok: true,
            message: "Loaded Legra categories.".to_string(),
            request_path: None,
            categories: Some(list_categories()?),
        });
    }

    if message.action == "import_paper" {
        let request = message
            .request
            .ok_or_else(|| "Import request is missing.".to_string())?;
        let path = write_import_request(request)?;
        return Ok(NativeResponse {
            ok: true,
            message: "Import request queued for Legra.".to_string(),
            request_path: Some(path.to_string_lossy().into_owned()),
            categories: None,
        });
    }

    Err(format!("Unsupported action: {}", message.action))
}

fn main() {
    let response = match run() {
        Ok(response) => response,
        Err(error) => NativeResponse {
            ok: false,
            message: error,
            request_path: None,
            categories: None,
        },
    };

    if let Err(error) = write_native_response(&response) {
        eprintln!("Could not write native response: {error}");
    }
}
