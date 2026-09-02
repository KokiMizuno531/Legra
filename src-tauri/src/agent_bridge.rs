use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    env, fs,
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        OnceLock,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const AGENT_IMPORT_SCHEMA_VERSION: u32 = 1;
const MANAGED_LIBRARY_SCHEMA_VERSION: u32 = 1;
const MAX_SEARCH_RESULTS: usize = 100;
const MAX_CATEGORY_COUNT: usize = 10_000;
const MAX_DIRECTORY_DEPTH: usize = 32;

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentifierKind {
    Doi,
    Arxiv,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentPaperIdentifier {
    pub kind: IdentifierKind,
    pub value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadPolicy {
    #[default]
    OpenAccessOnly,
    MetadataOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentImportRequest {
    pub schema_version: u32,
    pub job_id: String,
    pub idempotency_key: String,
    pub origin: String,
    pub identifier: AgentPaperIdentifier,
    pub category: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub download_policy: DownloadPolicy,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentJobState {
    Pending,
    Processing,
    Done,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentImportResult {
    pub schema_version: u32,
    pub job_id: String,
    pub status: AgentJobState,
    pub success: bool,
    #[serde(default)]
    pub paper_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub pdf_path: Option<String>,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub warnings: Vec<String>,
    pub completed_at: String,
    #[serde(default)]
    pub details: Option<Value>,
}

impl AgentImportResult {
    pub fn completed(job_id: impl Into<String>) -> Self {
        Self {
            schema_version: AGENT_IMPORT_SCHEMA_VERSION,
            job_id: job_id.into(),
            status: AgentJobState::Done,
            success: true,
            paper_id: None,
            title: None,
            pdf_path: None,
            message: String::new(),
            warnings: Vec::new(),
            completed_at: current_timestamp(),
            details: None,
        }
    }

    pub fn failed(job_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            schema_version: AGENT_IMPORT_SCHEMA_VERSION,
            job_id: job_id.into(),
            status: AgentJobState::Failed,
            success: false,
            paper_id: None,
            title: None,
            pdf_path: None,
            message: message.into(),
            warnings: Vec::new(),
            completed_at: current_timestamp(),
            details: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentImportStatus {
    pub job_id: String,
    pub state: AgentJobState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<AgentImportRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<AgentImportResult>,
}

#[derive(Debug, Clone)]
pub struct AgentQueueDirs {
    pub root: PathBuf,
    pub pending: PathBuf,
    pub processing: PathBuf,
    pub done: PathBuf,
    pub failed: PathBuf,
    pub idempotency: PathBuf,
}

impl AgentQueueDirs {
    pub fn ensure(&self) -> Result<(), String> {
        for directory in [
            &self.pending,
            &self.processing,
            &self.done,
            &self.failed,
            &self.idempotency,
        ] {
            fs::create_dir_all(directory)
                .map_err(|error| format!("Could not create {}: {error}", directory.display()))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct EnqueueImportInput {
    pub identifier: String,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub download_policy: DownloadPolicy,
    pub idempotency_key: Option<String>,
    pub origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnqueueImportResult {
    pub request: AgentImportRequest,
    pub state: AgentJobState,
    pub already_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryPaperSummary {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub year: Option<u16>,
    pub publication: Option<String>,
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub tags: Vec<String>,
    pub folder_category: Option<String>,
    pub has_pdf: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvePaperResult {
    pub identifier: AgentPaperIdentifier,
    pub found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper: Option<LibraryPaperSummary>,
    pub library_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchLibraryResult {
    pub query: String,
    pub count: usize,
    pub papers: Vec<LibraryPaperSummary>,
    pub library_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryListing {
    pub categories: Vec<String>,
    pub count: usize,
    pub library_source: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct StoredPaper {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    authors: Vec<String>,
    #[serde(default)]
    year: Option<u16>,
    #[serde(default)]
    publication: Option<String>,
    #[serde(default)]
    doi: Option<String>,
    #[serde(default)]
    arxiv_id: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    folder_category: Option<String>,
    #[serde(default)]
    pdf_path: Option<String>,
    #[serde(default)]
    original_pdf_path: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct StoredSettings {
    #[serde(default)]
    managed_directory: Option<String>,
    #[serde(default)]
    workspace_root: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct StoredAppData {
    #[serde(default)]
    papers: Vec<StoredPaper>,
    #[serde(default)]
    settings: StoredSettings,
}

#[derive(Debug, Deserialize)]
struct StoredManagedLibrary {
    schema_version: u32,
    data: StoredAppData,
}

struct LibrarySnapshot {
    papers: Vec<StoredPaper>,
    category_root: Option<PathBuf>,
    source: String,
}

fn nonempty_env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn home_dir() -> Result<PathBuf, String> {
    nonempty_env_path("HOME")
        .or_else(|| nonempty_env_path("USERPROFILE"))
        .ok_or_else(|| "Could not resolve the user home directory.".to_string())
}

pub fn legra_setting_dir() -> Result<PathBuf, String> {
    if let Some(path) = nonempty_env_path("LEGRA_SETTING_DIR") {
        return Ok(path);
    }

    #[cfg(target_os = "macos")]
    {
        return Ok(home_dir()?
            .join("Library")
            .join("Application Support")
            .join("Legra"));
    }

    #[cfg(target_os = "windows")]
    {
        let roaming =
            nonempty_env_path("APPDATA").unwrap_or(home_dir()?.join("AppData").join("Roaming"));
        return Ok(roaming.join("Legra"));
    }

    #[cfg(target_os = "linux")]
    {
        let data_home =
            nonempty_env_path("XDG_DATA_HOME").unwrap_or(home_dir()?.join(".local").join("share"));
        return Ok(data_home.join("Legra"));
    }

    #[allow(unreachable_code)]
    Ok(home_dir()?.join(".legra"))
}

pub fn agent_import_dirs() -> Result<AgentQueueDirs, String> {
    let root = legra_setting_dir()?.join("agent-imports");
    Ok(AgentQueueDirs {
        pending: root.join("pending"),
        processing: root.join("processing"),
        done: root.join("done"),
        failed: root.join("failed"),
        idempotency: root.join(".idempotency"),
        root,
    })
}

fn current_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
}

fn sha256_hex(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn safe_job_id(value: &str) -> Result<&str, String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err("Invalid agent import job ID.".to_string());
    }
    Ok(value)
}

fn job_file(directory: &Path, job_id: &str) -> Result<PathBuf, String> {
    Ok(directory.join(format!("{}.json", safe_job_id(job_id)?)))
}

fn result_file(directory: &Path, job_id: &str) -> Result<PathBuf, String> {
    Ok(directory.join(format!("{}.result.json", safe_job_id(job_id)?)))
}

fn temporary_path(path: &Path) -> Result<PathBuf, String> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Could not construct a temporary queue path.".to_string())?;
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(path.with_file_name(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        counter
    )))
}

fn write_synced_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("Could not create {}: {error}", path.display()))?;
    file.write_all(bytes)
        .and_then(|_| file.flush())
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("Could not persist {}: {error}", path.display()))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Queue file does not have a parent directory.".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = temporary_path(path)?;
    if let Err(error) = write_synced_file(&temporary, bytes) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "Could not atomically publish {}: {error}",
            path.display()
        ));
    }
    Ok(())
}

// Hard-link publication gives create-if-absent semantics on all supported desktop
// platforms. It reserves idempotency and pending records without overwriting a
// concurrently published job.
fn publish_no_replace(path: &Path, bytes: &[u8]) -> Result<bool, String> {
    let temporary = temporary_path(path)?;
    if let Err(error) = write_synced_file(&temporary, bytes) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    let published = match fs::hard_link(&temporary, path) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            return Err(format!(
                "Could not publish idempotency record {}: {error}",
                path.display()
            ));
        }
    };
    let _ = fs::remove_file(&temporary);
    Ok(published)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let file =
        File::open(path).map_err(|error| format!("Could not open {}: {error}", path.display()))?;
    serde_json::from_reader(file)
        .map_err(|error| format!("Could not parse {}: {error}", path.display()))
}

fn read_idempotency_record(path: &Path) -> Result<AgentImportRequest, String> {
    // Another MCP process may have just hard-linked the record. The linked file is
    // already flushed, but short retries also make this tolerant of unusual filesystems.
    let mut last_error = None;
    for _ in 0..10 {
        match read_json(path) {
            Ok(request) => return Ok(request),
            Err(error) => {
                last_error = Some(error);
                thread::sleep(Duration::from_millis(5));
            }
        }
    }
    Err(last_error.unwrap_or_else(|| "Could not read idempotency record.".to_string()))
}

fn doi_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)^10\.\d{4,9}/[a-z0-9][a-z0-9._;()/:+\-]*$").expect("valid DOI regex")
    })
}

fn modern_arxiv_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^\d{4}\.\d{4,5}$").expect("valid arXiv regex"))
}

fn legacy_arxiv_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)^[a-z][a-z0-9.\-]+/\d{7}$").expect("valid legacy arXiv regex")
    })
}

fn strip_arxiv_version(value: &str) -> &str {
    if let Some((base, version)) = value.rsplit_once('v') {
        if !base.is_empty() && !version.is_empty() && version.chars().all(|c| c.is_ascii_digit()) {
            return base;
        }
    }
    value
}

pub fn normalize_identifier(input: &str) -> Result<AgentPaperIdentifier, String> {
    let input = input.trim();
    if input.is_empty() || input.len() > 512 || input.chars().any(char::is_control) {
        return Err("Identifier must be a non-empty DOI or arXiv identifier.".to_string());
    }

    let lower = input.to_ascii_lowercase();
    let doi_candidate = if lower.starts_with("doi:") {
        Some(input[4..].trim())
    } else {
        [
            "https://doi.org/",
            "http://doi.org/",
            "https://dx.doi.org/",
            "http://dx.doi.org/",
            "https://www.doi.org/",
            "http://www.doi.org/",
        ]
        .iter()
        .find_map(|prefix| lower.starts_with(prefix).then(|| &input[prefix.len()..]))
        .or_else(|| lower.starts_with("10.").then_some(input))
    };

    if let Some(candidate) = doi_candidate {
        if candidate.contains(['?', '#']) {
            return Err("DOI URLs must not contain a query or fragment.".to_string());
        }
        let decoded = urlencoding::decode(candidate)
            .map_err(|_| "DOI contains invalid percent encoding.".to_string())?;
        let value = decoded.trim().trim_end_matches('.').to_ascii_lowercase();
        if !doi_regex().is_match(&value) {
            return Err("The DOI is not in a supported canonical form.".to_string());
        }
        if let Some(arxiv_id) = value.strip_prefix("10.48550/arxiv.") {
            let arxiv_id = strip_arxiv_version(arxiv_id);
            if modern_arxiv_regex().is_match(arxiv_id) || legacy_arxiv_regex().is_match(arxiv_id) {
                return Ok(AgentPaperIdentifier {
                    kind: IdentifierKind::Arxiv,
                    value: arxiv_id.to_string(),
                });
            }
        }
        return Ok(AgentPaperIdentifier {
            kind: IdentifierKind::Doi,
            value,
        });
    }

    let arxiv_candidate = if lower.starts_with("arxiv:") {
        Some(input[6..].trim())
    } else {
        [
            "https://arxiv.org/abs/",
            "http://arxiv.org/abs/",
            "https://www.arxiv.org/abs/",
            "http://www.arxiv.org/abs/",
            "https://arxiv.org/pdf/",
            "http://arxiv.org/pdf/",
            "https://www.arxiv.org/pdf/",
            "http://www.arxiv.org/pdf/",
        ]
        .iter()
        .find_map(|prefix| lower.starts_with(prefix).then(|| &input[prefix.len()..]))
        .or_else(|| {
            (modern_arxiv_regex().is_match(strip_arxiv_version(input))
                || legacy_arxiv_regex().is_match(strip_arxiv_version(input)))
            .then_some(input)
        })
    };

    if let Some(candidate) = arxiv_candidate {
        if candidate.contains(['?', '#']) {
            return Err("arXiv URLs must not contain a query or fragment.".to_string());
        }
        let candidate = candidate.trim_end_matches(".pdf");
        let value = strip_arxiv_version(candidate).to_ascii_lowercase();
        if !modern_arxiv_regex().is_match(&value) && !legacy_arxiv_regex().is_match(&value) {
            return Err("The arXiv identifier is not in a supported canonical form.".to_string());
        }
        return Ok(AgentPaperIdentifier {
            kind: IdentifierKind::Arxiv,
            value,
        });
    }

    Err("Only DOI/doi.org and arXiv identifiers or URLs are accepted.".to_string())
}

fn normalize_origin(origin: &str) -> String {
    let normalized = origin
        .trim()
        .chars()
        .filter(|character| !character.is_control())
        .take(120)
        .collect::<String>();
    if normalized.is_empty() {
        "unknown-mcp-client".to_string()
    } else {
        normalized
    }
}

fn normalize_category(category: Option<String>) -> Result<Option<String>, String> {
    let Some(category) = category else {
        return Ok(None);
    };
    let category = category.trim();
    if category.is_empty() {
        return Ok(None);
    }
    if category.len() > 300 || category.contains(['\\', '\0']) || Path::new(category).is_absolute()
    {
        return Err("Category must be a relative Legra category path.".to_string());
    }
    let mut parts = Vec::new();
    for part in category.split('/') {
        let part = part.trim();
        if part.is_empty() || matches!(part, "." | "..") || part.len() > 100 {
            return Err("Category contains an invalid path component.".to_string());
        }
        parts.push(part);
    }
    Ok(Some(parts.join("/")))
}

fn normalize_tags(tags: Vec<String>) -> Result<Vec<String>, String> {
    if tags.len() > 64 {
        return Err("At most 64 tags may be supplied.".to_string());
    }
    let mut normalized = Vec::new();
    for tag in tags {
        let tag = tag.split_whitespace().collect::<Vec<_>>().join(" ");
        if tag.is_empty() {
            continue;
        }
        if tag.len() > 100 || tag.chars().any(char::is_control) {
            return Err("Tags must be at most 100 characters and contain no controls.".to_string());
        }
        if !normalized.iter().any(|existing| existing == &tag) {
            normalized.push(tag);
        }
    }
    Ok(normalized)
}

fn import_state_for_job(dirs: &AgentQueueDirs, job_id: &str) -> Result<AgentJobState, String> {
    for (directory, state) in [
        (&dirs.pending, AgentJobState::Pending),
        (&dirs.processing, AgentJobState::Processing),
        (&dirs.done, AgentJobState::Done),
        (&dirs.failed, AgentJobState::Failed),
    ] {
        if job_file(directory, job_id)?.is_file() {
            return Ok(state);
        }
    }
    Ok(AgentJobState::Unknown)
}

fn request_matches_enqueue(
    request: &AgentImportRequest,
    identifier: &AgentPaperIdentifier,
    category: &Option<String>,
    tags: &[String],
    download_policy: &DownloadPolicy,
    idempotency_key: &str,
    origin: &str,
) -> bool {
    request.schema_version == AGENT_IMPORT_SCHEMA_VERSION
        && request.identifier == *identifier
        && request.category == *category
        && request.tags == tags
        && request.download_policy == *download_policy
        && request.idempotency_key == idempotency_key
        && request.origin == origin
}

fn recover_missing_idempotent_job(
    dirs: &AgentQueueDirs,
    request: &AgentImportRequest,
) -> Result<AgentJobState, String> {
    let state = import_state_for_job(dirs, &request.job_id)?;
    if state != AgentJobState::Unknown {
        return Ok(state);
    }
    let bytes = serde_json::to_vec_pretty(request).map_err(|error| error.to_string())?;
    let _ = publish_no_replace(&job_file(&dirs.pending, &request.job_id)?, &bytes)?;
    import_state_for_job(dirs, &request.job_id)
}

pub fn enqueue_import(input: EnqueueImportInput) -> Result<EnqueueImportResult, String> {
    let identifier = normalize_identifier(&input.identifier)?;
    let category = normalize_category(input.category)?;
    let tags = normalize_tags(input.tags)?;
    let origin = normalize_origin(&input.origin);

    let canonical = serde_json::json!({
        "origin": origin,
        "identifier": identifier,
        "category": category,
        "tags": tags,
        "download_policy": input.download_policy,
    });
    let idempotency_key = match input.idempotency_key {
        Some(value) => {
            let value = value.trim().to_string();
            if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
                return Err(
                    "idempotency_key must be 1-256 characters with no controls.".to_string()
                );
            }
            value
        }
        None => format!(
            "auto:{}",
            sha256_hex(
                serde_json::to_string(&canonical)
                    .unwrap_or_default()
                    .as_bytes()
            )
        ),
    };
    let job_digest =
        sha256_hex(format!("legra-agent-import-v1\0{origin}\0{idempotency_key}").as_bytes());
    let job_id = format!("agent-{}", &job_digest[..32]);
    safe_job_id(&job_id)?;

    let dirs = agent_import_dirs()?;
    dirs.ensure()?;
    let record_path = dirs.idempotency.join(format!("{job_id}.json"));
    if record_path.exists() {
        let request = read_idempotency_record(&record_path)?;
        if !request_matches_enqueue(
            &request,
            &identifier,
            &category,
            &tags,
            &input.download_policy,
            &idempotency_key,
            &origin,
        ) {
            return Err(
                "The idempotency_key was already used with different import arguments.".to_string(),
            );
        }
        return Ok(EnqueueImportResult {
            state: recover_missing_idempotent_job(&dirs, &request)?,
            request,
            already_exists: true,
        });
    }

    let request = AgentImportRequest {
        schema_version: AGENT_IMPORT_SCHEMA_VERSION,
        job_id: job_id.clone(),
        idempotency_key,
        origin,
        identifier,
        category,
        tags,
        download_policy: input.download_policy,
        created_at: current_timestamp(),
    };
    let bytes = serde_json::to_vec_pretty(&request).map_err(|error| error.to_string())?;
    let won_publication = publish_no_replace(&record_path, &bytes)?;
    if !won_publication {
        let existing = read_idempotency_record(&record_path)?;
        if !request_matches_enqueue(
            &existing,
            &request.identifier,
            &request.category,
            &request.tags,
            &request.download_policy,
            &request.idempotency_key,
            &request.origin,
        ) {
            return Err(
                "The idempotency_key was concurrently used with different import arguments."
                    .to_string(),
            );
        }
        return Ok(EnqueueImportResult {
            state: recover_missing_idempotent_job(&dirs, &existing)?,
            request: existing,
            already_exists: true,
        });
    }

    let _ = publish_no_replace(&job_file(&dirs.pending, &job_id)?, &bytes)?;
    Ok(EnqueueImportResult {
        request,
        state: AgentJobState::Pending,
        already_exists: false,
    })
}

pub fn list_pending_imports() -> Result<Vec<AgentImportRequest>, String> {
    let dirs = agent_import_dirs()?;
    dirs.ensure()?;
    let mut requests = Vec::new();
    for entry in fs::read_dir(&dirs.pending).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json")
            || path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.ends_with(".result.json"))
        {
            continue;
        }
        requests.push(read_json(&path)?);
    }
    requests.sort_by(|left: &AgentImportRequest, right: &AgentImportRequest| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.job_id.cmp(&right.job_id))
    });
    Ok(requests)
}

pub fn list_pending_import_ids() -> Result<Vec<String>, String> {
    let dirs = agent_import_dirs()?;
    dirs.ensure()?;
    let mut jobs = Vec::new();
    for entry in fs::read_dir(&dirs.pending).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(job_id) = file_name.strip_suffix(".json") else {
            continue;
        };
        if job_id.ends_with(".result") || safe_job_id(job_id).is_err() || !path.is_file() {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(UNIX_EPOCH);
        jobs.push((modified, job_id.to_string()));
    }
    jobs.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    Ok(jobs.into_iter().map(|(_, job_id)| job_id).collect())
}

fn touch_claimed_import(path: &Path) -> Result<(), String> {
    let file = OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|error| format!("Could not open claimed import {}: {error}", path.display()))?;
    file.set_times(fs::FileTimes::new().set_modified(SystemTime::now()))
        .map_err(|error| {
            format!(
                "Could not timestamp claimed import {}: {error}",
                path.display()
            )
        })
}

pub fn claim_pending_import(job_id: &str) -> Result<Option<PathBuf>, String> {
    safe_job_id(job_id)?;
    let dirs = agent_import_dirs()?;
    dirs.ensure()?;
    let pending = job_file(&dirs.pending, job_id)?;
    let processing = job_file(&dirs.processing, job_id)?;
    if !pending.exists() {
        return Ok(None);
    }
    if processing.exists() {
        return Ok(None);
    }
    match fs::rename(&pending, &processing) {
        Ok(()) => {
            if let Err(error) = touch_claimed_import(&processing) {
                let _ = fs::rename(&processing, &pending);
                return Err(error);
            }
            Ok(Some(processing))
        }
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::AlreadyExists
            ) =>
        {
            Ok(None)
        }
        Err(error) => Err(format!("Could not claim agent import {job_id}: {error}")),
    }
}

pub fn recover_stale_processing_imports(max_age: Duration) -> Result<usize, String> {
    let dirs = agent_import_dirs()?;
    dirs.ensure()?;
    let mut recovered = 0;
    for entry in fs::read_dir(&dirs.processing).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let processing = entry.path();
        let Some(file_name) = processing.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(job_id) = file_name.strip_suffix(".json") else {
            continue;
        };
        if job_id.ends_with(".result") || safe_job_id(job_id).is_err() || !processing.is_file() {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(UNIX_EPOCH);
        if SystemTime::now()
            .duration_since(modified)
            .unwrap_or_default()
            < max_age
        {
            continue;
        }

        let terminal_directory = if result_file(&dirs.done, job_id)?.is_file() {
            Some(&dirs.done)
        } else if result_file(&dirs.failed, job_id)?.is_file() {
            Some(&dirs.failed)
        } else {
            None
        };
        if let Some(directory) = terminal_directory {
            let finished = job_file(directory, job_id)?;
            if finished.is_file() {
                fs::remove_file(&processing).map_err(|error| {
                    format!("Could not remove duplicate processing job {job_id}: {error}")
                })?;
            } else {
                fs::rename(&processing, &finished).map_err(|error| {
                    format!("Could not recover completed agent import {job_id}: {error}")
                })?;
            }
            recovered += 1;
            continue;
        }

        let pending = job_file(&dirs.pending, job_id)?;
        if pending.is_file() {
            fs::remove_file(&processing).map_err(|error| {
                format!("Could not remove duplicate processing job {job_id}: {error}")
            })?;
        } else {
            fs::rename(&processing, &pending).map_err(|error| {
                format!("Could not requeue stale agent import {job_id}: {error}")
            })?;
        }
        recovered += 1;
    }
    Ok(recovered)
}

pub fn read_claimed_import(job_id: &str) -> Result<AgentImportRequest, String> {
    let dirs = agent_import_dirs()?;
    read_json(&job_file(&dirs.processing, safe_job_id(job_id)?)?)
}

fn finish_import(
    job_id: &str,
    result: &AgentImportResult,
    target_state: AgentJobState,
) -> Result<AgentImportStatus, String> {
    safe_job_id(job_id)?;
    if !matches!(target_state, AgentJobState::Done | AgentJobState::Failed) {
        return Err("Agent import can only finish as done or failed.".to_string());
    }
    let dirs = agent_import_dirs()?;
    dirs.ensure()?;
    let current_state = import_state_for_job(&dirs, job_id)?;
    if current_state == target_state {
        return import_status(job_id);
    }
    if current_state != AgentJobState::Processing {
        return Err(format!(
            "Agent import {job_id} is not processing (state: {:?}).",
            current_state
        ));
    }

    let mut stored_result = result.clone();
    stored_result.schema_version = AGENT_IMPORT_SCHEMA_VERSION;
    stored_result.job_id = job_id.to_string();
    stored_result.status = target_state.clone();
    stored_result.success = target_state == AgentJobState::Done;
    if stored_result.completed_at.trim().is_empty() {
        stored_result.completed_at = current_timestamp();
    }

    let target_directory = match target_state {
        AgentJobState::Done => &dirs.done,
        AgentJobState::Failed => &dirs.failed,
        _ => unreachable!(),
    };
    let finished = job_file(target_directory, job_id)?;
    if finished.exists() {
        let processing = job_file(&dirs.processing, job_id)?;
        fs::remove_file(&processing).map_err(|error| {
            format!("Could not remove duplicate processing job {job_id}: {error}")
        })?;
        return import_status(job_id);
    }
    let bytes = serde_json::to_vec_pretty(&stored_result).map_err(|error| error.to_string())?;
    let stored_result_path = result_file(target_directory, job_id)?;
    if stored_result_path.is_file() {
        let existing: AgentImportResult = read_json(&stored_result_path)?;
        if existing.job_id != job_id || existing.status != target_state {
            return Err(format!(
                "Existing result for agent import {job_id} does not match its terminal state."
            ));
        }
    } else {
        atomic_write(&stored_result_path, &bytes)?;
    }
    let processing = job_file(&dirs.processing, job_id)?;
    fs::rename(&processing, &finished)
        .map_err(|error| format!("Could not finish agent import {job_id}: {error}"))?;
    import_status(job_id)
}

pub fn complete_import(
    job_id: &str,
    result: &AgentImportResult,
) -> Result<AgentImportStatus, String> {
    finish_import(job_id, result, AgentJobState::Done)
}

pub fn fail_import(job_id: &str, result: &AgentImportResult) -> Result<AgentImportStatus, String> {
    finish_import(job_id, result, AgentJobState::Failed)
}

pub fn import_status(job_id: &str) -> Result<AgentImportStatus, String> {
    safe_job_id(job_id)?;
    let dirs = agent_import_dirs()?;
    dirs.ensure()?;
    for (directory, state) in [
        (&dirs.pending, AgentJobState::Pending),
        (&dirs.processing, AgentJobState::Processing),
        (&dirs.done, AgentJobState::Done),
        (&dirs.failed, AgentJobState::Failed),
    ] {
        let request_path = job_file(directory, job_id)?;
        if request_path.is_file() {
            let request = read_json(&request_path).ok();
            let result_path = result_file(directory, job_id)?;
            let result = if result_path.is_file() {
                Some(read_json(&result_path)?)
            } else {
                None
            };
            return Ok(AgentImportStatus {
                job_id: job_id.to_string(),
                state,
                request,
                result,
            });
        }
    }
    Ok(AgentImportStatus {
        job_id: job_id.to_string(),
        state: AgentJobState::Unknown,
        request: None,
        result: None,
    })
}

fn read_app_data(path: &Path) -> Result<StoredAppData, String> {
    if !path.exists() {
        return Ok(StoredAppData::default());
    }
    read_json(path)
}

fn active_library_snapshot() -> Result<LibrarySnapshot, String> {
    let setting_dir = legra_setting_dir()?;
    let local = read_app_data(&setting_dir.join("app-data.json"))?;
    if let Some(workspace_root) = local
        .settings
        .workspace_root
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let root = PathBuf::from(workspace_root);
        let workspace_file = root.join("paper-manager-workspace.json");
        if workspace_file.is_file() {
            let workspace = read_app_data(&workspace_file)?;
            return Ok(LibrarySnapshot {
                papers: workspace.papers,
                category_root: Some(root.join("papers")),
                source: "workspace".to_string(),
            });
        }
    }

    if let Some(managed_directory) = local
        .settings
        .managed_directory
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let root = PathBuf::from(managed_directory);
        let library_file = root.join(".legra").join("library.json");
        if library_file.is_file() {
            let library: StoredManagedLibrary = read_json(&library_file)?;
            if library.schema_version > MANAGED_LIBRARY_SCHEMA_VERSION {
                return Err("Managed library was created by a newer Legra version.".to_string());
            }
            return Ok(LibrarySnapshot {
                papers: library.data.papers,
                category_root: Some(root),
                source: "managed_library".to_string(),
            });
        }
    }

    Ok(LibrarySnapshot {
        papers: local.papers,
        category_root: local
            .settings
            .managed_directory
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from),
        source: "local_data".to_string(),
    })
}

fn paper_summary(paper: StoredPaper) -> LibraryPaperSummary {
    let has_pdf = paper
        .pdf_path
        .as_deref()
        .or(paper.original_pdf_path.as_deref())
        .is_some_and(|value| !value.trim().is_empty());
    LibraryPaperSummary {
        id: paper.id,
        title: paper.title,
        authors: paper.authors,
        year: paper.year,
        publication: paper.publication,
        doi: paper.doi,
        arxiv_id: paper.arxiv_id,
        tags: paper.tags,
        folder_category: paper.folder_category,
        has_pdf,
    }
}

fn stored_identifier_matches(paper: &StoredPaper, identifier: &AgentPaperIdentifier) -> bool {
    let candidates = match identifier.kind {
        IdentifierKind::Doi => [paper.doi.as_deref(), None],
        IdentifierKind::Arxiv => [paper.arxiv_id.as_deref(), paper.doi.as_deref()],
    };
    candidates.into_iter().flatten().any(|candidate| {
        normalize_identifier(candidate)
            .ok()
            .is_some_and(|value| value == *identifier)
    })
}

pub fn resolve_paper(identifier: &str) -> Result<ResolvePaperResult, String> {
    let identifier = normalize_identifier(identifier)?;
    let snapshot = active_library_snapshot()?;
    let paper = snapshot
        .papers
        .into_iter()
        .find(|paper| stored_identifier_matches(paper, &identifier))
        .map(paper_summary);
    Ok(ResolvePaperResult {
        identifier,
        found: paper.is_some(),
        paper,
        library_source: snapshot.source,
    })
}

pub fn search_library(query: &str, limit: usize) -> Result<SearchLibraryResult, String> {
    let query = query.split_whitespace().collect::<Vec<_>>().join(" ");
    if query.is_empty() || query.len() > 300 || query.chars().any(char::is_control) {
        return Err("Search query must be 1-300 characters with no controls.".to_string());
    }
    let tokens = query
        .to_lowercase()
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let snapshot = active_library_snapshot()?;
    let mut papers = Vec::new();
    for paper in snapshot.papers {
        let haystack = [
            paper.title.as_str(),
            paper.authors.join(" ").as_str(),
            paper.publication.as_deref().unwrap_or_default(),
            paper.doi.as_deref().unwrap_or_default(),
            paper.arxiv_id.as_deref().unwrap_or_default(),
            paper.tags.join(" ").as_str(),
            paper.folder_category.as_deref().unwrap_or_default(),
        ]
        .join(" ")
        .to_lowercase();
        if tokens.iter().all(|token| haystack.contains(token)) {
            papers.push(paper_summary(paper));
            if papers.len() >= limit.clamp(1, MAX_SEARCH_RESULTS) {
                break;
            }
        }
    }
    Ok(SearchLibraryResult {
        query,
        count: papers.len(),
        papers,
        library_source: snapshot.source,
    })
}

fn add_category_with_ancestors(categories: &mut BTreeSet<String>, category: &str) {
    let parts = category
        .split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty() && !matches!(*part, "." | ".."))
        .collect::<Vec<_>>();
    for index in 0..parts.len() {
        if categories.len() >= MAX_CATEGORY_COUNT {
            break;
        }
        categories.insert(parts[..=index].join("/"));
    }
}

fn collect_directory_categories(
    categories: &mut BTreeSet<String>,
    root: &Path,
    current: &Path,
    depth: usize,
) -> Result<(), String> {
    if depth >= MAX_DIRECTORY_DEPTH || categories.len() >= MAX_CATEGORY_COUNT || !current.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(current).map_err(|error| {
        format!(
            "Could not read category directory {}: {error}",
            current.display()
        )
    })? {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name == "notes" {
            continue;
        }
        let path = entry.path();
        if let Ok(relative) = path.strip_prefix(root) {
            let category = relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            add_category_with_ancestors(categories, &category);
        }
        collect_directory_categories(categories, root, &path, depth + 1)?;
    }
    Ok(())
}

pub fn list_categories() -> Result<CategoryListing, String> {
    let snapshot = active_library_snapshot()?;
    let mut categories = BTreeSet::new();
    for paper in &snapshot.papers {
        if let Some(category) = paper.folder_category.as_deref() {
            add_category_with_ancestors(&mut categories, category);
        }
    }
    if let Some(root) = snapshot.category_root.as_deref() {
        collect_directory_categories(&mut categories, root, root, 0)?;
    }
    let categories = categories.into_iter().collect::<Vec<_>>();
    Ok(CategoryListing {
        count: categories.len(),
        categories,
        library_source: snapshot.source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_strictly_normalized() {
        assert_eq!(
            normalize_identifier("https://doi.org/10.1000/ABC.12").unwrap(),
            AgentPaperIdentifier {
                kind: IdentifierKind::Doi,
                value: "10.1000/abc.12".to_string(),
            }
        );
        assert_eq!(
            normalize_identifier("https://arxiv.org/pdf/2401.12345v2.pdf").unwrap(),
            AgentPaperIdentifier {
                kind: IdentifierKind::Arxiv,
                value: "2401.12345".to_string(),
            }
        );
        assert_eq!(
            normalize_identifier("https://doi.org/10.48550/arXiv.2401.12345v2").unwrap(),
            AgentPaperIdentifier {
                kind: IdentifierKind::Arxiv,
                value: "2401.12345".to_string(),
            }
        );
        assert!(normalize_identifier("https://example.com/paper.pdf").is_err());
        assert!(normalize_identifier("/tmp/paper.pdf").is_err());
    }
}
