use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager};

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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
struct BibtexInput {
    paper_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SaveBibtexInput {
    path: String,
    content: String,
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
struct FetchMetadataInput {
    doi: Option<String>,
    arxiv_id: Option<String>,
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
struct BackupResult {
    backup_dir: String,
}

#[derive(Debug, Serialize)]
struct NoteStatus {
    note_id: String,
    exists: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Settings {
    id: String,
    managed_directory: Option<String>,
    filename_rule: String,
    note_directory: Option<String>,
    cloud_sync_expected: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
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

fn setting_dir() -> Result<PathBuf, String> {
    Ok(app_root_dir()?.join("setting"))
}

fn data_file_path() -> Result<PathBuf, String> {
    Ok(setting_dir()?.join("app-data.json"))
}

fn notes_dir() -> Result<PathBuf, String> {
    Ok(setting_dir()?.join("notes"))
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

fn default_settings(timestamp: &str) -> Settings {
    Settings {
        id: "settings-default".to_string(),
        managed_directory: None,
        filename_rule: "{year}_{first_author}_{journal}.pdf".to_string(),
        note_directory: None,
        cloud_sync_expected: true,
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
    serde_json::from_str(&json).map_err(|error| error.to_string())
}

fn save_data_file(data: &AppData) -> Result<AppStatus, String> {
    let dir = ensure_setting_dir()?;
    let data_file = dir.join("app-data.json");
    let json = serde_json::to_string_pretty(data).map_err(|error| error.to_string())?;
    fs::write(&data_file, json).map_err(|error| error.to_string())?;

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

fn clean_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
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
        .trim_start_matches("http://www.arxiv.org/abs/");

    if let Some((base, version)) = without_url.rsplit_once('v') {
        if !base.is_empty() && version.chars().all(|character| character.is_ascii_digit()) {
            return base.to_string();
        }
    }

    without_url.to_string()
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

fn validate_pdf_path(pdf_path: &Option<String>) -> Result<(), String> {
    let Some(path) = pdf_path else {
        return Ok(());
    };
    let path = Path::new(path);

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

fn sanitize_pdf_part(value: Option<&str>, fallback: &str) -> String {
    let value = value.unwrap_or(fallback).trim();
    if value.is_empty() {
        sanitize_filename(fallback)
    } else {
        sanitize_filename(value)
    }
}

fn target_pdf_path(
    data: &AppData,
    paper: &Paper,
    folder_category: Option<&str>,
) -> Result<PathBuf, String> {
    let target_dir = target_pdf_dir(data, folder_category)?;

    let year = paper
        .year
        .map(|year| year.to_string())
        .unwrap_or_else(|| "unknown_year".to_string());
    let first_author =
        sanitize_pdf_part(paper.authors.first().map(String::as_str), "unknown_author");
    let journal = sanitize_pdf_part(paper.publication.as_deref(), "unknown_journal");
    let base_name = format!("{}_{}_{}", sanitize_filename(&year), first_author, journal);
    let mut candidate = target_dir.join(format!("{base_name}.pdf"));
    let mut index = 2;

    while candidate.exists() {
        candidate = target_dir.join(format!("{base_name}_{index}.pdf"));
        index += 1;
    }

    Ok(candidate)
}

fn target_pdf_dir(data: &AppData, folder_category: Option<&str>) -> Result<PathBuf, String> {
    let managed_directory = data
        .settings
        .managed_directory
        .as_deref()
        .ok_or_else(|| "Managed directory is not set.".to_string())?;
    let managed_directory = PathBuf::from(managed_directory);

    if !managed_directory.exists() {
        return Err("Managed directory does not exist.".to_string());
    }

    if !managed_directory.is_dir() {
        return Err("Managed directory is not a directory.".to_string());
    }

    let category = normalize_optional(folder_category.map(str::to_string));
    let target_dir = if let Some(category) = category {
        managed_directory.join(sanitize_filename(&category))
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
    let current_pdf_path = data.papers[paper_index]
        .pdf_path
        .as_deref()
        .ok_or_else(|| "This paper does not have a PDF path.".to_string())?;
    let current_pdf_path = PathBuf::from(current_pdf_path);

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
        let current_note_path = PathBuf::from(&note.file_path);
        if !current_note_path.exists() || !current_note_path.is_file() {
            continue;
        }

        let file_name = current_note_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("note.md");
        let target_note_path = unique_path(&target_notes_dir, file_name);
        move_file_with_fallback(&current_note_path, &target_note_path)?;
        note.file_path = target_note_path.to_string_lossy().into_owned();
        note.updated_at = current_timestamp()?;
    }

    let timestamp = current_timestamp()?;
    let paper = &mut data.papers[paper_index];
    paper.pdf_path = Some(target.to_string_lossy().into_owned());
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

fn collect_files_by_name(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }

    if root.is_file() {
        files.push(root.to_path_buf());
        return Ok(());
    }

    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        collect_files_by_name(&entry.path(), files)?;
    }

    Ok(())
}

fn find_file_by_basename(root: &Path, original_path: &str) -> Result<Option<String>, String> {
    let Some(file_name) = Path::new(original_path).file_name() else {
        return Ok(None);
    };

    let mut files = Vec::new();
    collect_files_by_name(root, &mut files)?;
    Ok(files
        .into_iter()
        .find(|path| path.file_name() == Some(file_name))
        .map(|path| path.to_string_lossy().into_owned()))
}

fn relink_paths_to_root(data: &mut AppData, root: &Path) -> Result<(), String> {
    for paper in &mut data.papers {
        if let Some(pdf_path) = paper.pdf_path.as_deref() {
            if !Path::new(pdf_path).exists() {
                if let Some(relinked) = find_file_by_basename(root, pdf_path)? {
                    paper.pdf_path = Some(relinked);
                }
            }
        }
    }

    for note in &mut data.notes {
        if !Path::new(&note.file_path).exists() {
            if let Some(relinked) = find_file_by_basename(root, &note.file_path)? {
                note.file_path = relinked;
            }
        }
    }

    Ok(())
}

fn bibtex_escape(value: &str) -> String {
    value.replace('\n', " ").replace('\r', " ")
}

fn bibtex_key(paper: &Paper) -> String {
    if let Some(key) = paper.bibtex_key.as_deref() {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            return sanitize_filename(trimmed);
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

    if let Some(doi) = paper.doi.as_deref() {
        let doi_key = doi
            .split('/')
            .next_back()
            .map(sanitize_filename)
            .filter(|key| !key.is_empty());
        if let Some(key) = doi_key {
            return key;
        }
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

fn paper_to_bibtex(paper: &Paper) -> String {
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

    if paper.arxiv_id.is_some() {
        if let Some(year) = paper.year {
            fields.push(format!("  year = {{{year}}}"));
        }
        push_bibtex_field(&mut fields, "eprint", paper.arxiv_id.as_deref());
        fields.push("  archivePrefix = {arXiv}".to_string());

        return format!("@misc{{{},\n{}\n}}", bibtex_key(paper), fields.join(",\n"));
    }

    push_bibtex_field(&mut fields, "journal", paper.publication.as_deref());
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
        bibtex_key(paper),
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

    validate_pdf_path(&input.pdf_path)?;

    let mut data = load_or_default_app_data()?;
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
    if data.papers[paper_index].pdf_path.is_some() && data.settings.managed_directory.is_some() {
        let folder_category = data.papers[paper_index].folder_category.clone();
        organize_pdf_for_paper(&mut data, paper_index, folder_category)?;
    }
    save_data_file(&data)?;
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

    validate_pdf_path(&input.pdf_path)?;

    let mut data = load_or_default_app_data()?;
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
fn generate_bibtex(input: BibtexInput) -> Result<String, String> {
    if input.paper_ids.is_empty() {
        return Err("Select at least one paper.".to_string());
    }

    let data = load_or_default_app_data()?;
    let entries = input
        .paper_ids
        .iter()
        .map(|paper_id| {
            data.papers
                .iter()
                .find(|paper| &paper.id == paper_id)
                .map(paper_to_bibtex)
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
        "# paper-manager backup\n\nSelect this directory with Restore backup.\n",
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
        file_path: file_path.to_string_lossy().into_owned(),
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

    let normalized_path = path.to_string_lossy().into_owned();
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

#[tauri::command]
fn open_note(note_id: String) -> Result<(), String> {
    let data = load_or_default_app_data()?;
    let note = data
        .notes
        .iter()
        .find(|note| note.id == note_id)
        .ok_or_else(|| "Note was not found.".to_string())?;

    let note_path = Path::new(&note.file_path);
    if !note_path.exists() {
        return Err("Note file does not exist.".to_string());
    }

    if is_markdown_path(note_path) {
        let status = Command::new("open")
            .args(["-a", "MarkText"])
            .arg(note_path)
            .status()
            .map_err(|error| error.to_string())?;

        if status.success() {
            return Ok(());
        }

        return Err(
            "Could not open note with MarkText. Check that MarkText is installed.".to_string(),
        );
    }

    tauri_plugin_opener::open_path(&note.file_path, None::<&str>).map_err(|error| error.to_string())
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
    let pdf_path = Path::new(pdf_path);

    if !pdf_path.exists() {
        return Err("PDF file does not exist.".to_string());
    }

    if !pdf_path.is_file() {
        return Err("PDF path is not a file.".to_string());
    }

    let status = Command::new("open")
        .args(["-a", "Preview"])
        .arg(pdf_path)
        .status()
        .map_err(|error| error.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err("Could not open PDF with Preview.".to_string())
    }
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
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            save_app_data,
            load_app_data,
            fetch_paper_metadata,
            open_register_paper_window,
            open_edit_paper_window,
            register_paper,
            update_paper,
            update_managed_directory,
            organize_paper_pdf,
            generate_bibtex,
            save_bibtex,
            create_backup,
            restore_backup,
            relink_files,
            create_note,
            link_note,
            open_note,
            open_paper_pdf,
            check_note_files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
