use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    Emitter, Manager,
};

const SETTINGS_MENU_ID: &str = "settings";
const CHROME_NATIVE_HOST_NAME: &str = "app.legra.importer";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Paper {
    id: String,
    title: String,
    authors: Vec<String>,
    year: Option<u16>,
    publication: Option<String>,
    volume: Option<String>,
    issue: Option<String>,
    pages: Option<String>,
    numpages: Option<u16>,
    month: Option<String>,
    publisher: Option<String>,
    doi: Option<String>,
    arxiv_id: Option<String>,
    url: Option<String>,
    abstract_text: Option<String>,
    tags: Vec<String>,
    status: Option<String>,
    rating: Option<u8>,
    bibtex_key: Option<String>,
    pdf_path: Option<String>,
    original_pdf_path: Option<String>,
    folder_category: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct UpdatePaperInput {
    id: String,
    title: String,
    authors: Vec<String>,
    year: Option<u16>,
    publication: Option<String>,
    volume: Option<String>,
    issue: Option<String>,
    pages: Option<String>,
    numpages: Option<u16>,
    month: Option<String>,
    publisher: Option<String>,
    doi: Option<String>,
    arxiv_id: Option<String>,
    url: Option<String>,
    abstract_text: Option<String>,
    tags: Vec<String>,
    status: Option<String>,
    pdf_path: Option<String>,
    folder_category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Note {
    id: String,
    paper_id: String,
    title: String,
    file_path: String,
    file_type: Option<String>,
    summary: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct CreateNoteInput {
    paper_id: String,
    title: String,
}

#[derive(Debug, Deserialize)]
struct LinkNoteInput {
    paper_id: String,
    title: String,
    file_path: String,
}

#[derive(Debug, Deserialize)]
struct OrganizePdfInput {
    paper_id: String,
    folder_category: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeletePapersInput {
    paper_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct BibtexInput {
    paper_ids: Vec<String>,
    journal_output_style: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SaveBibtexInput {
    path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct UpdateSettingsInput {
    managed_directory: Option<String>,
    marktext_path: Option<String>,
    pdf_viewer_path: Option<String>,
    chrome_import_directory: Option<String>,
    chrome_extension_id: Option<String>,
    filename_rule: String,
    bibtex_key_rule: String,
    bibtex_export_rule: String,
    journal_output_style: String,
    journal_aliases: Vec<JournalAlias>,
    note_directory: Option<String>,
    cloud_sync_expected: bool,
}

#[derive(Debug, Deserialize)]
struct BackupInput {
    target_dir: String,
}

#[derive(Debug, Deserialize)]
struct RestoreInput {
    backup_dir: String,
}

#[derive(Debug, Deserialize)]
struct RelinkInput {
    root_dir: String,
}

#[derive(Debug, Deserialize)]
struct WorkspaceInput {
    workspace_dir: String,
}

#[derive(Debug, Deserialize)]
struct ChromeNativeHostInput {
    extension_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FetchMetadataInput {
    doi: Option<String>,
    arxiv_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResolvePaperImportInput {
    source: String,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize)]
struct ExtensionImportSummary {
    imported: usize,
    failed: usize,
    pending: usize,
    messages: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PaperMetadata {
    title: Option<String>,
    authors: Vec<String>,
    year: Option<u16>,
    publication: Option<String>,
    volume: Option<String>,
    issue: Option<String>,
    pages: Option<String>,
    numpages: Option<u16>,
    month: Option<String>,
    publisher: Option<String>,
    doi: Option<String>,
    arxiv_id: Option<String>,
    url: Option<String>,
    abstract_text: Option<String>,
}

#[derive(Debug, Serialize)]
struct PaperImportResolution {
    metadata: PaperMetadata,
    downloaded_pdf_path: Option<String>,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct BackupResult {
    backup_dir: String,
}

#[derive(Debug, Serialize)]
struct WorkspaceHealth {
    ok: bool,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ChromeNativeHostStatus {
    installed: bool,
    manifest_path: String,
    host_path: String,
    extension_id: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct NoteStatus {
    note_id: String,
    exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Settings {
    id: String,
    #[serde(default)]
    managed_directory: Option<String>,
    #[serde(default = "default_filename_rule")]
    filename_rule: String,
    #[serde(default)]
    note_directory: Option<String>,
    #[serde(default)]
    marktext_path: Option<String>,
    #[serde(default)]
    pdf_viewer_path: Option<String>,
    #[serde(default)]
    chrome_import_directory: Option<String>,
    #[serde(default)]
    chrome_extension_id: Option<String>,
    #[serde(default = "default_bibtex_key_rule")]
    bibtex_key_rule: String,
    #[serde(default = "default_bibtex_export_rule")]
    bibtex_export_rule: String,
    #[serde(default = "default_journal_output_style")]
    journal_output_style: String,
    #[serde(default = "default_journal_aliases")]
    journal_aliases: Vec<JournalAlias>,
    #[serde(default = "default_cloud_sync_expected")]
    cloud_sync_expected: bool,
    #[serde(default)]
    workspace_root: Option<String>,
    #[serde(default)]
    workspace_revision: Option<u64>,
    #[serde(default)]
    workspace_last_loaded_revision: Option<u64>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JournalAlias {
    full_name: String,
    abbreviation: String,
    aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppData {
    papers: Vec<Paper>,
    notes: Vec<Note>,
    settings: Settings,
}

#[derive(Debug, Deserialize)]
struct RegisterPaperInput {
    title: String,
    authors: Vec<String>,
    year: Option<u16>,
    publication: Option<String>,
    volume: Option<String>,
    issue: Option<String>,
    pages: Option<String>,
    numpages: Option<u16>,
    month: Option<String>,
    publisher: Option<String>,
    doi: Option<String>,
    arxiv_id: Option<String>,
    url: Option<String>,
    abstract_text: Option<String>,
    tags: Vec<String>,
    status: Option<String>,
    pdf_path: Option<String>,
    folder_category: Option<String>,
}

#[derive(Debug, Serialize)]
struct AppStatus {
    setting_dir: String,
    data_file: String,
    data_file_exists: bool,
}

fn app_root_dir() -> Result<PathBuf, String> {
    let exe_path = std::env::current_exe().map_err(|error| error.to_string())?;
    exe_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "Could not resolve application directory".to_string())
}

fn default_setting_dir() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("LEGRA_SETTING_DIR") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    let home = std::env::var("HOME").map_err(|_| "Could not resolve HOME directory.".to_string())?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("Legra"))
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

fn data_file_path() -> Result<PathBuf, String> {
    Ok(setting_dir()?.join("app-data.json"))
}

fn workspace_data_file(root: &Path) -> PathBuf {
    root.join("paper-manager-workspace.json")
}

fn workspace_meta_dir(root: &Path) -> PathBuf {
    root.join(".paper-manager")
}

fn workspace_papers_dir(root: &Path) -> PathBuf {
    root.join("papers")
}

fn workspace_notes_dir(root: &Path) -> PathBuf {
    root.join("notes")
}

fn workspace_exports_dir(root: &Path) -> PathBuf {
    root.join("exports")
}

fn workspace_lock_file(root: &Path) -> PathBuf {
    workspace_meta_dir(root).join("write.lock")
}

fn active_workspace_root(data: &AppData) -> Option<PathBuf> {
    data.settings
        .workspace_root
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn path_for_runtime(data: &AppData, stored_path: &str) -> PathBuf {
    let path = PathBuf::from(stored_path);
    if path.is_absolute() {
        return path;
    }

    active_workspace_root(data)
        .map(|root| root.join(&path))
        .unwrap_or(path)
}

fn path_for_storage(data: &AppData, path: &Path) -> String {
    if let Some(root) = active_workspace_root(data) {
        if let Ok(relative) = path.strip_prefix(root) {
            return relative.to_string_lossy().into_owned();
        }
    }

    path.to_string_lossy().into_owned()
}

fn notes_dir() -> Result<PathBuf, String> {
    if let Ok(data) = load_or_default_app_data() {
        if let Some(root) = active_workspace_root(&data) {
            return Ok(workspace_notes_dir(&root));
        }

        if let Some(path) = data.settings.note_directory.as_deref() {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed));
            }
        }
    }

    Ok(setting_dir()?.join("notes"))
}

fn extension_import_root_dir() -> Result<PathBuf, String> {
    Ok(setting_dir()?.join("extension-imports"))
}

fn extension_import_pending_dir() -> Result<PathBuf, String> {
    Ok(extension_import_root_dir()?.join("pending"))
}

fn extension_import_processed_dir() -> Result<PathBuf, String> {
    Ok(extension_import_root_dir()?.join("processed"))
}

fn extension_import_failed_dir() -> Result<PathBuf, String> {
    Ok(extension_import_root_dir()?.join("failed"))
}

fn chrome_native_host_manifest_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "Could not resolve HOME directory.".to_string())?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("Google")
        .join("Chrome")
        .join("NativeMessagingHosts")
        .join(format!("{CHROME_NATIVE_HOST_NAME}.json")))
}

fn chrome_native_host_binary_path() -> Result<PathBuf, String> {
    let app_dir = app_root_dir()?;
    let same_dir = app_dir.join("paper_manager_native_host");
    if same_dir.exists() {
        return Ok(same_dir);
    }

    let bundled_resource = app_dir
        .join("..")
        .join("Resources")
        .join("paper_manager_native_host");
    if bundled_resource.exists() {
        return fs::canonicalize(&bundled_resource).map_err(|error| error.to_string());
    }

    let development_path = app_dir
        .join("..")
        .join("debug")
        .join("paper_manager_native_host");
    if development_path.exists() {
        return fs::canonicalize(&development_path).map_err(|error| error.to_string());
    }

    Ok(same_dir)
}

fn normalize_chrome_extension_id(value: Option<String>) -> Result<String, String> {
    let extension_id = value
        .or_else(|| {
            load_or_default_app_data()
                .ok()
                .and_then(|data| data.settings.chrome_extension_id)
        })
        .unwrap_or_default()
        .trim()
        .to_string();

    if extension_id.is_empty() {
        return Err("Enter the Chrome extension ID before installing the Native Host.".to_string());
    }

    if extension_id.len() != 32
        || !extension_id
            .chars()
            .all(|character| matches!(character, 'a'..='p'))
    {
        return Err("Chrome extension ID must be 32 lowercase characters from a to p.".to_string());
    }

    Ok(extension_id)
}

fn ensure_setting_dir() -> Result<PathBuf, String> {
    let dir = setting_dir()?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn now_id() -> Result<String, String> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    Ok(format!("paper-{millis}"))
}

fn now_note_id() -> Result<String, String> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    Ok(format!("note-{millis}"))
}

fn current_timestamp() -> Result<String, String> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    Ok(format!("{seconds}"))
}

fn default_filename_rule() -> String {
    "{year}_{first_author}_{journal}.pdf".to_string()
}

fn default_bibtex_key_rule() -> String {
    String::new()
}

fn default_bibtex_export_rule() -> String {
    "doi_preferred".to_string()
}

fn default_journal_output_style() -> String {
    "as_stored".to_string()
}

fn journal_alias(full_name: &str, abbreviation: &str, aliases: &[&str]) -> JournalAlias {
    JournalAlias {
        full_name: full_name.to_string(),
        abbreviation: abbreviation.to_string(),
        aliases: aliases.iter().map(|alias| alias.to_string()).collect(),
    }
}

fn default_journal_aliases() -> Vec<JournalAlias> {
    vec![
        journal_alias("Physical Review Letters", "Phys. Rev. Lett.", &["PRL"]),
        journal_alias("Physical Review A", "Phys. Rev. A", &["PRA", "Phys Rev A"]),
        journal_alias("Physical Review B", "Phys. Rev. B", &["PRB", "Phys Rev B"]),
        journal_alias("Physical Review C", "Phys. Rev. C", &["PRC", "Phys Rev C"]),
        journal_alias("Physical Review D", "Phys. Rev. D", &["PRD", "Phys Rev D"]),
        journal_alias("Physical Review E", "Phys. Rev. E", &["PRE", "Phys Rev E"]),
        journal_alias("Physical Review X", "Phys. Rev. X", &["PRX", "Phys Rev X"]),
        journal_alias(
            "Physical Review Research",
            "Phys. Rev. Res.",
            &["PRResearch", "Phys Rev Res"],
        ),
        journal_alias(
            "Physical Review Applied",
            "Phys. Rev. Applied",
            &["PR Applied", "Phys Rev Applied"],
        ),
        journal_alias(
            "Physical Review Materials",
            "Phys. Rev. Mater.",
            &["PR Materials", "Phys Rev Mater"],
        ),
        journal_alias(
            "Physical Review Fluids",
            "Phys. Rev. Fluids",
            &["PR Fluids", "Phys Rev Fluids"],
        ),
        journal_alias("Reviews of Modern Physics", "Rev. Mod. Phys.", &["RMP"]),
        journal_alias("Journal of Applied Physics", "J. Appl. Phys.", &["JAP"]),
        journal_alias("Applied Physics Letters", "Appl. Phys. Lett.", &["APL"]),
        journal_alias(
            "Review of Scientific Instruments",
            "Rev. Sci. Instrum.",
            &["RSI"],
        ),
        journal_alias(
            "The Journal of Chemical Physics",
            "J. Chem. Phys.",
            &["JCP"],
        ),
        journal_alias("Applied Physics Reviews", "Appl. Phys. Rev.", &["APR"]),
        journal_alias("Physics of Fluids", "Phys. Fluids", &[]),
        journal_alias("American Journal of Physics", "Am. J. Phys.", &["AJP"]),
        journal_alias("New Journal of Physics", "New J. Phys.", &["NJP"]),
        journal_alias(
            "Journal of Physics A: Mathematical and Theoretical",
            "J. Phys. A",
            &[],
        ),
        journal_alias(
            "Journal of Physics B: Atomic, Molecular and Optical Physics",
            "J. Phys. B",
            &[],
        ),
        journal_alias(
            "Journal of Physics: Condensed Matter",
            "J. Phys.: Condens. Matter",
            &[],
        ),
        journal_alias("Journal of Physics D: Applied Physics", "J. Phys. D", &[]),
        journal_alias(
            "Journal of High Energy Physics",
            "J. High Energy Phys.",
            &["JHEP"],
        ),
        journal_alias(
            "Classical and Quantum Gravity",
            "Class. Quantum Gravity",
            &["CQG"],
        ),
        journal_alias(
            "Plasma Physics and Controlled Fusion",
            "Plasma Phys. Control. Fusion",
            &[],
        ),
        journal_alias("Nature Physics", "Nat. Phys.", &[]),
        journal_alias("Nature Materials", "Nat. Mater.", &[]),
        journal_alias("Nature Nanotechnology", "Nat. Nanotechnol.", &[]),
        journal_alias("Nature Communications", "Nat. Commun.", &[]),
        journal_alias("Communications Physics", "Commun. Phys.", &[]),
        journal_alias("Science", "Science", &[]),
        journal_alias("Science Advances", "Sci. Adv.", &[]),
        journal_alias(
            "Proceedings of the National Academy of Sciences",
            "Proc. Natl. Acad. Sci. U.S.A.",
            &["PNAS"],
        ),
        journal_alias("Nuclear Physics B", "Nucl. Phys. B", &[]),
        journal_alias("Physics Letters A", "Phys. Lett. A", &[]),
        journal_alias("Physics Letters B", "Phys. Lett. B", &[]),
        journal_alias("Physics Reports", "Phys. Rep.", &[]),
        journal_alias("Annals of Physics", "Ann. Phys.", &[]),
        journal_alias("Solid State Communications", "Solid State Commun.", &[]),
        journal_alias(
            "Journal of Magnetism and Magnetic Materials",
            "J. Magn. Magn. Mater.",
            &["JMMM"],
        ),
        journal_alias("Physica B: Condensed Matter", "Physica B", &[]),
        journal_alias(
            "Physica C: Superconductivity and its Applications",
            "Physica C",
            &[],
        ),
        journal_alias("European Physical Journal B", "Eur. Phys. J. B", &["EPJ B"]),
        journal_alias("European Physical Journal C", "Eur. Phys. J. C", &["EPJ C"]),
        journal_alias("Journal of Statistical Physics", "J. Stat. Phys.", &[]),
        journal_alias("Quantum", "Quantum", &[]),
        journal_alias("npj Quantum Information", "npj Quantum Inf.", &[]),
        journal_alias("2D Materials", "2D Mater.", &[]),
    ]
}

fn default_cloud_sync_expected() -> bool {
    true
}

fn default_settings(timestamp: &str) -> Settings {
    Settings {
        id: "settings-default".to_string(),
        managed_directory: None,
        filename_rule: default_filename_rule(),
        note_directory: None,
        marktext_path: None,
        pdf_viewer_path: None,
        chrome_import_directory: None,
        chrome_extension_id: None,
        bibtex_key_rule: default_bibtex_key_rule(),
        bibtex_export_rule: default_bibtex_export_rule(),
        journal_output_style: default_journal_output_style(),
        journal_aliases: default_journal_aliases(),
        cloud_sync_expected: true,
        workspace_root: None,
        workspace_revision: None,
        workspace_last_loaded_revision: None,
        created_at: timestamp.to_string(),
        updated_at: timestamp.to_string(),
    }
}

fn empty_app_data() -> Result<AppData, String> {
    let timestamp = current_timestamp()?;
    Ok(AppData {
        papers: Vec::new(),
        notes: Vec::new(),
        settings: default_settings(&timestamp),
    })
}

fn load_or_default_app_data() -> Result<AppData, String> {
    let data_file = data_file_path()?;
    if !data_file.exists() {
        return empty_app_data();
    }

    let json = fs::read_to_string(&data_file).map_err(|error| error.to_string())?;
    let local_data: AppData = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    let Some(root) = active_workspace_root(&local_data) else {
        return Ok(local_data);
    };

    let workspace_file = workspace_data_file(&root);
    if !workspace_file.exists() {
        return Ok(local_data);
    }

    let workspace_json = fs::read_to_string(&workspace_file).map_err(|error| error.to_string())?;
    let mut workspace_data: AppData =
        serde_json::from_str(&workspace_json).map_err(|error| error.to_string())?;
    let revision = workspace_data.settings.workspace_revision.unwrap_or(0);
    workspace_data.settings.workspace_root = Some(root.to_string_lossy().into_owned());
    workspace_data.settings.workspace_last_loaded_revision = Some(revision);
    Ok(workspace_data)
}

fn save_data_file(data: &AppData) -> Result<AppStatus, String> {
    let dir = ensure_setting_dir()?;
    let data_file = dir.join("app-data.json");
    let mut next_data = data.clone();

    if let Some(root) = active_workspace_root(&next_data) {
        fs::create_dir_all(workspace_meta_dir(&root)).map_err(|error| error.to_string())?;
        fs::create_dir_all(workspace_papers_dir(&root)).map_err(|error| error.to_string())?;
        fs::create_dir_all(workspace_notes_dir(&root)).map_err(|error| error.to_string())?;
        fs::create_dir_all(workspace_exports_dir(&root)).map_err(|error| error.to_string())?;

        let workspace_file = workspace_data_file(&root);
        if workspace_file.exists() {
            let current_json =
                fs::read_to_string(&workspace_file).map_err(|error| error.to_string())?;
            let current_data: AppData =
                serde_json::from_str(&current_json).map_err(|error| error.to_string())?;
            let current_revision = current_data.settings.workspace_revision.unwrap_or(0);
            let loaded_revision = next_data
                .settings
                .workspace_last_loaded_revision
                .unwrap_or(current_revision);
            if current_revision != loaded_revision {
                return Err(
                    "Shared workspace changed on disk. Reload before saving to avoid overwriting collaborators."
                        .to_string(),
                );
            }
            next_data.settings.workspace_revision = Some(current_revision + 1);
        } else {
            next_data.settings.workspace_revision = Some(1);
        }
        next_data.settings.workspace_last_loaded_revision = next_data.settings.workspace_revision;

        let lock = workspace_lock_file(&root);
        if lock.exists() {
            return Err(
                "Shared workspace is locked by another save operation. Try again after syncing."
                    .to_string(),
            );
        }
        fs::write(&lock, current_timestamp()?).map_err(|error| error.to_string())?;
        let workspace_json =
            serde_json::to_string_pretty(&next_data).map_err(|error| error.to_string())?;
        let write_result =
            fs::write(&workspace_file, workspace_json).map_err(|error| error.to_string());
        let _ = fs::remove_file(&lock);
        write_result?;
    }

    let local_json = serde_json::to_string_pretty(&next_data).map_err(|error| error.to_string())?;
    fs::write(&data_file, local_json).map_err(|error| error.to_string())?;

    Ok(AppStatus {
        setting_dir: dir.to_string_lossy().into_owned(),
        data_file: data_file.to_string_lossy().into_owned(),
        data_file_exists: true,
    })
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|inner| {
        let trimmed = inner.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_key(value: &str) -> String {
    value.trim().to_lowercase()
}

fn metadata_http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("paper-manager/0.1.0")
        .build()
        .map_err(|error| error.to_string())
}

fn html_entity_value(entity: &str) -> Option<String> {
    if let Some(hex) = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
    {
        return u32::from_str_radix(hex, 16)
            .ok()
            .and_then(char::from_u32)
            .map(|character| character.to_string());
    }

    if let Some(decimal) = entity.strip_prefix('#') {
        return decimal
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .map(|character| character.to_string());
    }

    let value = match entity {
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" => "'",
        "nbsp" => " ",
        "ndash" => "-",
        "mdash" => "-",
        "minus" => "-",
        "times" => "\\times",
        "alpha" => "\\alpha",
        "beta" => "\\beta",
        "gamma" => "\\gamma",
        "delta" => "\\delta",
        "epsilon" => "\\epsilon",
        "theta" => "\\theta",
        "lambda" => "\\lambda",
        "mu" => "\\mu",
        "nu" => "\\nu",
        "pi" => "\\pi",
        "rho" => "\\rho",
        "sigma" => "\\sigma",
        "tau" => "\\tau",
        "phi" => "\\phi",
        "chi" => "\\chi",
        "psi" => "\\psi",
        "omega" => "\\omega",
        "Alpha" => "\\Alpha",
        "Beta" => "\\Beta",
        "Gamma" => "\\Gamma",
        "Delta" => "\\Delta",
        "Theta" => "\\Theta",
        "Lambda" => "\\Lambda",
        "Pi" => "\\Pi",
        "Sigma" => "\\Sigma",
        "Phi" => "\\Phi",
        "Psi" => "\\Psi",
        "Omega" => "\\Omega",
        _ => return None,
    };
    Some(value.to_string())
}

fn decode_html_entities(value: &str) -> String {
    let mut output = String::new();
    let mut rest = value;

    while let Some(start) = rest.find('&') {
        output.push_str(&rest[..start]);
        let candidate = &rest[start + 1..];
        let Some(end) = candidate.find(';') else {
            output.push_str(&rest[start..]);
            return output;
        };
        let entity = &candidate[..end];
        if entity.len() <= 32 {
            if let Some(decoded) = html_entity_value(entity) {
                output.push_str(&decoded);
            } else {
                output.push('&');
                output.push_str(entity);
                output.push(';');
            }
            rest = &candidate[end + 1..];
        } else {
            output.push('&');
            rest = candidate;
        }
    }

    output.push_str(rest);
    output
}

fn tag_name(tag: &str) -> String {
    tag.trim()
        .trim_start_matches('/')
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_start_matches("mml:")
        .to_lowercase()
}

fn strip_html_tags_preserving_math(value: &str) -> String {
    let mut output = String::new();
    let mut rest = value;

    while let Some(start) = rest.find('<') {
        output.push_str(&rest[..start]);
        let candidate = &rest[start + 1..];
        let Some(end) = candidate.find('>') else {
            output.push_str(&rest[start..]);
            return output;
        };
        let raw_tag = &candidate[..end];
        let closing = raw_tag.trim_start().starts_with('/');
        match tag_name(raw_tag).as_str() {
            "sub" if !closing => output.push_str("_{"),
            "sub" if closing => output.push('}'),
            "sup" if !closing => output.push_str("^{"),
            "sup" if closing => output.push('}'),
            "br" | "p" | "div" if !closing => output.push(' '),
            _ => {}
        }
        rest = &candidate[end + 1..];
    }

    output.push_str(rest);
    output
}

fn clean_text(value: &str) -> String {
    let decoded = decode_html_entities(value);
    let without_tags = strip_html_tags_preserving_math(&decoded);
    without_tags
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_arxiv_id(value: &str) -> String {
    let trimmed = value.trim();
    let without_prefix = trimmed
        .strip_prefix("arXiv:")
        .or_else(|| trimmed.strip_prefix("arxiv:"))
        .unwrap_or(trimmed);
    let without_url = without_prefix
        .trim_start_matches("https://arxiv.org/abs/")
        .trim_start_matches("http://arxiv.org/abs/")
        .trim_start_matches("https://www.arxiv.org/abs/")
        .trim_start_matches("http://www.arxiv.org/abs/")
        .trim_start_matches("https://arxiv.org/pdf/")
        .trim_start_matches("http://arxiv.org/pdf/")
        .trim_start_matches("https://www.arxiv.org/pdf/")
        .trim_start_matches("http://www.arxiv.org/pdf/");
    let without_query = without_url
        .split(['?', '#'])
        .next()
        .unwrap_or(without_url)
        .trim_end_matches(".pdf");

    if let Some((base, version)) = without_query.rsplit_once('v') {
        if !base.is_empty() && version.chars().all(|character| character.is_ascii_digit()) {
            return base.to_string();
        }
    }

    without_query.to_string()
}

fn normalize_doi(value: &str) -> String {
    let trimmed = value.trim();
    let without_url = trimmed
        .trim_start_matches("https://doi.org/")
        .trim_start_matches("http://doi.org/")
        .trim_start_matches("https://dx.doi.org/")
        .trim_start_matches("http://dx.doi.org/")
        .trim_start_matches("doi:")
        .trim_start_matches("DOI:");
    without_url
        .split(['?', '#'])
        .next()
        .unwrap_or(without_url)
        .trim()
        .trim_end_matches('.')
        .to_string()
}

fn looks_like_arxiv(value: &str) -> bool {
    let lower = value.trim().to_lowercase();
    lower.starts_with("arxiv:")
        || lower.contains("arxiv.org/abs/")
        || lower.contains("arxiv.org/pdf/")
        || lower
            .chars()
            .next()
            .map(|character| character.is_ascii_digit())
            .unwrap_or(false)
            && lower.contains('.')
}

fn looks_like_doi(value: &str) -> bool {
    let lower = value.trim().to_lowercase();
    lower.starts_with("10.")
        || lower.starts_with("doi:")
        || lower.contains("doi.org/10.")
        || lower.contains("dx.doi.org/10.")
}

fn resolve_import_identifiers(source: &str) -> Result<(Option<String>, Option<String>), String> {
    let source = source.trim();
    if source.is_empty() {
        return Err("Enter a DOI, arXiv ID, or paper URL.".to_string());
    }

    if looks_like_doi(source) {
        return Ok((Some(normalize_doi(source)), None));
    }

    if looks_like_arxiv(source) {
        return Ok((None, Some(normalize_arxiv_id(source))));
    }

    Err("Could not recognize the input as a DOI or arXiv ID/URL.".to_string())
}

fn first_json_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|field| {
            field
                .as_str()
                .map(str::to_string)
                .or_else(|| field.as_array()?.first()?.as_str().map(str::to_string))
        })
        .map(|inner| clean_text(&inner))
        .filter(|inner| !inner.is_empty())
}

fn crossref_year_and_month(message: &serde_json::Value) -> (Option<u16>, Option<String>) {
    for key in ["published-print", "published-online", "published", "issued"] {
        let Some(parts) = message
            .get(key)
            .and_then(|value| value.get("date-parts"))
            .and_then(|value| value.as_array())
            .and_then(|parts| parts.first())
            .and_then(|part| part.as_array())
        else {
            continue;
        };

        let year = parts
            .first()
            .and_then(|value| value.as_u64())
            .and_then(|value| u16::try_from(value).ok());
        let month = parts
            .get(1)
            .and_then(|value| value.as_u64())
            .and_then(month_name);
        if year.is_some() {
            return (year, month);
        }
    }

    (None, None)
}

fn month_name(month: u64) -> Option<String> {
    let value = match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => return None,
    };
    Some(value.to_string())
}

fn crossref_authors(message: &serde_json::Value) -> Vec<String> {
    message
        .get("author")
        .and_then(|value| value.as_array())
        .map(|authors| {
            authors
                .iter()
                .filter_map(|author| {
                    let given = author.get("given").and_then(|value| value.as_str());
                    let family = author.get("family").and_then(|value| value.as_str());
                    let name = match (given, family) {
                        (Some(given), Some(family)) => format!("{given} {family}"),
                        (Some(given), None) => given.to_string(),
                        (None, Some(family)) => family.to_string(),
                        (None, None) => author
                            .get("name")
                            .and_then(|value| value.as_str())
                            .unwrap_or("")
                            .to_string(),
                    };
                    let name = clean_text(&name);
                    (!name.is_empty()).then_some(name)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn fetch_crossref_metadata(doi: &str) -> Result<PaperMetadata, String> {
    let doi = doi.trim();
    if doi.is_empty() {
        return Err("DOI is required.".to_string());
    }

    let client = metadata_http_client()?;
    let url = format!(
        "https://api.crossref.org/works/{}",
        urlencoding::encode(doi)
    );
    let response = client
        .get(url)
        .send()
        .map_err(|error| format!("Could not fetch DOI metadata: {error}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Could not fetch DOI metadata. HTTP status: {}.",
            response.status()
        ));
    }

    let json = response
        .json::<serde_json::Value>()
        .map_err(|error| format!("Could not parse DOI metadata: {error}"))?;
    let message = json
        .get("message")
        .ok_or_else(|| "Crossref response did not contain metadata.".to_string())?;
    let (year, month) = crossref_year_and_month(message);

    Ok(PaperMetadata {
        title: first_json_string(message, "title"),
        authors: crossref_authors(message),
        year,
        publication: first_json_string(message, "container-title"),
        volume: first_json_string(message, "volume"),
        issue: first_json_string(message, "issue"),
        pages: first_json_string(message, "page"),
        numpages: None,
        month,
        publisher: first_json_string(message, "publisher"),
        doi: first_json_string(message, "DOI").or_else(|| Some(doi.to_string())),
        arxiv_id: None,
        url: first_json_string(message, "URL"),
        abstract_text: first_json_string(message, "abstract"),
    })
}

fn crossref_pdf_url_for_doi(doi: &str) -> Result<Option<String>, String> {
    let doi = doi.trim();
    if doi.is_empty() {
        return Ok(None);
    }

    let client = metadata_http_client()?;
    let url = format!(
        "https://api.crossref.org/works/{}",
        urlencoding::encode(doi)
    );
    let response = client
        .get(url)
        .send()
        .map_err(|error| format!("Could not check DOI PDF links: {error}"))?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let json = response
        .json::<serde_json::Value>()
        .map_err(|error| format!("Could not parse DOI PDF links: {error}"))?;
    let Some(links) = json
        .get("message")
        .and_then(|message| message.get("link"))
        .and_then(|links| links.as_array())
    else {
        return Ok(None);
    };

    Ok(links.iter().find_map(|link| {
        let url = link.get("URL").and_then(|value| value.as_str())?;
        let content_type = link
            .get("content-type")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_lowercase();
        let lower_url = url.to_lowercase();
        (content_type.contains("pdf") || lower_url.ends_with(".pdf")).then(|| url.to_string())
    }))
}

fn child_text<'a>(node: roxmltree::Node<'a, 'a>, child_name: &str) -> Option<String> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == child_name)
        .and_then(|child| child.text())
        .map(clean_text)
        .filter(|value| !value.is_empty())
}

fn fetch_arxiv_metadata(arxiv_id: &str) -> Result<PaperMetadata, String> {
    let arxiv_id = normalize_arxiv_id(arxiv_id);
    if arxiv_id.is_empty() {
        return Err("arXiv ID is required.".to_string());
    }

    let client = metadata_http_client()?;
    let url = format!(
        "https://export.arxiv.org/api/query?id_list={}&max_results=1",
        urlencoding::encode(&arxiv_id)
    );
    let response = client
        .get(url)
        .send()
        .map_err(|error| format!("Could not fetch arXiv metadata: {error}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Could not fetch arXiv metadata. HTTP status: {}.",
            response.status()
        ));
    }

    let xml = response
        .text()
        .map_err(|error| format!("Could not read arXiv metadata: {error}"))?;
    let document = roxmltree::Document::parse(&xml)
        .map_err(|error| format!("Could not parse arXiv metadata: {error}"))?;
    let entry = document
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "entry")
        .ok_or_else(|| "arXiv metadata was not found.".to_string())?;

    let authors = entry
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == "author")
        .filter_map(|author| child_text(author, "name"))
        .collect::<Vec<_>>();
    let published = child_text(entry, "published");
    let year = published
        .as_deref()
        .and_then(|value| value.get(0..4))
        .and_then(|value| value.parse::<u16>().ok());
    let month = published
        .as_deref()
        .and_then(|value| value.get(5..7))
        .and_then(|value| value.parse::<u64>().ok())
        .and_then(month_name);
    let doi = child_text(entry, "doi");
    let journal_ref = child_text(entry, "journal_ref");

    Ok(PaperMetadata {
        title: child_text(entry, "title"),
        authors,
        year,
        publication: journal_ref,
        volume: None,
        issue: None,
        pages: None,
        numpages: None,
        month,
        publisher: None,
        doi,
        arxiv_id: Some(arxiv_id.clone()),
        url: child_text(entry, "id").or_else(|| Some(format!("https://arxiv.org/abs/{arxiv_id}"))),
        abstract_text: child_text(entry, "summary"),
    })
}

fn validate_pdf_path(data: &AppData, pdf_path: &Option<String>) -> Result<(), String> {
    let Some(path) = pdf_path else {
        return Ok(());
    };
    let path = path_for_runtime(data, path);
    let path = path.as_path();

    if !path.exists() {
        return Err("PDF file does not exist.".to_string());
    }

    if !path.is_file() {
        return Err("Selected PDF path is not a file.".to_string());
    }

    let is_pdf = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false);

    if !is_pdf {
        return Err("Selected file is not a PDF.".to_string());
    }

    Ok(())
}

fn sanitize_filename(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let trimmed = sanitized.trim_matches('_');

    if trimmed.is_empty() {
        "note".to_string()
    } else {
        trimmed.to_string()
    }
}

fn author_last_name(author: &str) -> &str {
    author.split_whitespace().last().unwrap_or(author)
}

fn doi_suffix(doi: Option<&str>) -> Option<String> {
    doi.and_then(|value| value.split('/').next_back())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn template_value(paper: &Paper, token: &str) -> String {
    match token {
        "id" => paper.id.clone(),
        "title" => paper.title.clone(),
        "year" => paper
            .year
            .map(|year| year.to_string())
            .unwrap_or_else(|| "unknown_year".to_string()),
        "first_author" => paper
            .authors
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown_author".to_string()),
        "last_name" => paper
            .authors
            .first()
            .map(|author| author_last_name(author).to_string())
            .unwrap_or_else(|| "paper".to_string()),
        "journal" | "publication" => paper.publication.clone().unwrap_or_else(|| {
            if paper.arxiv_id.is_some() {
                "preprint".to_string()
            } else {
                "unknown_journal".to_string()
            }
        }),
        "doi" => paper.doi.clone().unwrap_or_default(),
        "doi_suffix" => doi_suffix(paper.doi.as_deref()).unwrap_or_default(),
        "arxiv_id" => paper.arxiv_id.clone().unwrap_or_default(),
        "volume" => paper.volume.clone().unwrap_or_default(),
        "issue" => paper.issue.clone().unwrap_or_default(),
        "pages" => paper.pages.clone().unwrap_or_default(),
        "month" => paper.month.clone().unwrap_or_default(),
        "publisher" => paper.publisher.clone().unwrap_or_default(),
        "status" => paper.status.clone().unwrap_or_default(),
        "category" => paper.folder_category.clone().unwrap_or_default(),
        _ => String::new(),
    }
}

fn render_paper_template(rule: &str, paper: &Paper) -> String {
    let mut output = String::new();
    let mut chars = rule.chars().peekable();

    while let Some(character) = chars.next() {
        if character != '{' {
            output.push(character);
            continue;
        }

        let mut token = String::new();
        let mut closed = false;
        for next in chars.by_ref() {
            if next == '}' {
                closed = true;
                break;
            }
            token.push(next);
        }

        if closed {
            output.push_str(&template_value(paper, token.trim()));
        } else {
            output.push('{');
            output.push_str(&token);
        }
    }

    output
}

fn render_pdf_file_name(rule: &str, paper: &Paper) -> String {
    let rule = if rule.trim().is_empty() {
        default_filename_rule()
    } else {
        rule.trim().to_string()
    };
    let rendered = render_paper_template(&rule, paper);
    let stem = rendered
        .strip_suffix(".pdf")
        .or_else(|| rendered.strip_suffix(".PDF"))
        .unwrap_or(&rendered);
    format!("{}.pdf", sanitize_filename(stem))
}

fn sanitize_category_path(category: &str) -> Option<PathBuf> {
    let parts = category
        .split(['/', '\\'])
        .map(str::trim)
        .filter(|part| !part.is_empty() && *part != "." && *part != "..")
        .map(sanitize_filename)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    if parts.is_empty() {
        return None;
    }

    let mut path = PathBuf::new();
    for part in parts {
        path.push(part);
    }
    Some(path)
}

fn target_pdf_path(
    data: &AppData,
    paper: &Paper,
    folder_category: Option<&str>,
) -> Result<PathBuf, String> {
    let target_dir = target_pdf_dir(data, folder_category)?;
    Ok(unique_path(
        &target_dir,
        &render_pdf_file_name(&data.settings.filename_rule, paper),
    ))
}

fn target_pdf_dir(data: &AppData, folder_category: Option<&str>) -> Result<PathBuf, String> {
    let managed_directory = if let Some(root) = active_workspace_root(data) {
        workspace_papers_dir(&root)
    } else {
        let managed_directory = data
            .settings
            .managed_directory
            .as_deref()
            .ok_or_else(|| "Managed directory is not set.".to_string())?;
        PathBuf::from(managed_directory)
    };

    if !managed_directory.exists() {
        fs::create_dir_all(&managed_directory).map_err(|error| error.to_string())?;
    }

    if !managed_directory.is_dir() {
        return Err("Managed directory is not a directory.".to_string());
    }

    let category = normalize_optional(folder_category.map(str::to_string));
    let target_dir = if let Some(category) = category {
        if let Some(category_path) = sanitize_category_path(&category) {
            managed_directory.join(category_path)
        } else {
            managed_directory
        }
    } else {
        managed_directory
    };
    fs::create_dir_all(&target_dir).map_err(|error| error.to_string())?;
    Ok(target_dir)
}

fn unique_path(target_dir: &Path, file_name: &str) -> PathBuf {
    let source_name = Path::new(file_name);
    let stem = source_name
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    let extension = source_name.extension().and_then(|value| value.to_str());
    let mut candidate = target_dir.join(file_name);
    let mut index = 2;

    while candidate.exists() {
        let indexed_name = if let Some(extension) = extension {
            format!("{stem}_{index}.{extension}")
        } else {
            format!("{stem}_{index}")
        };
        candidate = target_dir.join(indexed_name);
        index += 1;
    }

    candidate
}

fn imports_dir() -> Result<PathBuf, String> {
    if let Ok(data) = load_or_default_app_data() {
        if let Some(path) = data.settings.chrome_import_directory.as_deref() {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed));
            }
        }
    }

    Ok(setting_dir()?.join("imports"))
}

fn download_pdf(url: &str, file_stem: &str) -> Result<PathBuf, String> {
    let client = metadata_http_client()?;
    let response = client
        .get(url)
        .send()
        .map_err(|error| format!("Could not download PDF: {error}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Could not download PDF. HTTP status: {}.",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .map_err(|error| format!("Could not read PDF download: {error}"))?;
    if !bytes.starts_with(b"%PDF") {
        return Err("Downloaded file was not a PDF.".to_string());
    }

    let dir = imports_dir()?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let file_name = format!("{}.pdf", sanitize_filename(file_stem));
    let target = unique_path(&dir, &file_name);
    fs::write(&target, &bytes).map_err(|error| error.to_string())?;
    Ok(target)
}

fn download_arxiv_pdf(arxiv_id: &str) -> Result<PathBuf, String> {
    let arxiv_id = normalize_arxiv_id(arxiv_id);
    let url = format!("https://arxiv.org/pdf/{arxiv_id}.pdf");
    download_pdf(&url, &format!("arxiv_{arxiv_id}"))
}

fn download_crossref_pdf(doi: &str) -> Result<Option<PathBuf>, String> {
    let Some(url) = crossref_pdf_url_for_doi(doi)? else {
        return Ok(None);
    };
    let file_stem = format!("doi_{}", normalize_doi(doi));
    download_pdf(&url, &file_stem).map(Some)
}

fn move_file_with_fallback(source: &Path, target: &Path) -> Result<(), String> {
    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(source, target).map_err(|error| error.to_string())?;
            fs::remove_file(source).map_err(|error| {
                let _ = fs::remove_file(target);
                error.to_string()
            })
        }
    }
}

fn organize_pdf_for_paper(
    data: &mut AppData,
    paper_index: usize,
    folder_category: Option<String>,
) -> Result<(), String> {
    let workspace_root = active_workspace_root(data);
    let current_pdf_path = data.papers[paper_index]
        .pdf_path
        .as_deref()
        .ok_or_else(|| "This paper does not have a PDF path.".to_string())?;
    let current_pdf_path = path_for_runtime(data, current_pdf_path);

    if !current_pdf_path.exists() {
        return Err("PDF file does not exist.".to_string());
    }

    if !current_pdf_path.is_file() {
        return Err("PDF path is not a file.".to_string());
    }

    let target = target_pdf_path(data, &data.papers[paper_index], folder_category.as_deref())?;
    let target_dir = target_pdf_dir(data, folder_category.as_deref())?;
    let target_notes_dir = target_dir.join("notes");

    move_file_with_fallback(&current_pdf_path, &target)?;
    fs::create_dir_all(&target_notes_dir).map_err(|error| error.to_string())?;

    let paper_id = data.papers[paper_index].id.clone();
    for note in data
        .notes
        .iter_mut()
        .filter(|note| note.paper_id == paper_id)
    {
        let current_note_path = {
            let path = PathBuf::from(&note.file_path);
            if path.is_absolute() {
                path
            } else if let Some(root) = workspace_root.as_ref() {
                root.join(path)
            } else {
                path
            }
        };
        if !current_note_path.exists() || !current_note_path.is_file() {
            continue;
        }

        let file_name = current_note_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("note.md");
        let target_note_path = unique_path(&target_notes_dir, file_name);
        move_file_with_fallback(&current_note_path, &target_note_path)?;
        note.file_path = if let Some(root) = workspace_root.as_ref() {
            target_note_path
                .strip_prefix(root)
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|_| target_note_path.to_string_lossy().into_owned())
        } else {
            target_note_path.to_string_lossy().into_owned()
        };
        note.updated_at = current_timestamp()?;
    }

    let timestamp = current_timestamp()?;
    let paper = &mut data.papers[paper_index];
    paper.pdf_path = Some(if let Some(root) = workspace_root.as_ref() {
        target
            .strip_prefix(root)
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|_| target.to_string_lossy().into_owned())
    } else {
        target.to_string_lossy().into_owned()
    });
    paper.folder_category = folder_category;
    paper.updated_at = timestamp;
    Ok(())
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<(), String> {
    if !source.exists() {
        return Ok(());
    }

    if source.is_file() {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::copy(source, target).map_err(|error| error.to_string())?;
        return Ok(());
    }

    fs::create_dir_all(target).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        copy_dir_recursive(&source_path, &target_path)?;
    }

    Ok(())
}

fn collect_files_by_basename(
    root: &Path,
    files: &mut HashMap<String, PathBuf>,
) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }

    if root.is_file() {
        if let Some(file_name) = root.file_name().and_then(|value| value.to_str()) {
            files
                .entry(file_name.to_string())
                .or_insert_with(|| root.to_path_buf());
        }
        return Ok(());
    }

    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        collect_files_by_basename(&entry.path(), files)?;
    }

    Ok(())
}

fn relink_paths_to_root(data: &mut AppData, root: &Path) -> Result<(), String> {
    let mut files_by_name = HashMap::new();
    collect_files_by_basename(root, &mut files_by_name)?;

    for paper in &mut data.papers {
        if let Some(pdf_path) = paper.pdf_path.as_deref() {
            if !Path::new(pdf_path).exists() {
                if let Some(file_name) = Path::new(pdf_path)
                    .file_name()
                    .and_then(|value| value.to_str())
                {
                    if let Some(relinked) = files_by_name.get(file_name) {
                        paper.pdf_path = Some(relinked.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }

    for note in &mut data.notes {
        if !Path::new(&note.file_path).exists() {
            if let Some(file_name) = Path::new(&note.file_path)
                .file_name()
                .and_then(|value| value.to_str())
            {
                if let Some(relinked) = files_by_name.get(file_name) {
                    note.file_path = relinked.to_string_lossy().into_owned();
                }
            }
        }
    }

    Ok(())
}

fn relativize_paths_to_workspace(data: &mut AppData) {
    let Some(root) = active_workspace_root(data) else {
        return;
    };

    for paper in &mut data.papers {
        if let Some(pdf_path) = paper.pdf_path.as_mut() {
            let absolute = PathBuf::from(pdf_path.as_str());
            if let Ok(relative) = absolute.strip_prefix(&root) {
                *pdf_path = relative.to_string_lossy().into_owned();
            }
        }
        if let Some(original_pdf_path) = paper.original_pdf_path.as_mut() {
            let absolute = PathBuf::from(original_pdf_path.as_str());
            if let Ok(relative) = absolute.strip_prefix(&root) {
                *original_pdf_path = relative.to_string_lossy().into_owned();
            }
        }
    }

    for note in &mut data.notes {
        let absolute = PathBuf::from(&note.file_path);
        if let Ok(relative) = absolute.strip_prefix(&root) {
            note.file_path = relative.to_string_lossy().into_owned();
        }
    }
}

fn ensure_workspace_dirs(root: &Path) -> Result<(), String> {
    fs::create_dir_all(workspace_meta_dir(root)).map_err(|error| error.to_string())?;
    fs::create_dir_all(workspace_papers_dir(root)).map_err(|error| error.to_string())?;
    fs::create_dir_all(workspace_notes_dir(root)).map_err(|error| error.to_string())?;
    fs::create_dir_all(workspace_exports_dir(root)).map_err(|error| error.to_string())?;
    Ok(())
}

fn bibtex_escape(value: &str) -> String {
    value.replace('\n', " ").replace('\r', " ")
}

fn normalize_journal_key(value: &str) -> String {
    value
        .chars()
        .filter_map(|character| {
            if character.is_ascii_alphanumeric() {
                Some(character.to_ascii_lowercase())
            } else if character.is_whitespace() || character == '&' {
                Some(' ')
            } else {
                None
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_journal_for_bibtex(publication: Option<&str>, settings: &Settings) -> Option<String> {
    let publication = publication?.trim();
    if publication.is_empty() {
        return None;
    }

    match settings.journal_output_style.as_str() {
        "full" | "abbreviation" => {}
        _ => return Some(publication.to_string()),
    }

    let key = normalize_journal_key(publication);
    if key.is_empty() {
        return Some(publication.to_string());
    }

    for alias in &settings.journal_aliases {
        let candidates = std::iter::once(alias.full_name.as_str())
            .chain(std::iter::once(alias.abbreviation.as_str()))
            .chain(alias.aliases.iter().map(String::as_str));

        if candidates
            .filter(|candidate| !candidate.trim().is_empty())
            .any(|candidate| normalize_journal_key(candidate) == key)
        {
            let preferred = if settings.journal_output_style == "full" {
                alias.full_name.trim()
            } else {
                alias.abbreviation.trim()
            };

            if !preferred.is_empty() {
                return Some(preferred.to_string());
            }
        }
    }

    Some(publication.to_string())
}

fn bibtex_key(paper: &Paper, settings: &Settings) -> String {
    if let Some(key) = paper.bibtex_key.as_deref() {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            return sanitize_filename(trimmed);
        }
    }

    let rule = settings.bibtex_key_rule.trim();
    if !rule.is_empty() {
        let rendered = sanitize_filename(&render_paper_template(rule, paper));
        if !rendered.is_empty() {
            return rendered;
        }
    }

    if let Some(doi) = paper.doi.as_deref() {
        let doi_key = doi_suffix(Some(doi))
            .map(|suffix| sanitize_filename(&suffix))
            .filter(|key| !key.is_empty());
        if let Some(key) = doi_key {
            return key;
        }
    }

    if paper.arxiv_id.is_some() {
        let first_author = paper
            .authors
            .first()
            .map(|author| author.split_whitespace().last().unwrap_or(author))
            .unwrap_or("paper");
        let year = paper
            .year
            .map(|year| year.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        return sanitize_filename(&format!("{first_author}-{year}"));
    }

    let first_author = paper
        .authors
        .first()
        .map(|author| author.split_whitespace().last().unwrap_or(author))
        .unwrap_or("paper");
    let year = paper
        .year
        .map(|year| year.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    sanitize_filename(&format!("{first_author}{year}"))
}

fn push_bibtex_field(fields: &mut Vec<String>, name: &str, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }

    fields.push(format!("  {name} = {{{}}}", bibtex_escape(trimmed)));
}

fn paper_to_bibtex(paper: &Paper, settings: &Settings) -> String {
    let mut fields = Vec::new();
    push_bibtex_field(&mut fields, "title", Some(&paper.title));
    if !paper.authors.is_empty() {
        fields.push(format!(
            "  author = {{{}}}",
            paper
                .authors
                .iter()
                .map(|author| bibtex_escape(author))
                .collect::<Vec<_>>()
                .join(" and ")
        ));
    }

    if paper.doi.is_none() && paper.arxiv_id.is_some() {
        if let Some(year) = paper.year {
            fields.push(format!("  year = {{{year}}}"));
        }
        push_bibtex_field(&mut fields, "eprint", paper.arxiv_id.as_deref());
        fields.push("  archivePrefix = {arXiv}".to_string());

        return format!(
            "@misc{{{},\n{}\n}}",
            bibtex_key(paper, settings),
            fields.join(",\n")
        );
    }

    let journal = normalize_journal_for_bibtex(paper.publication.as_deref(), settings);
    push_bibtex_field(&mut fields, "journal", journal.as_deref());
    push_bibtex_field(&mut fields, "volume", paper.volume.as_deref());
    push_bibtex_field(&mut fields, "issue", paper.issue.as_deref());
    push_bibtex_field(&mut fields, "pages", paper.pages.as_deref());
    if let Some(numpages) = paper.numpages {
        fields.push(format!("  numpages = {{{numpages}}}"));
    }
    if let Some(year) = paper.year {
        fields.push(format!("  year = {{{year}}}"));
    }
    push_bibtex_field(&mut fields, "month", paper.month.as_deref());
    push_bibtex_field(&mut fields, "publisher", paper.publisher.as_deref());
    push_bibtex_field(&mut fields, "doi", paper.doi.as_deref());
    push_bibtex_field(&mut fields, "url", paper.url.as_deref());

    format!(
        "@article{{{},\n{}\n}}",
        bibtex_key(paper, settings),
        fields.join(",\n")
    )
}

fn note_title_from_path(file_path: &Path) -> String {
    file_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Untitled note")
        .to_string()
}

fn ensure_paper_exists(data: &AppData, paper_id: &str) -> Result<(), String> {
    if data.papers.iter().any(|paper| paper.id == paper_id) {
        Ok(())
    } else {
        Err("Paper was not found.".to_string())
    }
}

fn note_file_type(file_path: &Path) -> Option<String> {
    file_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_lowercase())
}

fn is_markdown_path(file_path: &Path) -> bool {
    file_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| matches!(extension.to_lowercase().as_str(), "md" | "markdown"))
        .unwrap_or(false)
}

fn ensure_not_duplicate(
    data: &AppData,
    id: Option<&str>,
    title: &str,
    doi: &Option<String>,
    pdf_path: &Option<String>,
) -> Result<(), String> {
    let input_doi = doi.as_deref().map(normalize_key);
    let input_title = normalize_key(title);
    let input_pdf_path = pdf_path.as_deref().map(normalize_key);

    for paper in &data.papers {
        if id.is_some_and(|target_id| target_id == paper.id) {
            continue;
        }

        if let (Some(existing), Some(next)) = (paper.doi.as_deref().map(normalize_key), &input_doi)
        {
            if existing == *next {
                return Err("A paper with the same DOI already exists.".to_string());
            }
        }

        if normalize_key(&paper.title) == input_title {
            return Err("A paper with the same title already exists.".to_string());
        }

        if let (Some(existing), Some(next)) = (
            paper.pdf_path.as_deref().map(normalize_key),
            &input_pdf_path,
        ) {
            if existing == *next {
                return Err("A paper with the same PDF path already exists.".to_string());
            }
        }
    }

    Ok(())
}

fn register_paper_input(input: RegisterPaperInput) -> Result<AppData, String> {
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err("Title is required.".to_string());
    }

    let authors = input
        .authors
        .iter()
        .map(|author| author.trim().to_string())
        .filter(|author| !author.is_empty())
        .collect::<Vec<_>>();

    if authors.is_empty() {
        return Err("At least one author is required.".to_string());
    }

    let mut data = load_or_default_app_data()?;
    validate_pdf_path(&data, &input.pdf_path)?;
    ensure_not_duplicate(&data, None, &input.title, &input.doi, &input.pdf_path)?;

    let timestamp = current_timestamp()?;
    let pdf_path = normalize_optional(input.pdf_path);
    let paper = Paper {
        id: now_id()?,
        title,
        authors,
        year: input.year,
        publication: normalize_optional(input.publication),
        volume: normalize_optional(input.volume),
        issue: normalize_optional(input.issue),
        pages: normalize_optional(input.pages),
        numpages: input.numpages,
        month: normalize_optional(input.month),
        publisher: normalize_optional(input.publisher),
        doi: normalize_optional(input.doi),
        arxiv_id: normalize_optional(input.arxiv_id),
        url: normalize_optional(input.url),
        abstract_text: normalize_optional(input.abstract_text),
        tags: input
            .tags
            .iter()
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .collect(),
        status: normalize_optional(input.status),
        rating: None,
        bibtex_key: None,
        pdf_path: pdf_path.clone(),
        original_pdf_path: pdf_path,
        folder_category: normalize_optional(input.folder_category),
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    data.papers.push(paper);
    let paper_index = data.papers.len() - 1;
    if data.papers[paper_index].pdf_path.is_some()
        && (data.settings.managed_directory.is_some() || data.settings.workspace_root.is_some())
    {
        let folder_category = data.papers[paper_index].folder_category.clone();
        organize_pdf_for_paper(&mut data, paper_index, folder_category)?;
    }
    save_data_file(&data)?;
    Ok(data)
}

fn merge_tags(existing: &mut Vec<String>, incoming: Vec<String>) {
    for tag in incoming {
        let tag = tag.trim();
        if !tag.is_empty() && !existing.iter().any(|existing_tag| existing_tag == tag) {
            existing.push(tag.to_string());
        }
    }
}

fn register_or_update_import_input_into_data(
    data: &mut AppData,
    input: RegisterPaperInput,
) -> Result<String, String> {
    validate_pdf_path(&data, &input.pdf_path)?;
    let input_doi = input.doi.as_deref().map(normalize_key);
    let input_title = normalize_key(&input.title);
    let existing_index = data.papers.iter().position(|paper| {
        input_doi
            .as_ref()
            .is_some_and(|doi| paper.doi.as_deref().map(normalize_key).as_ref() == Some(doi))
            || normalize_key(&paper.title) == input_title
    });

    let Some(paper_index) = existing_index else {
        ensure_not_duplicate(data, None, &input.title, &input.doi, &input.pdf_path)?;
        let timestamp = current_timestamp()?;
        let pdf_path = normalize_optional(input.pdf_path);
        let paper = Paper {
            id: now_id()?,
            title: input.title.trim().to_string(),
            authors: input
                .authors
                .iter()
                .map(|author| author.trim().to_string())
                .filter(|author| !author.is_empty())
                .collect(),
            year: input.year,
            publication: normalize_optional(input.publication),
            volume: normalize_optional(input.volume),
            issue: normalize_optional(input.issue),
            pages: normalize_optional(input.pages),
            numpages: input.numpages,
            month: normalize_optional(input.month),
            publisher: normalize_optional(input.publisher),
            doi: normalize_optional(input.doi),
            arxiv_id: normalize_optional(input.arxiv_id),
            url: normalize_optional(input.url),
            abstract_text: normalize_optional(input.abstract_text),
            tags: input
                .tags
                .iter()
                .map(|tag| tag.trim().to_string())
                .filter(|tag| !tag.is_empty())
                .collect(),
            status: normalize_optional(input.status),
            rating: None,
            bibtex_key: None,
            pdf_path: pdf_path.clone(),
            original_pdf_path: pdf_path,
            folder_category: normalize_optional(input.folder_category),
            created_at: timestamp.clone(),
            updated_at: timestamp,
        };
        if paper.title.is_empty() {
            return Err("Title is required.".to_string());
        }
        if paper.authors.is_empty() {
            return Err("At least one author is required.".to_string());
        }

        data.papers.push(paper);
        let paper_index = data.papers.len() - 1;
        if data.papers[paper_index].pdf_path.is_some()
            && (data.settings.managed_directory.is_some() || data.settings.workspace_root.is_some())
        {
            let folder_category = data.papers[paper_index].folder_category.clone();
            organize_pdf_for_paper(data, paper_index, folder_category)?;
        }
        return Ok(data.papers[paper_index].title.clone());
    };

    let timestamp = current_timestamp()?;
    let mut added_pdf = false;
    {
        let paper = &mut data.papers[paper_index];
        if paper.authors.is_empty() && !input.authors.is_empty() {
            paper.authors = input.authors.clone();
        }
        if paper.year.is_none() {
            paper.year = input.year;
        }
        if paper.publication.is_none() {
            paper.publication = normalize_optional(input.publication.clone());
        }
        if paper.volume.is_none() {
            paper.volume = normalize_optional(input.volume.clone());
        }
        if paper.issue.is_none() {
            paper.issue = normalize_optional(input.issue.clone());
        }
        if paper.pages.is_none() {
            paper.pages = normalize_optional(input.pages.clone());
        }
        if paper.numpages.is_none() {
            paper.numpages = input.numpages;
        }
        if paper.month.is_none() {
            paper.month = normalize_optional(input.month.clone());
        }
        if paper.publisher.is_none() {
            paper.publisher = normalize_optional(input.publisher.clone());
        }
        if paper.doi.is_none() {
            paper.doi = normalize_optional(input.doi.clone());
        }
        if paper.arxiv_id.is_none() {
            paper.arxiv_id = normalize_optional(input.arxiv_id.clone());
        }
        if paper.url.is_none() {
            paper.url = normalize_optional(input.url.clone());
        }
        if paper.abstract_text.is_none() {
            paper.abstract_text = normalize_optional(input.abstract_text.clone());
        }
        if paper.folder_category.is_none() {
            paper.folder_category = normalize_optional(input.folder_category.clone());
        }
        if paper.pdf_path.is_none() {
            let pdf_path = normalize_optional(input.pdf_path.clone());
            added_pdf = pdf_path.is_some();
            paper.pdf_path = pdf_path.clone();
            paper.original_pdf_path = pdf_path;
        }
        merge_tags(&mut paper.tags, input.tags.clone());
        paper.updated_at = timestamp;
    }

    if added_pdf
        && (data.settings.managed_directory.is_some() || data.settings.workspace_root.is_some())
    {
        let folder_category = data.papers[paper_index].folder_category.clone();
        organize_pdf_for_paper(data, paper_index, folder_category)?;
    }

    Ok(data.papers[paper_index].title.clone())
}

fn metadata_for_import_request(
    request: &ExtensionImportRequest,
) -> Result<Option<PaperMetadata>, String> {
    let has_identifier = request
        .doi
        .as_ref()
        .map(|doi| !doi.trim().is_empty())
        .unwrap_or(false)
        || request
            .arxiv_id
            .as_ref()
            .map(|arxiv_id| !arxiv_id.trim().is_empty())
            .unwrap_or(false);

    if !has_identifier {
        return Ok(None);
    }

    if let Some(doi) = request.doi.as_deref().map(normalize_doi) {
        if !doi.is_empty() {
            match fetch_crossref_metadata(&doi) {
                Ok(metadata) => return Ok(Some(metadata)),
                Err(error) => {
                    if request
                        .arxiv_id
                        .as_ref()
                        .map(|arxiv_id| !arxiv_id.trim().is_empty())
                        .unwrap_or(false)
                    {
                        // arXiv pages often expose a 10.48550/arXiv.* DOI that may not be in Crossref yet.
                    } else {
                        return Err(error);
                    }
                }
            }
        }
    }

    if let Some(arxiv_id) = request.arxiv_id.as_deref().map(normalize_arxiv_id) {
        if !arxiv_id.is_empty() {
            return fetch_arxiv_metadata(&arxiv_id).map(Some);
        }
    }

    Ok(None)
}

fn valid_import_pdf_path(request: &ExtensionImportRequest) -> Option<String> {
    request.pdf_path.clone().and_then(|pdf_path| {
        let path = Path::new(&pdf_path);
        let is_pdf = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("pdf"))
            .unwrap_or(false);
        (path.exists() && path.is_file() && is_pdf).then_some(pdf_path)
    })
}

fn import_pdf_path(request: &ExtensionImportRequest) -> Option<String> {
    if let Some(pdf_path) = valid_import_pdf_path(request) {
        return Some(pdf_path);
    }

    request
        .arxiv_id
        .as_deref()
        .and_then(|arxiv_id| download_arxiv_pdf(arxiv_id).ok())
        .map(|path| path.to_string_lossy().into_owned())
}

fn import_request_to_register_input(
    request: ExtensionImportRequest,
) -> Result<RegisterPaperInput, String> {
    let metadata = metadata_for_import_request(&request)?;
    let valid_pdf_path = import_pdf_path(&request);
    let title = metadata
        .as_ref()
        .and_then(|metadata| metadata.title.clone())
        .or_else(|| request.title.clone())
        .map(|title| clean_text(&title))
        .filter(|title| !title.is_empty())
        .ok_or_else(|| {
            "Import request did not contain a title and metadata fetch failed.".to_string()
        })?;
    let authors = metadata
        .as_ref()
        .map(|metadata| metadata.authors.clone())
        .filter(|authors| !authors.is_empty())
        .or_else(|| request.authors.clone())
        .map(|authors| {
            authors
                .iter()
                .map(|author| clean_text(author))
                .filter(|author| !author.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|authors| !authors.is_empty())
        .ok_or_else(|| {
            "Import request did not contain authors and metadata fetch failed.".to_string()
        })?;
    let publication = metadata
        .as_ref()
        .and_then(|metadata| metadata.publication.clone())
        .or(request.publication)
        .map(|publication| clean_text(&publication));
    let mut tags = request.tags.unwrap_or_default();
    if !tags.iter().any(|tag| tag == "chrome-import") {
        tags.push("chrome-import".to_string());
    }
    let has_pdf_warning = request
        .import_warnings
        .as_ref()
        .map(|warnings| !warnings.is_empty())
        .unwrap_or(false)
        || (request.pdf_path.is_some() && valid_pdf_path.is_none());
    if has_pdf_warning && !tags.iter().any(|tag| tag == "pdf-missing") {
        tags.push("pdf-missing".to_string());
    }

    Ok(RegisterPaperInput {
        title,
        authors,
        year: metadata
            .as_ref()
            .and_then(|metadata| metadata.year)
            .or(request.year),
        publication,
        volume: metadata
            .as_ref()
            .and_then(|metadata| metadata.volume.clone()),
        issue: metadata
            .as_ref()
            .and_then(|metadata| metadata.issue.clone()),
        pages: metadata
            .as_ref()
            .and_then(|metadata| metadata.pages.clone()),
        numpages: metadata.as_ref().and_then(|metadata| metadata.numpages),
        month: metadata
            .as_ref()
            .and_then(|metadata| metadata.month.clone()),
        publisher: metadata
            .as_ref()
            .and_then(|metadata| metadata.publisher.clone()),
        doi: metadata
            .as_ref()
            .and_then(|metadata| metadata.doi.clone())
            .or_else(|| request.doi.map(|doi| normalize_doi(&doi))),
        arxiv_id: metadata
            .as_ref()
            .and_then(|metadata| metadata.arxiv_id.clone())
            .or_else(|| {
                request
                    .arxiv_id
                    .map(|arxiv_id| normalize_arxiv_id(&arxiv_id))
            }),
        url: metadata
            .as_ref()
            .and_then(|metadata| metadata.url.clone())
            .or(request.source_url),
        abstract_text: metadata
            .as_ref()
            .and_then(|metadata| metadata.abstract_text.clone()),
        tags,
        status: Some("unread".to_string()),
        pdf_path: valid_pdf_path,
        folder_category: request.suggested_category,
    })
}

fn move_import_file(source: &Path, target_dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(target_dir).map_err(|error| error.to_string())?;
    let file_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("import.json");
    let target = unique_path(target_dir, file_name);
    fs::rename(source, &target).map_err(|error| error.to_string())?;
    Ok(target)
}

fn pending_extension_import_files() -> Result<Vec<PathBuf>, String> {
    let pending_dir = extension_import_pending_dir()?;
    if !pending_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = fs::read_dir(pending_dir)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(|extension| extension.eq_ignore_ascii_case("json"))
                    .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

fn process_extension_import_file_into_data(
    data: &mut AppData,
    path: &Path,
) -> Result<String, String> {
    let json = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let request: ExtensionImportRequest =
        serde_json::from_str(&json).map_err(|error| error.to_string())?;
    let title_hint = request
        .title
        .clone()
        .or_else(|| request.doi.clone())
        .or_else(|| request.arxiv_id.clone())
        .unwrap_or_else(|| "import request".to_string());
    let input = import_request_to_register_input(request)?;
    let paper_title = register_or_update_import_input_into_data(data, input)?;
    move_import_file(path, &extension_import_processed_dir()?)?;
    Ok(format!(
        "Imported \"{}\".",
        if paper_title.is_empty() {
            title_hint
        } else {
            paper_title
        }
    ))
}

fn is_extension_download_path(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|part| part == "paper-manager-import")
            .unwrap_or(false)
    })
}

fn cleanup_extension_download_from_request_file(path: &Path) {
    let Ok(json) = fs::read_to_string(path) else {
        return;
    };
    let Ok(request) = serde_json::from_str::<ExtensionImportRequest>(&json) else {
        return;
    };
    let Some(pdf_path) = request.pdf_path else {
        return;
    };
    let pdf_path = PathBuf::from(pdf_path);
    if pdf_path.exists() && pdf_path.is_file() && is_extension_download_path(&pdf_path) {
        let _ = fs::remove_file(pdf_path);
    }
}

#[tauri::command]
fn get_app_status() -> Result<AppStatus, String> {
    let dir = setting_dir()?;
    let data_file = data_file_path()?;

    Ok(AppStatus {
        setting_dir: dir.to_string_lossy().into_owned(),
        data_file: data_file.to_string_lossy().into_owned(),
        data_file_exists: data_file.exists(),
    })
}

#[tauri::command]
fn save_app_data(data: AppData) -> Result<AppStatus, String> {
    save_data_file(&data)
}

#[tauri::command]
fn load_app_data() -> Result<AppData, String> {
    load_or_default_app_data()
}

#[tauri::command]
fn fetch_paper_metadata(input: FetchMetadataInput) -> Result<PaperMetadata, String> {
    let doi = normalize_optional(input.doi);
    let arxiv_id = normalize_optional(input.arxiv_id);

    if let Some(doi) = doi.as_deref() {
        return fetch_crossref_metadata(doi);
    }

    if let Some(arxiv_id) = arxiv_id.as_deref() {
        return fetch_arxiv_metadata(arxiv_id);
    }

    Err("Enter a DOI or arXiv ID before fetching metadata.".to_string())
}

#[tauri::command]
fn resolve_paper_import(input: ResolvePaperImportInput) -> Result<PaperImportResolution, String> {
    let (doi, arxiv_id) = resolve_import_identifiers(&input.source)?;
    let mut warnings = Vec::new();
    let mut downloaded_pdf_path = None;

    let metadata = if let Some(doi) = doi.as_deref() {
        let metadata = fetch_crossref_metadata(doi)?;
        match download_crossref_pdf(doi) {
            Ok(Some(path)) => {
                downloaded_pdf_path = Some(path.to_string_lossy().into_owned());
            }
            Ok(None) => {
                warnings.push(
                    "No direct open PDF link was found for this DOI. Select a PDF manually if needed."
                        .to_string(),
                );
            }
            Err(error) => warnings.push(error),
        }
        metadata
    } else if let Some(arxiv_id) = arxiv_id.as_deref() {
        let metadata = fetch_arxiv_metadata(arxiv_id)?;
        match download_arxiv_pdf(arxiv_id) {
            Ok(path) => {
                downloaded_pdf_path = Some(path.to_string_lossy().into_owned());
            }
            Err(error) => warnings.push(error),
        }
        metadata
    } else {
        return Err("Enter a DOI or arXiv ID before fetching metadata.".to_string());
    };

    Ok(PaperImportResolution {
        metadata,
        downloaded_pdf_path,
        warnings,
    })
}

#[tauri::command]
fn process_extension_imports(app: tauri::AppHandle) -> Result<ExtensionImportSummary, String> {
    fs::create_dir_all(extension_import_pending_dir()?).map_err(|error| error.to_string())?;
    fs::create_dir_all(extension_import_processed_dir()?).map_err(|error| error.to_string())?;
    fs::create_dir_all(extension_import_failed_dir()?).map_err(|error| error.to_string())?;

    let files = pending_extension_import_files()?;
    let mut imported = 0;
    let mut failed = 0;
    let mut messages = Vec::new();
    let mut data = load_or_default_app_data()?;

    for path in &files {
        match process_extension_import_file_into_data(&mut data, path) {
            Ok(message) => {
                imported += 1;
                messages.push(message);
            }
            Err(error) => {
                failed += 1;
                cleanup_extension_download_from_request_file(&path);
                let failed_path = move_import_file(&path, &extension_import_failed_dir()?)?;
                let error_path = failed_path.with_extension("error.txt");
                fs::write(&error_path, &error).map_err(|write_error| write_error.to_string())?;
                messages.push(format!(
                    "Failed {}: {error}",
                    failed_path
                        .file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("import.json")
                ));
            }
        }
    }

    if imported > 0 {
        save_data_file(&data)?;
    }

    let pending = files.len().saturating_sub(imported + failed);
    if imported > 0 {
        let _ = app.emit("paper-manager:data-updated", ());
    }

    Ok(ExtensionImportSummary {
        imported,
        failed,
        pending,
        messages,
    })
}

#[tauri::command]
fn open_register_paper_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("register-paper") {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }

    let window = tauri::WebviewWindowBuilder::new(
        &app,
        "register-paper",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .inner_size(820.0, 900.0)
    .min_inner_size(720.0, 680.0)
    .build()
    .map_err(|error| error.to_string())?;

    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn open_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }

    let window = tauri::WebviewWindowBuilder::new(
        &app,
        "settings",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .inner_size(820.0, 760.0)
    .min_inner_size(680.0, 560.0)
    .build()
    .map_err(|error| error.to_string())?;

    window
        .set_title("Legra settings")
        .map_err(|error| error.to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn open_edit_paper_window(app: tauri::AppHandle, paper_id: String) -> Result<(), String> {
    let data = load_or_default_app_data()?;
    let paper = data
        .papers
        .iter()
        .find(|paper| paper.id == paper_id)
        .ok_or_else(|| "Paper was not found.".to_string())?;
    let label = format!("edit-paper-{}", paper.id);

    if let Some(window) = app.get_webview_window(&label) {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }

    let window =
        tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::App("index.html".into()))
            .inner_size(820.0, 900.0)
            .min_inner_size(720.0, 680.0)
            .build()
            .map_err(|error| error.to_string())?;

    window
        .set_title(&format!("Edit paper - {}", paper.title))
        .map_err(|error| error.to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn register_paper(app: tauri::AppHandle, input: RegisterPaperInput) -> Result<AppData, String> {
    let data = register_paper_input(input)?;
    let _ = app.emit("paper-manager:data-updated", ());

    Ok(data)
}

#[tauri::command]
fn update_paper(app: tauri::AppHandle, input: UpdatePaperInput) -> Result<AppData, String> {
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err("Title is required.".to_string());
    }

    let authors = input
        .authors
        .iter()
        .map(|author| author.trim().to_string())
        .filter(|author| !author.is_empty())
        .collect::<Vec<_>>();

    if authors.is_empty() {
        return Err("At least one author is required.".to_string());
    }

    let mut data = load_or_default_app_data()?;
    validate_pdf_path(&data, &input.pdf_path)?;
    ensure_not_duplicate(
        &data,
        Some(&input.id),
        &input.title,
        &input.doi,
        &input.pdf_path,
    )?;

    let timestamp = current_timestamp()?;
    let paper = data
        .papers
        .iter_mut()
        .find(|paper| paper.id == input.id)
        .ok_or_else(|| "Paper was not found.".to_string())?;

    paper.title = title;
    paper.authors = authors;
    paper.year = input.year;
    paper.publication = normalize_optional(input.publication);
    paper.volume = normalize_optional(input.volume);
    paper.issue = normalize_optional(input.issue);
    paper.pages = normalize_optional(input.pages);
    paper.numpages = input.numpages;
    paper.month = normalize_optional(input.month);
    paper.publisher = normalize_optional(input.publisher);
    paper.doi = normalize_optional(input.doi);
    paper.arxiv_id = normalize_optional(input.arxiv_id);
    paper.url = normalize_optional(input.url);
    paper.abstract_text = normalize_optional(input.abstract_text);
    paper.tags = input
        .tags
        .iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect();
    paper.status = normalize_optional(input.status);
    paper.pdf_path = normalize_optional(input.pdf_path);
    paper.folder_category = normalize_optional(input.folder_category);
    paper.updated_at = timestamp;

    save_data_file(&data)?;
    let _ = app.emit("paper-manager:data-updated", ());

    Ok(data)
}

#[tauri::command]
fn update_managed_directory(
    app: tauri::AppHandle,
    managed_directory: String,
) -> Result<AppData, String> {
    let path = PathBuf::from(managed_directory.trim());
    if !path.exists() {
        return Err("Managed directory does not exist.".to_string());
    }

    if !path.is_dir() {
        return Err("Managed directory is not a directory.".to_string());
    }

    let mut data = load_or_default_app_data()?;
    let timestamp = current_timestamp()?;
    data.settings.managed_directory = Some(path.to_string_lossy().into_owned());
    data.settings.updated_at = timestamp;
    save_data_file(&data)?;
    let _ = app.emit("paper-manager:data-updated", ());
    Ok(data)
}

fn validate_optional_directory(
    value: &Option<String>,
    label: &str,
) -> Result<Option<String>, String> {
    let Some(value) = normalize_optional(value.clone()) else {
        return Ok(None);
    };
    let path = PathBuf::from(&value);
    if !path.exists() {
        return Err(format!("{label} does not exist."));
    }
    if !path.is_dir() {
        return Err(format!("{label} is not a directory."));
    }
    Ok(Some(path.to_string_lossy().into_owned()))
}

fn validate_optional_app_path(
    value: &Option<String>,
    label: &str,
) -> Result<Option<String>, String> {
    let Some(value) = normalize_optional(value.clone()) else {
        return Ok(None);
    };
    let path = PathBuf::from(&value);
    if path.exists() {
        if path.is_file() || path.is_dir() {
            return Ok(Some(path.to_string_lossy().into_owned()));
        }
        return Err(format!("{label} is not a file or application directory."));
    }

    Ok(Some(value))
}

#[tauri::command]
fn update_settings(app: tauri::AppHandle, input: UpdateSettingsInput) -> Result<AppData, String> {
    let managed_directory =
        validate_optional_directory(&input.managed_directory, "Managed directory")?;
    let note_directory = validate_optional_directory(&input.note_directory, "Note directory")?;
    let chrome_import_directory =
        validate_optional_directory(&input.chrome_import_directory, "Chrome import directory")?;
    let chrome_extension_id = normalize_optional(input.chrome_extension_id);
    if let Some(extension_id) = chrome_extension_id.as_deref() {
        normalize_chrome_extension_id(Some(extension_id.to_string()))?;
    }
    let marktext_path = validate_optional_app_path(&input.marktext_path, "MarkText path")?;
    let pdf_viewer_path = validate_optional_app_path(&input.pdf_viewer_path, "PDF viewer path")?;

    let filename_rule = input.filename_rule.trim();
    if filename_rule.is_empty() {
        return Err("Filename rule is required.".to_string());
    }

    let mut data = load_or_default_app_data()?;
    data.settings.managed_directory = managed_directory;
    data.settings.note_directory = note_directory;
    data.settings.chrome_import_directory = chrome_import_directory;
    data.settings.chrome_extension_id = chrome_extension_id;
    data.settings.marktext_path = marktext_path;
    data.settings.pdf_viewer_path = pdf_viewer_path;
    data.settings.filename_rule = filename_rule.to_string();
    data.settings.bibtex_key_rule = input.bibtex_key_rule.trim().to_string();
    data.settings.bibtex_export_rule = input.bibtex_export_rule.trim().to_string();
    data.settings.journal_output_style = match input.journal_output_style.trim() {
        "full" => "full".to_string(),
        "abbreviation" => "abbreviation".to_string(),
        _ => "as_stored".to_string(),
    };
    data.settings.journal_aliases = input
        .journal_aliases
        .into_iter()
        .map(|alias| JournalAlias {
            full_name: alias.full_name.trim().to_string(),
            abbreviation: alias.abbreviation.trim().to_string(),
            aliases: alias
                .aliases
                .into_iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect(),
        })
        .filter(|alias| !alias.full_name.is_empty() || !alias.abbreviation.is_empty())
        .collect();
    data.settings.cloud_sync_expected = input.cloud_sync_expected;
    data.settings.updated_at = current_timestamp()?;

    save_data_file(&data)?;
    let _ = app.emit("paper-manager:data-updated", ());
    Ok(data)
}

fn chrome_native_host_status_for(extension_id: String) -> Result<ChromeNativeHostStatus, String> {
    let manifest_path = chrome_native_host_manifest_path()?;
    let host_path = chrome_native_host_binary_path()?;
    let installed = manifest_path.exists();
    let message = if installed {
        "Chrome Native Host manifest is installed.".to_string()
    } else {
        "Chrome Native Host manifest is not installed.".to_string()
    };

    Ok(ChromeNativeHostStatus {
        installed,
        manifest_path: manifest_path.to_string_lossy().into_owned(),
        host_path: host_path.to_string_lossy().into_owned(),
        extension_id,
        message,
    })
}

#[tauri::command]
fn check_chrome_native_host(
    input: Option<ChromeNativeHostInput>,
) -> Result<ChromeNativeHostStatus, String> {
    let extension_id = normalize_chrome_extension_id(input.and_then(|input| input.extension_id))?;
    chrome_native_host_status_for(extension_id)
}

#[tauri::command]
fn install_chrome_native_host(
    input: ChromeNativeHostInput,
) -> Result<ChromeNativeHostStatus, String> {
    let extension_id = normalize_chrome_extension_id(input.extension_id)?;
    let manifest_path = chrome_native_host_manifest_path()?;
    let host_path = chrome_native_host_binary_path()?;
    if !host_path.exists() {
        return Err(format!(
            "Chrome Native Host binary was not found at {}. Build or install Legra first.",
            host_path.to_string_lossy()
        ));
    }

    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let manifest = serde_json::json!({
        "name": CHROME_NATIVE_HOST_NAME,
        "description": "Legra Chrome import native host",
        "path": host_path.to_string_lossy(),
        "type": "stdio",
        "allowed_origins": [format!("chrome-extension://{extension_id}/")]
    });
    let json = serde_json::to_string_pretty(&manifest).map_err(|error| error.to_string())?;
    fs::write(&manifest_path, json).map_err(|error| error.to_string())?;

    Ok(ChromeNativeHostStatus {
        installed: true,
        manifest_path: manifest_path.to_string_lossy().into_owned(),
        host_path: host_path.to_string_lossy().into_owned(),
        extension_id,
        message: "Chrome Native Host manifest installed.".to_string(),
    })
}

#[tauri::command]
fn uninstall_chrome_native_host() -> Result<ChromeNativeHostStatus, String> {
    let extension_id = normalize_chrome_extension_id(None)?;
    let manifest_path = chrome_native_host_manifest_path()?;
    if manifest_path.exists() {
        fs::remove_file(&manifest_path).map_err(|error| error.to_string())?;
    }

    let mut status = chrome_native_host_status_for(extension_id)?;
    status.message = "Chrome Native Host manifest removed.".to_string();
    Ok(status)
}

#[tauri::command]
fn organize_paper_pdf(app: tauri::AppHandle, input: OrganizePdfInput) -> Result<AppData, String> {
    let mut data = load_or_default_app_data()?;
    let paper_index = data
        .papers
        .iter()
        .position(|paper| paper.id == input.paper_id)
        .ok_or_else(|| "Paper was not found.".to_string())?;
    let folder_category = normalize_optional(input.folder_category);
    organize_pdf_for_paper(&mut data, paper_index, folder_category)?;

    save_data_file(&data)?;
    let _ = app.emit("paper-manager:data-updated", ());
    Ok(data)
}

#[tauri::command]
fn delete_papers(app: tauri::AppHandle, input: DeletePapersInput) -> Result<AppData, String> {
    let paper_ids = input
        .paper_ids
        .iter()
        .map(|paper_id| paper_id.trim().to_string())
        .filter(|paper_id| !paper_id.is_empty())
        .collect::<Vec<_>>();

    if paper_ids.is_empty() {
        return Err("Select at least one paper to delete.".to_string());
    }

    let mut data = load_or_default_app_data()?;
    let missing = paper_ids
        .iter()
        .filter(|paper_id| !data.papers.iter().any(|paper| &paper.id == *paper_id))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!("Paper was not found: {}", missing.join(", ")));
    }

    data.papers.retain(|paper| !paper_ids.contains(&paper.id));
    data.notes
        .retain(|note| !paper_ids.contains(&note.paper_id));
    save_data_file(&data)?;
    let _ = app.emit("paper-manager:data-updated", ());
    Ok(data)
}

#[tauri::command]
fn generate_bibtex(input: BibtexInput) -> Result<String, String> {
    if input.paper_ids.is_empty() {
        return Err("Select at least one paper.".to_string());
    }

    let data = load_or_default_app_data()?;
    let mut settings = data.settings.clone();
    if let Some(style) = input.journal_output_style.as_deref() {
        settings.journal_output_style = match style.trim() {
            "full" => "full".to_string(),
            "abbreviation" => "abbreviation".to_string(),
            _ => "as_stored".to_string(),
        };
    }
    let settings = &settings;
    let entries = input
        .paper_ids
        .iter()
        .map(|paper_id| {
            data.papers
                .iter()
                .find(|paper| &paper.id == paper_id)
                .map(|paper| paper_to_bibtex(paper, settings))
                .ok_or_else(|| format!("Paper was not found: {paper_id}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(entries.join("\n\n"))
}

#[tauri::command]
fn save_bibtex(input: SaveBibtexInput) -> Result<(), String> {
    let path = PathBuf::from(input.path.trim());
    if path.as_os_str().is_empty() {
        return Err("Output path is required.".to_string());
    }

    fs::write(path, input.content).map_err(|error| error.to_string())
}

#[tauri::command]
fn create_backup(input: BackupInput) -> Result<BackupResult, String> {
    let target_dir = PathBuf::from(input.target_dir.trim());
    if !target_dir.exists() {
        return Err("Backup target directory does not exist.".to_string());
    }

    if !target_dir.is_dir() {
        return Err("Backup target is not a directory.".to_string());
    }

    let timestamp = current_timestamp()?;
    let backup_dir = target_dir.join(format!("paper-manager-backup-{timestamp}"));
    fs::create_dir_all(&backup_dir).map_err(|error| error.to_string())?;

    let setting_backup_dir = backup_dir.join("setting");
    copy_dir_recursive(&setting_dir()?, &setting_backup_dir)?;

    let data = load_or_default_app_data()?;
    if let Some(managed_directory) = data.settings.managed_directory.as_deref() {
        let managed_directory = Path::new(managed_directory);
        if managed_directory.exists() {
            copy_dir_recursive(managed_directory, &backup_dir.join("managed-directory"))?;
        }
    }

    fs::write(
        backup_dir.join("README.md"),
        "# Legra backup\n\nSelect this directory with Restore backup.\n",
    )
    .map_err(|error| error.to_string())?;

    Ok(BackupResult {
        backup_dir: backup_dir.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
fn restore_backup(input: RestoreInput) -> Result<AppData, String> {
    let backup_dir = PathBuf::from(input.backup_dir.trim());
    if !backup_dir.exists() || !backup_dir.is_dir() {
        return Err("Backup directory does not exist.".to_string());
    }

    let backup_data_file = backup_dir.join("setting").join("app-data.json");
    if !backup_data_file.exists() {
        return Err("Backup does not contain setting/app-data.json.".to_string());
    }

    let json = fs::read_to_string(&backup_data_file).map_err(|error| error.to_string())?;
    let mut data: AppData = serde_json::from_str(&json).map_err(|error| error.to_string())?;

    let managed_backup_dir = backup_dir.join("managed-directory");
    if managed_backup_dir.exists() {
        data.settings.managed_directory = Some(managed_backup_dir.to_string_lossy().into_owned());
        relink_paths_to_root(&mut data, &managed_backup_dir)?;
    }

    let setting_backup_dir = backup_dir.join("setting");
    copy_dir_recursive(&setting_backup_dir, &setting_dir()?)?;
    save_data_file(&data)?;
    Ok(data)
}

#[tauri::command]
fn relink_files(input: RelinkInput) -> Result<AppData, String> {
    let root_dir = PathBuf::from(input.root_dir.trim());
    if !root_dir.exists() || !root_dir.is_dir() {
        return Err("Relink directory does not exist.".to_string());
    }

    let mut data = load_or_default_app_data()?;
    relink_paths_to_root(&mut data, &root_dir)?;
    data.settings.managed_directory = Some(root_dir.to_string_lossy().into_owned());
    data.settings.updated_at = current_timestamp()?;
    save_data_file(&data)?;
    Ok(data)
}

#[tauri::command]
fn create_shared_workspace(
    app: tauri::AppHandle,
    input: WorkspaceInput,
) -> Result<AppData, String> {
    let root = PathBuf::from(input.workspace_dir.trim());
    if !root.exists() || !root.is_dir() {
        return Err("Workspace directory does not exist.".to_string());
    }

    ensure_workspace_dirs(&root)?;
    let workspace_file = workspace_data_file(&root);
    if workspace_file.exists() {
        return Err("Workspace already contains paper-manager-workspace.json.".to_string());
    }

    let timestamp = current_timestamp()?;
    let mut data = empty_app_data()?;
    data.settings.workspace_root = Some(root.to_string_lossy().into_owned());
    data.settings.workspace_revision = Some(0);
    data.settings.workspace_last_loaded_revision = Some(0);
    data.settings.managed_directory =
        Some(workspace_papers_dir(&root).to_string_lossy().into_owned());
    data.settings.note_directory = Some(workspace_notes_dir(&root).to_string_lossy().into_owned());
    data.settings.updated_at = timestamp;
    save_data_file(&data)?;
    let _ = app.emit("paper-manager:data-updated", ());
    Ok(load_or_default_app_data()?)
}

#[tauri::command]
fn open_shared_workspace(app: tauri::AppHandle, input: WorkspaceInput) -> Result<AppData, String> {
    let root = PathBuf::from(input.workspace_dir.trim());
    if !root.exists() || !root.is_dir() {
        return Err("Workspace directory does not exist.".to_string());
    }

    let workspace_file = workspace_data_file(&root);
    if !workspace_file.exists() {
        return Err("Workspace does not contain paper-manager-workspace.json.".to_string());
    }

    let json = fs::read_to_string(&workspace_file).map_err(|error| error.to_string())?;
    let mut data: AppData = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    let revision = data.settings.workspace_revision.unwrap_or(0);
    data.settings.workspace_root = Some(root.to_string_lossy().into_owned());
    data.settings.workspace_last_loaded_revision = Some(revision);
    data.settings.managed_directory =
        Some(workspace_papers_dir(&root).to_string_lossy().into_owned());
    data.settings.note_directory = Some(workspace_notes_dir(&root).to_string_lossy().into_owned());

    let local_json = serde_json::to_string_pretty(&data).map_err(|error| error.to_string())?;
    fs::create_dir_all(setting_dir()?).map_err(|error| error.to_string())?;
    fs::write(data_file_path()?, local_json).map_err(|error| error.to_string())?;
    let _ = app.emit("paper-manager:data-updated", ());
    Ok(load_or_default_app_data()?)
}

#[tauri::command]
fn convert_current_library_to_workspace(
    app: tauri::AppHandle,
    input: WorkspaceInput,
) -> Result<AppData, String> {
    let root = PathBuf::from(input.workspace_dir.trim());
    if !root.exists() || !root.is_dir() {
        return Err("Workspace directory does not exist.".to_string());
    }

    ensure_workspace_dirs(&root)?;
    let workspace_file = workspace_data_file(&root);
    if workspace_file.exists() {
        return Err("Workspace already contains paper-manager-workspace.json.".to_string());
    }

    let mut data = load_or_default_app_data()?;
    if let Some(managed_directory) = data.settings.managed_directory.as_deref() {
        let managed_directory = Path::new(managed_directory);
        if managed_directory.exists() {
            copy_dir_recursive(managed_directory, &workspace_papers_dir(&root))?;
        }
    }
    if let Some(note_directory) = data.settings.note_directory.as_deref() {
        let note_directory = Path::new(note_directory);
        if note_directory.exists() {
            copy_dir_recursive(note_directory, &workspace_notes_dir(&root))?;
        }
    }

    data.settings.workspace_root = Some(root.to_string_lossy().into_owned());
    data.settings.workspace_revision = Some(0);
    data.settings.workspace_last_loaded_revision = Some(0);
    data.settings.managed_directory =
        Some(workspace_papers_dir(&root).to_string_lossy().into_owned());
    data.settings.note_directory = Some(workspace_notes_dir(&root).to_string_lossy().into_owned());
    data.settings.updated_at = current_timestamp()?;
    relink_paths_to_root(&mut data, &root)?;
    relativize_paths_to_workspace(&mut data);

    save_data_file(&data)?;
    let _ = app.emit("paper-manager:data-updated", ());
    Ok(load_or_default_app_data()?)
}

#[tauri::command]
fn check_workspace_health() -> Result<WorkspaceHealth, String> {
    let data = load_or_default_app_data()?;
    let mut warnings = Vec::new();
    let Some(root) = active_workspace_root(&data) else {
        return Ok(WorkspaceHealth {
            ok: false,
            warnings: vec!["No shared workspace is active.".to_string()],
        });
    };

    if !workspace_data_file(&root).exists() {
        warnings.push("paper-manager-workspace.json is missing.".to_string());
    }
    if workspace_lock_file(&root).exists() {
        warnings
            .push("Workspace write.lock exists. Another app instance may be saving.".to_string());
    }

    for paper in &data.papers {
        if let Some(pdf_path) = paper.pdf_path.as_deref() {
            if !path_for_runtime(&data, pdf_path).exists() {
                warnings.push(format!("Missing PDF: {}", paper.title));
            }
        }
    }
    for note in &data.notes {
        if !path_for_runtime(&data, &note.file_path).exists() {
            warnings.push(format!("Missing note: {}", note.title));
        }
    }

    Ok(WorkspaceHealth {
        ok: warnings.is_empty(),
        warnings,
    })
}

#[tauri::command]
fn create_note(input: CreateNoteInput) -> Result<AppData, String> {
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err("Note title is required.".to_string());
    }

    let mut data = load_or_default_app_data()?;
    ensure_paper_exists(&data, &input.paper_id)?;

    let timestamp = current_timestamp()?;
    let note_id = now_note_id()?;
    let dir = notes_dir()?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;

    let file_name = format!(
        "{}_{}.md",
        sanitize_filename(&input.paper_id),
        sanitize_filename(&title)
    );
    let file_path = dir.join(file_name);
    if file_path.exists() {
        return Err("A note file with the same name already exists.".to_string());
    }

    let initial_content = format!("# {title}\n\n");
    fs::write(&file_path, initial_content).map_err(|error| error.to_string())?;

    data.notes.push(Note {
        id: note_id,
        paper_id: input.paper_id,
        title,
        file_path: path_for_storage(&data, &file_path),
        file_type: Some("md".to_string()),
        summary: None,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    });

    save_data_file(&data)?;
    Ok(data)
}

#[tauri::command]
fn link_note(input: LinkNoteInput) -> Result<AppData, String> {
    let mut data = load_or_default_app_data()?;
    ensure_paper_exists(&data, &input.paper_id)?;

    let path = PathBuf::from(input.file_path.trim());
    if !path.exists() {
        return Err("Note file does not exist.".to_string());
    }

    if !path.is_file() {
        return Err("Selected note path is not a file.".to_string());
    }

    let normalized_path = path_for_storage(&data, &path);
    if data
        .notes
        .iter()
        .any(|note| normalize_key(&note.file_path) == normalize_key(&normalized_path))
    {
        return Err("This note file is already linked.".to_string());
    }

    let title =
        normalize_optional(Some(input.title)).unwrap_or_else(|| note_title_from_path(&path));
    let timestamp = current_timestamp()?;
    data.notes.push(Note {
        id: now_note_id()?,
        paper_id: input.paper_id,
        title,
        file_path: normalized_path,
        file_type: note_file_type(&path),
        summary: None,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    });

    save_data_file(&data)?;
    Ok(data)
}

fn open_path_with_application(
    target_path: &Path,
    configured_app: Option<&str>,
    fallback_app: &str,
) -> Result<(), String> {
    let app = configured_app
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback_app);
    let status = Command::new("open")
        .args(["-a", app])
        .arg(target_path)
        .status()
        .map_err(|error| error.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Could not open file with {app}."))
    }
}

#[tauri::command]
fn open_note(note_id: String) -> Result<(), String> {
    let data = load_or_default_app_data()?;
    let note = data
        .notes
        .iter()
        .find(|note| note.id == note_id)
        .ok_or_else(|| "Note was not found.".to_string())?;

    let note_path_buf = path_for_runtime(&data, &note.file_path);
    let note_path = note_path_buf.as_path();
    if !note_path.exists() {
        return Err("Note file does not exist.".to_string());
    }

    if is_markdown_path(note_path) {
        return open_path_with_application(
            note_path,
            data.settings.marktext_path.as_deref(),
            "MarkText",
        );
    }

    tauri_plugin_opener::open_path(note_path.to_string_lossy().as_ref(), None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn open_paper_pdf(paper_id: String) -> Result<(), String> {
    let data = load_or_default_app_data()?;
    let paper = data
        .papers
        .iter()
        .find(|paper| paper.id == paper_id)
        .ok_or_else(|| "Paper was not found.".to_string())?;
    let pdf_path = paper
        .pdf_path
        .as_deref()
        .ok_or_else(|| "This paper does not have a PDF path.".to_string())?;
    let pdf_path_buf = path_for_runtime(&data, pdf_path);
    let pdf_path = pdf_path_buf.as_path();

    if !pdf_path.exists() {
        return Err("PDF file does not exist.".to_string());
    }

    if !pdf_path.is_file() {
        return Err("PDF path is not a file.".to_string());
    }

    open_path_with_application(
        pdf_path,
        data.settings.pdf_viewer_path.as_deref(),
        "Preview",
    )
}

#[tauri::command]
fn check_note_files() -> Result<Vec<NoteStatus>, String> {
    let data = load_or_default_app_data()?;
    Ok(data
        .notes
        .iter()
        .map(|note| NoteStatus {
            note_id: note.id.clone(),
            exists: Path::new(&note.file_path).exists(),
        })
        .collect())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .menu(|app| {
            let app_menu = Submenu::with_items(
                app,
                "Legra",
                true,
                &[
                    &PredefinedMenuItem::about(app, None, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &MenuItem::with_id(app, SETTINGS_MENU_ID, "Settings...", true, None::<&str>)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::services(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::hide(app, None)?,
                    &PredefinedMenuItem::hide_others(app, None)?,
                    &PredefinedMenuItem::show_all(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::quit(app, None)?,
                ],
            )?;
            let edit_menu = Submenu::with_items(
                app,
                "Edit",
                true,
                &[
                    &PredefinedMenuItem::undo(app, None)?,
                    &PredefinedMenuItem::redo(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::cut(app, None)?,
                    &PredefinedMenuItem::copy(app, None)?,
                    &PredefinedMenuItem::paste(app, None)?,
                    &PredefinedMenuItem::select_all(app, None)?,
                ],
            )?;
            let window_menu = Submenu::with_items(
                app,
                "Window",
                true,
                &[
                    &PredefinedMenuItem::minimize(app, None)?,
                    &PredefinedMenuItem::maximize(app, None)?,
                    &PredefinedMenuItem::fullscreen(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::close_window(app, None)?,
                ],
            )?;

            Menu::with_items(app, &[&app_menu, &edit_menu, &window_menu])
        })
        .on_menu_event(|app, event| {
            if event.id().as_ref() == SETTINGS_MENU_ID {
                let _ = open_settings_window(app.clone());
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            save_app_data,
            load_app_data,
            fetch_paper_metadata,
            resolve_paper_import,
            process_extension_imports,
            open_register_paper_window,
            open_settings_window,
            open_edit_paper_window,
            register_paper,
            update_paper,
            update_managed_directory,
            update_settings,
            check_chrome_native_host,
            install_chrome_native_host,
            uninstall_chrome_native_host,
            organize_paper_pdf,
            delete_papers,
            generate_bibtex,
            save_bibtex,
            create_backup,
            restore_backup,
            relink_files,
            create_shared_workspace,
            open_shared_workspace,
            convert_current_library_to_workspace,
            check_workspace_health,
            create_note,
            link_note,
            open_note,
            open_paper_pdf,
            check_note_files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
