import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { open, save } from "@tauri-apps/plugin-dialog";
import "./App.css";

type AppStatus = {
  setting_dir: string;
  data_file: string;
  data_file_exists: boolean;
};

type Paper = {
  id: string;
  title: string;
  authors: string[];
  year?: number;
  publication?: string;
  volume?: string;
  issue?: string;
  pages?: string;
  numpages?: number;
  month?: string;
  publisher?: string;
  doi?: string;
  arxiv_id?: string;
  url?: string;
  abstract_text?: string;
  tags: string[];
  status?: string;
  rating?: number;
  bibtex_key?: string;
  pdf_path?: string;
  original_pdf_path?: string;
  folder_category?: string;
  created_at: string;
  updated_at: string;
};

type AppData = {
  papers: Paper[];
  notes: Note[];
  settings: {
    id: string;
    managed_directory?: string;
    filename_rule: string;
    note_directory?: string;
    marktext_path?: string;
    pdf_viewer_path?: string;
    chrome_import_directory?: string;
    chrome_extension_id?: string;
    bibtex_key_rule: string;
    bibtex_export_rule: string;
    journal_output_style: string;
    journal_aliases: JournalAlias[];
    workspace_root?: string;
    workspace_revision?: number;
    workspace_last_loaded_revision?: number;
    cloud_sync_expected: boolean;
    created_at: string;
    updated_at: string;
  };
};

type JournalAlias = {
  full_name: string;
  abbreviation: string;
  aliases: string[];
};

type Note = {
    id: string;
    paper_id: string;
    title: string;
    file_path: string;
    file_type?: string;
    summary?: string;
    created_at: string;
    updated_at: string;
};

type BackupResult = {
  backup_dir: string;
};

type PaperForm = {
  title: string;
  authors: string;
  year: string;
  publication: string;
  volume: string;
  issue: string;
  pages: string;
  numpages: string;
  month: string;
  publisher: string;
  doi: string;
  arxiv_id: string;
  url: string;
  abstract_text: string;
  tags: string;
  status: string;
  pdf_path: string;
  folder_category: string;
};

type PaperMetadata = {
  title?: string;
  authors: string[];
  year?: number;
  publication?: string;
  volume?: string;
  issue?: string;
  pages?: string;
  numpages?: number;
  month?: string;
  publisher?: string;
  doi?: string;
  arxiv_id?: string;
  url?: string;
  abstract_text?: string;
};

type PaperImportResolution = {
  metadata: PaperMetadata;
  downloaded_pdf_path?: string;
  warnings: string[];
};

type ExtensionImportSummary = {
  imported: number;
  failed: number;
  pending: number;
  messages: string[];
};

type WorkspaceHealth = {
  ok: boolean;
  warnings: string[];
};

type ChromeNativeHostStatus = {
  installed: boolean;
  manifest_path: string;
  manifest_paths: string[];
  host_path: string;
  extension_id: string;
  message: string;
};

type PlatformInfo = {
  os: "macos" | "windows" | "linux" | "unknown";
  path_separator: string;
};

type Filters = {
  query: string;
  tag: string;
  category: string;
  status: string;
  year: string;
  sort: string;
};

type SettingsForm = {
  managed_directory: string;
  marktext_path: string;
  pdf_viewer_path: string;
  chrome_import_directory: string;
  chrome_extension_id: string;
  filename_rule: string;
  bibtex_key_rule: string;
  bibtex_export_rule: string;
  journal_output_style: string;
  journal_aliases: JournalAlias[];
  note_directory: string;
  workspace_root: string;
  cloud_sync_expected: boolean;
};

const initialForm: PaperForm = {
  title: "",
  authors: "",
  year: "",
  publication: "",
  volume: "",
  issue: "",
  pages: "",
  numpages: "",
  month: "",
  publisher: "",
  doi: "",
  arxiv_id: "",
  url: "",
  abstract_text: "",
  tags: "",
  status: "unread",
  pdf_path: "",
  folder_category: "",
};

const initialFilters: Filters = {
  query: "",
  tag: "",
  category: "all",
  status: "all",
  year: "",
  sort: "updated_desc",
};

const initialSettingsForm: SettingsForm = {
  managed_directory: "",
  marktext_path: "",
  pdf_viewer_path: "",
  chrome_import_directory: "",
  chrome_extension_id: "",
  filename_rule: "{year}_{first_author}_{journal}.pdf",
  bibtex_key_rule: "",
  bibtex_export_rule: "doi_preferred",
  journal_output_style: "as_stored",
  journal_aliases: [],
  note_directory: "",
  workspace_root: "",
  cloud_sync_expected: true,
};

function parseList(value: string) {
  return value
    .split(/[,;\n]/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function optionalString(value: string) {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : undefined;
}

function pathDisplayName(value: string | undefined) {
  if (!value) {
    return "Not set";
  }
  return value.split(/[\\/]/).filter(Boolean).pop() || value;
}

function metadataKey(form: PaperForm) {
  const doi = form.doi.trim();
  if (doi) {
    return `doi:${doi.toLowerCase()}`;
  }

  const arxivId = form.arxiv_id.trim();
  if (arxivId) {
    return `arxiv:${arxivId.toLowerCase()}`;
  }

  return "";
}

function mergeResolvedImport(current: PaperForm, resolution: PaperImportResolution): PaperForm {
  return {
    ...metadataToForm(current, resolution.metadata),
    pdf_path: resolution.downloaded_pdf_path ?? current.pdf_path,
  };
}

function isReadyToRegister(form: PaperForm) {
  return form.title.trim().length > 0 && parseList(form.authors).length > 0;
}

function metadataToForm(current: PaperForm, metadata: PaperMetadata): PaperForm {
  const resolvedFromArxiv = Boolean(metadata.arxiv_id) && !metadata.doi;

  return {
    ...current,
    title: metadata.title ?? current.title,
    authors: metadata.authors.length > 0 ? metadata.authors.join(", ") : current.authors,
    year: metadata.year?.toString() ?? current.year,
    publication: metadata.publication ?? current.publication,
    volume: metadata.volume ?? current.volume,
    issue: metadata.issue ?? current.issue,
    pages: metadata.pages ?? current.pages,
    numpages: metadata.numpages?.toString() ?? current.numpages,
    month: metadata.month ?? current.month,
    publisher: metadata.publisher ?? current.publisher,
    doi: resolvedFromArxiv ? "" : metadata.doi ?? current.doi,
    arxiv_id: metadata.arxiv_id ?? current.arxiv_id,
    url: metadata.url ?? current.url,
    abstract_text: metadata.abstract_text ?? current.abstract_text,
  };
}

function paperToForm(paper: Paper): PaperForm {
  return {
    title: paper.title,
    authors: paper.authors.join(", "),
    year: paper.year?.toString() ?? "",
    publication: paper.publication ?? "",
    volume: paper.volume ?? "",
    issue: paper.issue ?? "",
    pages: paper.pages ?? "",
    numpages: paper.numpages?.toString() ?? "",
    month: paper.month ?? "",
    publisher: paper.publisher ?? "",
    doi: paper.doi ?? "",
    arxiv_id: paper.arxiv_id ?? "",
    url: paper.url ?? "",
    abstract_text: paper.abstract_text ?? "",
    tags: paper.tags.join(", "),
    status: paper.status ?? "unread",
    pdf_path: paper.pdf_path ?? "",
    folder_category: paper.folder_category ?? "",
  };
}

function settingsToForm(settings: AppData["settings"]): SettingsForm {
  return {
    managed_directory: settings.managed_directory ?? "",
    marktext_path: settings.marktext_path ?? "",
    pdf_viewer_path: settings.pdf_viewer_path ?? "",
    chrome_import_directory: settings.chrome_import_directory ?? "",
    chrome_extension_id: settings.chrome_extension_id ?? initialSettingsForm.chrome_extension_id,
    filename_rule: settings.filename_rule || initialSettingsForm.filename_rule,
    bibtex_key_rule: settings.bibtex_key_rule ?? "",
    bibtex_export_rule: settings.bibtex_export_rule || initialSettingsForm.bibtex_export_rule,
    journal_output_style:
      settings.journal_output_style || initialSettingsForm.journal_output_style,
    journal_aliases: settings.journal_aliases ?? [],
    note_directory: settings.note_directory ?? "",
    workspace_root: settings.workspace_root ?? "",
    cloud_sync_expected: settings.cloud_sync_expected,
  };
}

function formToInput(form: PaperForm) {
  const year = form.year.trim() ? Number(form.year) : undefined;
  if (year !== undefined && !Number.isInteger(year)) {
    throw new Error("Year must be a number.");
  }
  const numpages = form.numpages.trim() ? Number(form.numpages) : undefined;
  if (numpages !== undefined && !Number.isInteger(numpages)) {
    throw new Error("Number of pages must be a number.");
  }

  return {
    title: form.title,
    authors: parseList(form.authors),
    year,
    publication: optionalString(form.publication),
    volume: optionalString(form.volume),
    issue: optionalString(form.issue),
    pages: optionalString(form.pages),
    numpages,
    month: optionalString(form.month),
    publisher: optionalString(form.publisher),
    doi: optionalString(form.doi),
    arxiv_id: optionalString(form.arxiv_id),
    url: optionalString(form.url),
    abstract_text: optionalString(form.abstract_text),
    tags: parseList(form.tags),
    status: optionalString(form.status),
    pdf_path: optionalString(form.pdf_path),
    folder_category: optionalString(form.folder_category),
  };
}

function includesText(value: string | undefined, query: string) {
  return value?.toLowerCase().includes(query) ?? false;
}

function categoryAncestors(category: string) {
  const parts = category
    .split("/")
    .map((part) => part.trim())
    .filter(Boolean);
  return parts.map((_, index) => parts.slice(0, index + 1).join("/"));
}

function categoryMatchesFilter(category: string | undefined, filter: string) {
  if (filter === "all") {
    return true;
  }

  if (filter === "none") {
    return !category;
  }

  const normalizedCategory = category?.trim();
  if (!normalizedCategory) {
    return false;
  }

  return normalizedCategory === filter || normalizedCategory.startsWith(`${filter}/`);
}

function App() {
  const currentWindowLabel = getCurrentWebviewWindow().label;
  const isRegisterWindow = currentWindowLabel === "register-paper";
  const isSettingsWindow = currentWindowLabel === "settings";
  const editWindowPaperId = currentWindowLabel.startsWith("edit-paper-")
    ? currentWindowLabel.slice("edit-paper-".length)
    : null;
  const isEditWindow = editWindowPaperId !== null;
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [platformInfo, setPlatformInfo] = useState<PlatformInfo | null>(null);
  const [data, setData] = useState<AppData | null>(null);
  const [form, setForm] = useState<PaperForm>(initialForm);
  const [editForm, setEditForm] = useState<PaperForm>(initialForm);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [focusedPaperId, setFocusedPaperId] = useState<string | null>(null);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [filters, setFilters] = useState<Filters>(initialFilters);
  const [settingsForm, setSettingsForm] = useState<SettingsForm>(initialSettingsForm);
  const [utilityMenuOpen, setUtilityMenuOpen] = useState(false);
  const [bibtexOutput, setBibtexOutput] = useState("");
  const [bibtexJournalStyle, setBibtexJournalStyle] = useState(
    initialSettingsForm.journal_output_style,
  );
  const [chromeNativeHostStatus, setChromeNativeHostStatus] =
    useState<ChromeNativeHostStatus | null>(null);
  const [importSource, setImportSource] = useState("");
  const [metadataLoading, setMetadataLoading] = useState(false);
  const [message, setMessage] = useState("Loading saved papers...");
  const [error, setError] = useState<string | null>(null);

  const allPapers = data?.papers ?? [];
  const editingPaper = allPapers.find((paper) => paper.id === editingId) ?? null;
  const focusedPaper = allPapers.find((paper) => paper.id === focusedPaperId) ?? null;
  const utilityMenuRef = useRef<HTMLDivElement | null>(null);
  const importProcessingRef = useRef(false);

  const availableTags = useMemo(
    () => Array.from(new Set(allPapers.flatMap((paper) => paper.tags))).sort(),
    [allPapers],
  );
  const availableCategories = useMemo(
    () =>
      Array.from(
        new Set(
          allPapers
            .map((paper) => paper.folder_category?.trim())
            .filter((category): category is string => Boolean(category))
            .flatMap(categoryAncestors),
        ),
      ).sort(),
    [allPapers],
  );

  const filteredPapers = useMemo(() => {
    const query = filters.query.trim().toLowerCase();
    const year = filters.year.trim();

    return [...allPapers]
      .filter((paper) => {
        const matchesQuery =
          query.length === 0 ||
          includesText(paper.title, query) ||
          paper.authors.some((author) => author.toLowerCase().includes(query)) ||
          includesText(paper.doi, query) ||
          includesText(paper.arxiv_id, query);
        const matchesTag = filters.tag === "" || paper.tags.includes(filters.tag);
        const matchesCategory = categoryMatchesFilter(paper.folder_category, filters.category);
        const matchesStatus = filters.status === "all" || paper.status === filters.status;
        const matchesYear = year.length === 0 || paper.year?.toString() === year;
        return matchesQuery && matchesTag && matchesCategory && matchesStatus && matchesYear;
      })
      .sort((first, second) => {
        switch (filters.sort) {
          case "year_desc":
            return (second.year ?? 0) - (first.year ?? 0);
          case "year_asc":
            return (first.year ?? 0) - (second.year ?? 0);
          case "title_asc":
            return first.title.localeCompare(second.title);
          default:
            return second.updated_at.localeCompare(first.updated_at);
        }
      });
  }, [allPapers, filters]);

  async function refreshStatus() {
    const nextStatus = await invoke<AppStatus>("get_app_status");
    setStatus(nextStatus);
  }

  async function loadSavedData() {
    setError(null);
    const [nextStatus, nextData, nextPlatformInfo] = await Promise.all([
      invoke<AppStatus>("get_app_status"),
      invoke<AppData>("load_app_data"),
      invoke<PlatformInfo>("get_platform_info"),
    ]);
    setStatus(nextStatus);
    setData(nextData);
    setPlatformInfo(nextPlatformInfo);
    setSettingsForm(settingsToForm(nextData.settings));
    setBibtexJournalStyle(
      nextData.settings.journal_output_style || initialSettingsForm.journal_output_style,
    );
    setMessage(`Loaded ${nextData.papers.length} papers.`);
  }

  async function processExtensionImports(options: { silent?: boolean } = {}) {
    if (importProcessingRef.current) {
      return;
    }

    importProcessingRef.current = true;
    if (!options.silent) {
      setError(null);
    }
    try {
      const summary = await invoke<ExtensionImportSummary>("process_extension_imports");
      if (summary.imported > 0 || summary.failed > 0) {
        await loadSavedData();
        setMessage(
          `Chrome imports: ${summary.imported} imported, ${summary.failed} failed, ${summary.pending} pending.`,
        );
      } else if (!options.silent) {
        setMessage("No Chrome extension imports are pending.");
      }
      if (summary.failed > 0) {
        setError(summary.messages.join(" "));
      }
    } catch (reason) {
      if (!options.silent) {
        setError(String(reason));
        setMessage("Could not process Chrome extension imports.");
      }
    } finally {
      importProcessingRef.current = false;
    }
  }

  async function choosePdf(target: "register" | "edit") {
    setError(null);
    const selected = await open({
      multiple: false,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });

    if (typeof selected !== "string") {
      return;
    }

    if (target === "register") {
      setForm((current) => ({ ...current, pdf_path: selected }));
    } else {
      setEditForm((current) => ({ ...current, pdf_path: selected }));
    }
  }

  async function chooseManagedDirectory() {
    setError(null);
    const selected = await open({
      directory: true,
      multiple: false,
    });

    if (typeof selected !== "string") {
      return;
    }

    try {
      const nextData = await invoke<AppData>("update_managed_directory", {
        managedDirectory: selected,
      });
      setData(nextData);
      setMessage("Managed directory updated.");
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not update managed directory.");
    }
  }

  async function chooseFolderCategory(target: "register" | "edit") {
    setError(null);
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: data?.settings.managed_directory,
    });

    if (typeof selected !== "string") {
      return;
    }

    try {
      const category = await invoke<string>("resolve_folder_category", {
        selectedDirectory: selected,
      });
      if (target === "register") {
        updateForm("folder_category", category);
      } else {
        updateEditForm("folder_category", category);
      }
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function openRegisterPaperWindow() {
    setError(null);
    try {
      await invoke("open_register_paper_window");
      setMessage("Opened register window.");
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not open register window.");
    }
  }

  async function openEditPaperWindow(paperId: string) {
    setError(null);
    try {
      await invoke("open_edit_paper_window", { paperId });
      setMessage("Opened edit window.");
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not open edit window.");
    }
  }

  async function chooseSettingsDirectory(field: keyof SettingsForm) {
    setError(null);
    const selected = await open({
      directory: true,
      multiple: false,
    });

    if (typeof selected === "string") {
      updateSettingsForm(field, selected);
    }
  }

  async function chooseSettingsApp(field: keyof SettingsForm) {
    setError(null);
    const selected = await open({
      directory: platformInfo?.os === "macos",
      multiple: false,
    });

    if (typeof selected === "string") {
      updateSettingsForm(field, selected);
    }
  }

  async function createBackup() {
    setError(null);
    const selected = await open({
      directory: true,
      multiple: false,
    });

    if (typeof selected !== "string") {
      return;
    }

    try {
      const result = await invoke<BackupResult>("create_backup", {
        input: { target_dir: selected },
      });
      setMessage(`Backup created: ${result.backup_dir}.`);
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not create backup.");
    }
  }

  async function restoreBackup() {
    setError(null);
    const selected = await open({
      directory: true,
      multiple: false,
    });

    if (typeof selected !== "string") {
      return;
    }

    try {
      const nextData = await invoke<AppData>("restore_backup", {
        input: { backup_dir: selected },
      });
      setData(nextData);
      setEditingId(null);
      setEditForm(initialForm);
      setSelectedIds([]);
      await refreshStatus();
      setMessage("Backup restored.");
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not restore backup.");
    }
  }

  async function relinkFiles() {
    setError(null);
    const selected = await open({
      directory: true,
      multiple: false,
    });

    if (typeof selected !== "string") {
      return;
    }

    try {
      const nextData = await invoke<AppData>("relink_files", {
        input: { root_dir: selected },
      });
      setData(nextData);
      if (editingId) {
        const updatedPaper = nextData.papers.find((paper) => paper.id === editingId);
        if (updatedPaper) {
          setEditForm(paperToForm(updatedPaper));
        }
      }
      setMessage("File paths relinked.");
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not relink files.");
    }
  }

  async function chooseWorkspaceDirectory() {
    const selected = await open({
      directory: true,
      multiple: false,
    });
    return typeof selected === "string" ? selected : null;
  }

  async function createSharedWorkspace() {
    setError(null);
    const selected = await chooseWorkspaceDirectory();
    if (!selected) {
      return;
    }

    try {
      const nextData = await invoke<AppData>("create_shared_workspace", {
        input: { workspace_dir: selected },
      });
      setData(nextData);
      setSettingsForm(settingsToForm(nextData.settings));
      await refreshStatus();
      setMessage("Shared workspace created.");
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not create shared workspace.");
    }
  }

  async function openSharedWorkspace() {
    setError(null);
    const selected = await chooseWorkspaceDirectory();
    if (!selected) {
      return;
    }

    try {
      const nextData = await invoke<AppData>("open_shared_workspace", {
        input: { workspace_dir: selected },
      });
      setData(nextData);
      setSettingsForm(settingsToForm(nextData.settings));
      setSelectedIds([]);
      setFocusedPaperId(null);
      await refreshStatus();
      setMessage("Shared workspace opened.");
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not open shared workspace.");
    }
  }

  async function convertCurrentLibraryToWorkspace() {
    setError(null);
    const selected = await chooseWorkspaceDirectory();
    if (!selected) {
      return;
    }

    const confirmed = window.confirm(
      "Convert the current library into this shared workspace? Existing files are copied into the workspace.",
    );
    if (!confirmed) {
      return;
    }

    try {
      const nextData = await invoke<AppData>("convert_current_library_to_workspace", {
        input: { workspace_dir: selected },
      });
      setData(nextData);
      setSettingsForm(settingsToForm(nextData.settings));
      await refreshStatus();
      setMessage("Current library converted to a shared workspace.");
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not convert library to shared workspace.");
    }
  }

  async function checkWorkspaceHealth() {
    setError(null);
    try {
      const health = await invoke<WorkspaceHealth>("check_workspace_health");
      if (health.ok) {
        setMessage("Workspace health check passed.");
      } else {
        setError(health.warnings.join(" "));
        setMessage(`Workspace health check found ${health.warnings.length} issue(s).`);
      }
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not check workspace health.");
    }
  }

  async function fetchMetadataForRegistration() {
    const key = metadataKey(form);
    if (!key) {
      setError("Enter a DOI or arXiv ID before fetching metadata.");
      return false;
    }

    setError(null);
    setMetadataLoading(true);
    try {
      const metadata = await invoke<PaperMetadata>("fetch_paper_metadata", {
        input: {
          doi: optionalString(form.doi),
          arxiv_id: optionalString(form.arxiv_id),
        },
      });
      setForm((current) => metadataToForm(current, metadata));
      setMessage("Metadata fetched. Review the fields before registering.");
      return true;
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not fetch metadata. You can still register manually.");
      return false;
    } finally {
      setMetadataLoading(false);
    }
  }

  async function resolvePaperImport() {
    if (!importSource.trim()) {
      setError("Enter a DOI, arXiv ID, or paper URL before fetching.");
      return null;
    }

    setError(null);
    setMetadataLoading(true);
    try {
      const resolution = await invoke<PaperImportResolution>("resolve_paper_import", {
        input: { source: importSource },
      });
      setForm((current) => mergeResolvedImport(current, resolution));
      const warningText =
        resolution.warnings.length > 0 ? ` ${resolution.warnings.join(" ")}` : "";
      setMessage(
        resolution.downloaded_pdf_path
          ? `Metadata fetched and PDF downloaded.${warningText}`
          : `Metadata fetched.${warningText}`,
      );
      return resolution;
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not resolve paper input.");
      return null;
    } finally {
      setMetadataLoading(false);
    }
  }

  async function registerForm(registrationForm: PaperForm) {
    setError(null);

    if (!isReadyToRegister(registrationForm)) {
      setError("Title and at least one author are required.");
      setMessage("Registration failed.");
      return;
    }

    try {
      const nextData = await invoke<AppData>("register_paper", {
        input: formToInput(registrationForm),
      });
      setData(nextData);
      await refreshStatus();
      setForm(initialForm);
      setImportSource("");
      const registeredPaper = nextData.papers[nextData.papers.length - 1];
      setMessage(
        registeredPaper?.pdf_path
          ? `Registered and organized "${registeredPaper.title}".`
          : `Registered "${registeredPaper?.title}".`,
      );
    } catch (reason) {
      setError(String(reason));
      setMessage("Registration failed.");
    }
  }

  async function registerPaper(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await registerForm(form);
  }

  async function fetchAndRegisterPaper() {
    const resolution = await resolvePaperImport();
    if (!resolution) {
      return;
    }

    const resolvedForm = mergeResolvedImport(form, resolution);
    await registerForm(resolvedForm);
  }

  async function saveEditedPaper(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!editingId) {
      setError("Select a paper before saving changes.");
      return;
    }

    setError(null);
    try {
      const nextData = await invoke<AppData>("update_paper", {
        input: {
          id: editingId,
          ...formToInput(editForm),
        },
      });
      setData(nextData);
      const updatedPaper = nextData.papers.find((paper) => paper.id === editingId);
      if (updatedPaper) {
        setEditForm(paperToForm(updatedPaper));
      }
      setMessage(`Updated "${editForm.title}".`);
    } catch (reason) {
      setError(String(reason));
      setMessage("Update failed.");
    }
  }

  async function saveSettings(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    try {
      const nextData = await invoke<AppData>("update_settings", {
        input: {
          managed_directory: optionalString(settingsForm.managed_directory),
          marktext_path: optionalString(settingsForm.marktext_path),
          pdf_viewer_path: optionalString(settingsForm.pdf_viewer_path),
          chrome_import_directory: optionalString(settingsForm.chrome_import_directory),
          chrome_extension_id: optionalString(settingsForm.chrome_extension_id),
          filename_rule: settingsForm.filename_rule,
          bibtex_key_rule: settingsForm.bibtex_key_rule,
          bibtex_export_rule: settingsForm.bibtex_export_rule,
          journal_output_style: settingsForm.journal_output_style,
          journal_aliases: settingsForm.journal_aliases,
          note_directory: optionalString(settingsForm.note_directory),
          cloud_sync_expected: settingsForm.cloud_sync_expected,
        },
      });
      setData(nextData);
      setSettingsForm(settingsToForm(nextData.settings));
      setMessage("Settings updated.");
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not update settings.");
    }
  }

  async function checkChromeNativeHostStatus() {
    setError(null);
    try {
      const status = await invoke<ChromeNativeHostStatus>("check_chrome_native_host", {
        input: { extension_id: optionalString(settingsForm.chrome_extension_id) },
      });
      setChromeNativeHostStatus(status);
      setMessage(status.message);
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not check Chrome Native Host.");
    }
  }

  async function installChromeNativeHost() {
    setError(null);
    try {
      const status = await invoke<ChromeNativeHostStatus>("install_chrome_native_host", {
        input: { extension_id: optionalString(settingsForm.chrome_extension_id) },
      });
      setChromeNativeHostStatus(status);
      setMessage(status.message);
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not install Chrome Native Host.");
    }
  }

  async function uninstallChromeNativeHost() {
    setError(null);
    try {
      const status = await invoke<ChromeNativeHostStatus>("uninstall_chrome_native_host");
      setChromeNativeHostStatus(status);
      setMessage(status.message);
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not uninstall Chrome Native Host.");
    }
  }

  async function openPaperNote(noteId: string) {
    setError(null);
    try {
      await invoke("open_note", { noteId });
      setMessage("Opened note in external editor.");
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not open note.");
    }
  }

  async function openPaperPdf(paperId: string) {
    setError(null);
    try {
      await invoke("open_paper_pdf", { paperId });
      setMessage("Opened PDF in Preview.");
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not open PDF.");
    }
  }

  async function organizeEditingPdf() {
    if (!editingId) {
      setError("Select a paper before organizing PDF.");
      return;
    }

    setError(null);
    try {
      const nextData = await invoke<AppData>("organize_paper_pdf", {
        input: {
          paper_id: editingId,
          folder_category: optionalString(editForm.folder_category),
        },
      });
      setData(nextData);
      const updatedPaper = nextData.papers.find((paper) => paper.id === editingId);
      if (updatedPaper) {
        setEditForm(paperToForm(updatedPaper));
      }
      setMessage("PDF organized into managed directory.");
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not organize PDF.");
    }
  }

  async function deletePapers(paperIds: string[]) {
    const uniqueIds = Array.from(new Set(paperIds)).filter(Boolean);
    if (uniqueIds.length === 0) {
      setError("Select at least one paper to delete.");
      return;
    }

    const label =
      uniqueIds.length === 1
        ? allPapers.find((paper) => paper.id === uniqueIds[0])?.title ?? "this paper"
        : `${uniqueIds.length} papers`;
    const confirmed = window.confirm(
      `Delete ${label} from Legra? PDF and note files will remain on disk.`,
    );
    if (!confirmed) {
      return;
    }

    setError(null);
    try {
      const nextData = await invoke<AppData>("delete_papers", {
        input: { paper_ids: uniqueIds },
      });
      setData(nextData);
      setSelectedIds((current) => current.filter((paperId) => !uniqueIds.includes(paperId)));
      if (focusedPaperId && uniqueIds.includes(focusedPaperId)) {
        setFocusedPaperId(null);
      }
      if (editingId && uniqueIds.includes(editingId)) {
        setEditingId(null);
        setEditForm(initialForm);
      }
      setBibtexOutput("");
      setMessage(`Deleted ${uniqueIds.length} paper${uniqueIds.length === 1 ? "" : "s"}.`);
    } catch (reason) {
      setError(String(reason));
      setMessage("Delete failed.");
    }
  }

  async function generateSelectedBibtex() {
    setError(null);
    try {
      const content = await invoke<string>("generate_bibtex", {
        input: {
          paper_ids: selectedIds,
          journal_output_style: bibtexJournalStyle,
        },
      });
      setBibtexOutput(content);
      setMessage(`Generated BibTeX for ${selectedIds.length} papers.`);
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not generate BibTeX.");
    }
  }

  async function copyBibtex() {
    if (!bibtexOutput.trim()) {
      setError("Generate BibTeX before copying.");
      return;
    }

    try {
      await navigator.clipboard.writeText(bibtexOutput);
      setMessage("Copied BibTeX to clipboard.");
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not copy BibTeX.");
    }
  }

  async function saveBibtexFile() {
    if (!bibtexOutput.trim()) {
      setError("Generate BibTeX before saving.");
      return;
    }

    setError(null);
    const selected = await save({
      defaultPath: "references.bib",
      filters: [{ name: "BibTeX", extensions: ["bib"] }],
    });

    if (typeof selected !== "string") {
      return;
    }

    const outputPath = selected.toLowerCase().endsWith(".bib") ? selected : `${selected}.bib`;
    try {
      await invoke("save_bibtex", {
        input: {
          path: outputPath,
          content: bibtexOutput,
        },
      });
      setMessage(`Saved BibTeX to ${outputPath}.`);
    } catch (reason) {
      setError(String(reason));
      setMessage("Could not save BibTeX.");
    }
  }

  function updateForm(field: keyof PaperForm, value: string) {
    setForm((current) => ({ ...current, [field]: value }));
  }

  function updateEditForm(field: keyof PaperForm, value: string) {
    setEditForm((current) => ({ ...current, [field]: value }));
  }

  function updateFilters(field: keyof Filters, value: string) {
    setFilters((current) => ({ ...current, [field]: value }));
  }

  function updateSettingsForm(field: keyof SettingsForm, value: string | boolean) {
    setSettingsForm((current) => ({ ...current, [field]: value }));
  }

  function addJournalAlias() {
    setSettingsForm((current) => ({
      ...current,
      journal_aliases: [
        ...current.journal_aliases,
        { full_name: "", abbreviation: "", aliases: [] },
      ],
    }));
  }

  function updateJournalAlias(index: number, field: keyof JournalAlias, value: string) {
    setSettingsForm((current) => ({
      ...current,
      journal_aliases: current.journal_aliases.map((alias, aliasIndex) => {
        if (aliasIndex !== index) {
          return alias;
        }

        return {
          ...alias,
          [field]: field === "aliases" ? parseList(value) : value,
        };
      }),
    }));
  }

  function deleteJournalAlias(index: number) {
    setSettingsForm((current) => ({
      ...current,
      journal_aliases: current.journal_aliases.filter((_, aliasIndex) => aliasIndex !== index),
    }));
  }

  function togglePaperSelection(paperId: string) {
    setSelectedIds((current) =>
      current.includes(paperId)
        ? current.filter((selectedId) => selectedId !== paperId)
        : [...current, paperId],
    );
  }

  function toggleAllVisible() {
    const visibleIds = filteredPapers.map((paper) => paper.id);
    const allVisibleSelected = visibleIds.every((paperId) => selectedIds.includes(paperId));
    setSelectedIds((current) =>
      allVisibleSelected
        ? current.filter((paperId) => !visibleIds.includes(paperId))
        : Array.from(new Set([...current, ...visibleIds])),
    );
  }

  useEffect(() => {
    loadSavedData().catch((reason) => {
      setError(String(reason));
      setMessage("Failed to load saved data.");
    });
  }, []);

  useEffect(() => {
    if (isRegisterWindow || isEditWindow || isSettingsWindow) {
      return;
    }

    processExtensionImports({ silent: true }).catch((reason) => {
      setError(String(reason));
      setMessage("Failed to process Chrome extension imports.");
    });

    const interval = window.setInterval(() => {
      processExtensionImports({ silent: true }).catch(() => {});
    }, 15000);

    return () => {
      window.clearInterval(interval);
    };
  }, [isEditWindow, isRegisterWindow, isSettingsWindow]);

  useEffect(() => {
    if (isRegisterWindow || isEditWindow || isSettingsWindow) {
      return;
    }

    let unlisten = () => {};
    listen("paper-manager:data-updated", () => {
      if (importProcessingRef.current) {
        return;
      }

      loadSavedData().catch((reason) => {
        setError(String(reason));
        setMessage("Failed to refresh saved data.");
      });
    })
      .then((cleanup) => {
        unlisten = cleanup;
      })
      .catch(() => {});

    return () => {
      unlisten();
    };
  }, [isEditWindow, isRegisterWindow, isSettingsWindow]);

  useEffect(() => {
    if (!editWindowPaperId || !data) {
      return;
    }

    const paper = data.papers.find((candidate) => candidate.id === editWindowPaperId);
    if (!paper) {
      setEditingId(null);
      setEditForm(initialForm);
      setError("Paper was not found.");
      setMessage("Could not load paper for editing.");
      return;
    }

    setEditingId(paper.id);
    setEditForm(paperToForm(paper));
    setMessage(`Editing "${paper.title}".`);
  }, [data, editWindowPaperId]);

  useEffect(() => {
    if (!utilityMenuOpen) {
      return;
    }

    function closeUtilityMenuOnOutsideClick(event: PointerEvent) {
      const target = event.target;
      if (
        target instanceof Node &&
        utilityMenuRef.current &&
        !utilityMenuRef.current.contains(target)
      ) {
        setUtilityMenuOpen(false);
      }
    }

    document.addEventListener("pointerdown", closeUtilityMenuOnOutsideClick);
    return () => {
      document.removeEventListener("pointerdown", closeUtilityMenuOnOutsideClick);
    };
  }, [utilityMenuOpen]);

  useEffect(() => {
    if (!focusedPaperId) {
      return;
    }

    if (!filteredPapers.some((paper) => paper.id === focusedPaperId)) {
      setFocusedPaperId(null);
    }
  }, [filteredPapers, focusedPaperId]);

  return isSettingsWindow ? (
    <main className="app-shell">
      <section className="workspace register-workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">Settings window</p>
            <h1>Legra</h1>
          </div>
          <div className="status-pill">
            {data?.settings.updated_at ? "Settings loaded" : "Loading"}
          </div>
        </header>

        <section className="panel bridge-panel">
          <div className="storage-info">
            <h2>Storage</h2>
            <p className="path-text">
              {data?.settings.workspace_root
                ? `Workspace: ${pathDisplayName(data.settings.workspace_root)}`
                : `Managed: ${pathDisplayName(data?.settings.managed_directory)}`}
            </p>
            <p>{message}</p>
          </div>
          <div className="storage-actions">
            <button type="button" className="secondary-action" onClick={loadSavedData}>
              Reload
            </button>
          </div>
        </section>

        {error ? <p className="error">Error: {error}</p> : null}

        <form className="panel settings-form" onSubmit={saveSettings}>
          <div className="section-heading">
            <h2>Settings</h2>
            <p>Configure storage, external apps, import staging, and naming rules.</p>
          </div>

          <div className="settings-section">
            <h3>Storage</h3>
            <label>
              Managed directory
              <div className="file-row">
                <input
                  value={settingsForm.managed_directory}
                  onChange={(event) =>
                    updateSettingsForm("managed_directory", event.currentTarget.value)
                  }
                  placeholder={
                    platformInfo?.os === "windows" ? "C:\\path\\to\\library" : "/path/to/library"
                  }
                />
                <button
                  type="button"
                  onClick={() => chooseSettingsDirectory("managed_directory")}
                >
                  Select
                </button>
              </div>
            </label>

            <label>
              Chrome / metadata import directory
              <div className="file-row">
                <input
                  value={settingsForm.chrome_import_directory}
                  onChange={(event) =>
                    updateSettingsForm("chrome_import_directory", event.currentTarget.value)
                  }
                  placeholder="Leave blank for app setting/imports"
                />
                <button
                  type="button"
                  onClick={() => chooseSettingsDirectory("chrome_import_directory")}
                >
                  Select
                </button>
              </div>
            </label>

            <div className="native-host-panel">
              <label>
                Chrome extension ID
                <input
                  value={settingsForm.chrome_extension_id}
                  onChange={(event) =>
                    updateSettingsForm("chrome_extension_id", event.currentTarget.value)
                  }
                  placeholder="32-character Chrome extension ID"
                />
              </label>
              <div className="form-action-row compact-action-row">
                <button
                  type="button"
                  className="secondary-action"
                  onClick={checkChromeNativeHostStatus}
                >
                  Check Native Host
                </button>
                <button
                  type="button"
                  className="secondary-action"
                  onClick={installChromeNativeHost}
                >
                  Install Native Host
                </button>
                <button
                  type="button"
                  className="danger-action"
                  onClick={uninstallChromeNativeHost}
                >
                  Uninstall Native Host
                </button>
              </div>
              {chromeNativeHostStatus ? (
                <dl className="native-host-status">
                  <dt>Status</dt>
                  <dd>{chromeNativeHostStatus.installed ? "Installed" : "Not installed"}</dd>
                  <dt>Manifest</dt>
                  <dd>
                    {(chromeNativeHostStatus.manifest_paths.length > 0
                      ? chromeNativeHostStatus.manifest_paths
                      : [chromeNativeHostStatus.manifest_path]
                    ).join(", ")}
                  </dd>
                  <dt>Host</dt>
                  <dd>{chromeNativeHostStatus.host_path}</dd>
                </dl>
              ) : (
                <p className="settings-help">
                  Install the Native Host manifest after loading the Chrome extension.
                </p>
              )}
            </div>

            <label>
              Note directory
              <div className="file-row">
                <input
                  value={settingsForm.note_directory}
                  onChange={(event) =>
                    updateSettingsForm("note_directory", event.currentTarget.value)
                  }
                  placeholder="Leave blank for app setting/notes"
                />
                <button
                  type="button"
                  onClick={() => chooseSettingsDirectory("note_directory")}
                >
                  Select
                </button>
              </div>
            </label>
          </div>

          <div className="settings-section">
            <h3>Shared workspace</h3>
            <label>
              Workspace root
              <input
                readOnly
                value={settingsForm.workspace_root}
                placeholder="No shared workspace is active"
              />
            </label>
            <p className="settings-help">
              Revision: {data?.settings.workspace_revision ?? "local only"}. Use a Google Drive,
              Dropbox, or iCloud Drive folder to share the workspace with collaborators.
            </p>
            <div className="form-action-row compact-action-row">
              <button type="button" className="secondary-action" onClick={createSharedWorkspace}>
                Create workspace
              </button>
              <button type="button" className="secondary-action" onClick={openSharedWorkspace}>
                Open workspace
              </button>
              <button
                type="button"
                className="secondary-action"
                onClick={convertCurrentLibraryToWorkspace}
              >
                Convert current library
              </button>
              <button type="button" className="secondary-action" onClick={checkWorkspaceHealth}>
                Health check
              </button>
            </div>
          </div>

          <div className="settings-section">
            <h3>External apps</h3>
            <div className="field-row">
              <label>
                {platformInfo?.os === "macos"
                  ? "MarkText path or app name"
                  : "Markdown editor executable or command"}
                <div className="file-row">
                  <input
                    value={settingsForm.marktext_path}
                    onChange={(event) =>
                      updateSettingsForm("marktext_path", event.currentTarget.value)
                    }
                    placeholder={
                      platformInfo?.os === "macos"
                        ? "MarkText or /Applications/MarkText.app"
                        : platformInfo?.os === "windows"
                          ? "C:\\Program Files\\MarkText\\MarkText.exe"
                          : "marktext or /usr/bin/marktext"
                    }
                  />
                  <button type="button" onClick={() => chooseSettingsApp("marktext_path")}>
                    Select
                  </button>
                </div>
              </label>

              <label>
                {platformInfo?.os === "macos"
                  ? "PDF viewer path or app name"
                  : "PDF viewer executable or command"}
                <div className="file-row">
                  <input
                    value={settingsForm.pdf_viewer_path}
                    onChange={(event) =>
                      updateSettingsForm("pdf_viewer_path", event.currentTarget.value)
                    }
                    placeholder={
                      platformInfo?.os === "macos"
                        ? "Preview or /Applications/Preview.app"
                        : platformInfo?.os === "windows"
                          ? "C:\\path\\to\\viewer.exe"
                          : "evince or /usr/bin/evince"
                    }
                  />
                  <button type="button" onClick={() => chooseSettingsApp("pdf_viewer_path")}>
                    Select
                  </button>
                </div>
              </label>
            </div>
          </div>

          <div className="settings-section">
            <h3>Naming rules</h3>
            <label>
              PDF filename rule
              <input
                value={settingsForm.filename_rule}
                onChange={(event) => updateSettingsForm("filename_rule", event.currentTarget.value)}
                placeholder="{year}_{first_author}_{journal}.pdf"
              />
            </label>

            <label>
              BibTeX citation key rule
              <input
                value={settingsForm.bibtex_key_rule}
                onChange={(event) =>
                  updateSettingsForm("bibtex_key_rule", event.currentTarget.value)
                }
                placeholder="Empty: DOI suffix, then arXiv author-year"
              />
            </label>

            <label>
              BibTeX export rule
              <select
                value={settingsForm.bibtex_export_rule}
                onChange={(event) =>
                  updateSettingsForm("bibtex_export_rule", event.currentTarget.value)
                }
              >
                <option value="doi_preferred">DOI / journal metadata preferred</option>
                <option value="arxiv_only_when_no_doi">arXiv only when DOI is missing</option>
              </select>
            </label>

            <label>
              BibTeX journal output
              <select
                value={settingsForm.journal_output_style}
                onChange={(event) =>
                  updateSettingsForm("journal_output_style", event.currentTarget.value)
                }
              >
                <option value="as_stored">As stored</option>
                <option value="full">Full journal name</option>
                <option value="abbreviation">Abbreviation</option>
              </select>
            </label>

            <div className="journal-alias-editor">
              <div className="section-heading compact-heading">
                <h3>Journal aliases</h3>
                <button type="button" className="secondary-action" onClick={addJournalAlias}>
                  Add journal
                </button>
              </div>

              {settingsForm.journal_aliases.length === 0 ? (
                <div className="empty-state compact-empty">
                  No journal aliases registered.
                </div>
              ) : (
                <div className="journal-alias-list">
                  {settingsForm.journal_aliases.map((alias, index) => (
                    <div className="journal-alias-row" key={index}>
                      <label>
                        Full name
                        <input
                          value={alias.full_name}
                          onChange={(event) =>
                            updateJournalAlias(index, "full_name", event.currentTarget.value)
                          }
                          placeholder="Physical Review B"
                        />
                      </label>
                      <label>
                        Abbreviation
                        <input
                          value={alias.abbreviation}
                          onChange={(event) =>
                            updateJournalAlias(index, "abbreviation", event.currentTarget.value)
                          }
                          placeholder="Phys. Rev. B"
                        />
                      </label>
                      <label>
                        Aliases
                        <input
                          value={alias.aliases.join(", ")}
                          onChange={(event) =>
                            updateJournalAlias(index, "aliases", event.currentTarget.value)
                          }
                          placeholder="PRB, Phys Rev B"
                        />
                      </label>
                      <button
                        type="button"
                        className="danger-action"
                        onClick={() => deleteJournalAlias(index)}
                      >
                        Delete
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>

            <p className="settings-help">
              Placeholders: {"{year}"}, {"{first_author}"}, {"{last_name}"}, {"{journal}"}, {"{title}"}, {"{doi_suffix}"}, {"{arxiv_id}"}, {"{volume}"}, {"{pages}"}, {"{category}"}.
            </p>
          </div>

          <label className="checkbox-label">
            <input
              checked={settingsForm.cloud_sync_expected}
              onChange={(event) =>
                updateSettingsForm("cloud_sync_expected", event.currentTarget.checked)
              }
              type="checkbox"
            />
            Cloud sync expected
          </label>

          <div className="form-action-row">
            <button type="submit" className="primary-action">
              Save settings
            </button>
            <button type="button" className="secondary-action" onClick={loadSavedData}>
              Revert
            </button>
          </div>
        </form>
      </section>
    </main>
  ) : isEditWindow ? (
    <main className="app-shell">
      <section className="workspace register-workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">Edit paper window</p>
            <h1>Legra</h1>
          </div>
          <div className="status-pill">
            {editingPaper ? "Editing paper" : "No paper selected"}
          </div>
        </header>

        <section className="panel bridge-panel">
          <div className="storage-info">
            <h2>Storage</h2>
            <p className="path-text">
              {data?.settings.workspace_root
                ? `Workspace: ${pathDisplayName(data.settings.workspace_root)}`
                : `Managed: ${pathDisplayName(data?.settings.managed_directory)}`}
            </p>
            <p>{message}</p>
          </div>
          <div className="storage-actions">
            <button type="button" className="secondary-action" onClick={loadSavedData}>
              Reload
            </button>
          </div>
        </section>

        {error ? <p className="error">Error: {error}</p> : null}

        <form className="panel registration-form detail-form" onSubmit={saveEditedPaper}>
          <div className="section-heading">
            <h2>Paper detail</h2>
            <p>{editingPaper ? editingPaper.title : "Loading selected paper..."}</p>
          </div>

          <fieldset disabled={!editingPaper}>
            <label>
              Title
              <input
                required
                value={editForm.title}
                onChange={(event) => updateEditForm("title", event.currentTarget.value)}
              />
            </label>

            <label>
              Authors
              <input
                required
                value={editForm.authors}
                onChange={(event) => updateEditForm("authors", event.currentTarget.value)}
              />
            </label>

            <div className="field-row">
              <label>
                Year
                <input
                  inputMode="numeric"
                  value={editForm.year}
                  onChange={(event) => updateEditForm("year", event.currentTarget.value)}
                />
              </label>
              <label>
                Status
                <select
                  value={editForm.status}
                  onChange={(event) => updateEditForm("status", event.currentTarget.value)}
                >
                  <option value="unread">Unread</option>
                  <option value="reading">Reading</option>
                  <option value="done">Done</option>
                </select>
              </label>
            </div>

            <label>
              Journal / publication
              <input
                value={editForm.publication}
                onChange={(event) => updateEditForm("publication", event.currentTarget.value)}
              />
            </label>

            <div className="field-row">
              <label>
                Volume
                <input
                  value={editForm.volume}
                  onChange={(event) => updateEditForm("volume", event.currentTarget.value)}
                />
              </label>
              <label>
                Issue
                <input
                  value={editForm.issue}
                  onChange={(event) => updateEditForm("issue", event.currentTarget.value)}
                />
              </label>
            </div>

            <div className="field-row">
              <label>
                Pages
                <input
                  value={editForm.pages}
                  onChange={(event) => updateEditForm("pages", event.currentTarget.value)}
                />
              </label>
              <label>
                Number of pages
                <input
                  inputMode="numeric"
                  value={editForm.numpages}
                  onChange={(event) => updateEditForm("numpages", event.currentTarget.value)}
                />
              </label>
            </div>

            <div className="field-row">
              <label>
                Month
                <input
                  value={editForm.month}
                  onChange={(event) => updateEditForm("month", event.currentTarget.value)}
                />
              </label>
              <label>
                Publisher
                <input
                  value={editForm.publisher}
                  onChange={(event) => updateEditForm("publisher", event.currentTarget.value)}
                />
              </label>
            </div>

            <div className="field-row">
              <label>
                DOI
                <input
                  value={editForm.doi}
                  onChange={(event) => updateEditForm("doi", event.currentTarget.value)}
                />
              </label>
              <label>
                arXiv ID
                <input
                  value={editForm.arxiv_id}
                  onChange={(event) => updateEditForm("arxiv_id", event.currentTarget.value)}
                />
              </label>
            </div>

            <label>
              URL
              <input
                value={editForm.url}
                onChange={(event) => updateEditForm("url", event.currentTarget.value)}
              />
            </label>

            <label>
              Abstract
              <textarea
                value={editForm.abstract_text}
                onChange={(event) => updateEditForm("abstract_text", event.currentTarget.value)}
              />
            </label>

            <label>
              Tags
              <input
                value={editForm.tags}
                onChange={(event) => updateEditForm("tags", event.currentTarget.value)}
                placeholder="tag1, tag2"
              />
            </label>

            <label>
              PDF path
              <div className="file-row">
                <input
                  value={editForm.pdf_path}
                  onChange={(event) => updateEditForm("pdf_path", event.currentTarget.value)}
                />
                <button type="button" onClick={() => choosePdf("edit")}>
                  Select
                </button>
              </div>
            </label>

            <label>
              Folder category
              <div className="file-row">
                <input
                  value={editForm.folder_category}
                  onChange={(event) =>
                    updateEditForm("folder_category", event.currentTarget.value)
                  }
                  placeholder={data?.settings.managed_directory ?? "Set managed directory first"}
                />
                <button
                  type="button"
                  disabled={!data?.settings.managed_directory}
                  onClick={() => chooseFolderCategory("edit")}
                >
                  Select
                </button>
              </div>
            </label>

            <button type="submit" className="primary-action">
              Save changes
            </button>
            <button
              type="button"
              className="secondary-action"
              disabled={!editingPaper?.pdf_path || !data?.settings.managed_directory}
              onClick={organizeEditingPdf}
            >
              Organize PDF
            </button>
          </fieldset>
        </form>
      </section>
    </main>
  ) : isRegisterWindow ? (
    <main className="app-shell">
      <section className="workspace register-workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">Register paper window</p>
            <h1>Legra</h1>
          </div>
          <div className="status-pill">
            {status?.data_file_exists ? `${allPapers.length} papers` : "No saved data"}
          </div>
        </header>

        <section className="panel bridge-panel">
          <div className="storage-info">
            <h2>Storage</h2>
            <p className="path-text">
              {data?.settings.workspace_root
                ? `Workspace: ${pathDisplayName(data.settings.workspace_root)}`
                : `Managed: ${pathDisplayName(data?.settings.managed_directory)}`}
            </p>
            <p>{message}</p>
          </div>
          <div className="storage-actions">
            <button type="button" className="secondary-action" onClick={chooseManagedDirectory}>
              Set directory
            </button>
            <button type="button" className="secondary-action" onClick={loadSavedData}>
              Reload
            </button>
          </div>
        </section>

        {error ? <p className="error">Error: {error}</p> : null}

        <form className="panel registration-form" onSubmit={registerPaper}>
          <div className="section-heading">
            <h2>Register paper</h2>
            <p>Paste a DOI, arXiv ID, or paper URL to fetch metadata and import available PDFs.</p>
          </div>

          <label>
            Paper ID or URL
            <input
              value={importSource}
              onChange={(event) => setImportSource(event.currentTarget.value)}
              placeholder="10.1103/... or 2401.00000 or https://arxiv.org/abs/..."
            />
          </label>

          <div className="form-action-row">
            <button
              type="button"
              className="secondary-action"
              disabled={metadataLoading || !importSource.trim()}
              onClick={resolvePaperImport}
            >
              {metadataLoading ? "Fetching..." : "Fetch metadata"}
            </button>
            <button
              type="button"
              className="primary-action"
              disabled={metadataLoading || !importSource.trim()}
              onClick={fetchAndRegisterPaper}
            >
              Fetch and register
            </button>
            <button type="submit" className="secondary-action">
              Register manually
            </button>
          </div>

          <label>
            PDF path
            <div className="file-row">
              <input
                value={form.pdf_path}
                onChange={(event) => updateForm("pdf_path", event.currentTarget.value)}
                placeholder="/path/to/paper.pdf"
              />
              <button type="button" onClick={() => choosePdf("register")}>
                Select
              </button>
            </div>
          </label>

          <label>
            Folder category
            <div className="file-row">
              <input
                value={form.folder_category}
                onChange={(event) => updateForm("folder_category", event.currentTarget.value)}
                placeholder={data?.settings.managed_directory ?? "Set managed directory first"}
              />
              <button
                type="button"
                disabled={!data?.settings.managed_directory}
                onClick={() => chooseFolderCategory("register")}
              >
                Select
              </button>
            </div>
          </label>

          <details className="advanced-details">
            <summary>Advanced details</summary>

            <div className="advanced-details-body">
              <div className="field-row">
                <label>
                  DOI
                  <input
                    value={form.doi}
                    onChange={(event) => updateForm("doi", event.currentTarget.value)}
                    placeholder="10.1103/..."
                  />
                </label>
                <label>
                  arXiv ID
                  <input
                    value={form.arxiv_id}
                    onChange={(event) => updateForm("arxiv_id", event.currentTarget.value)}
                    placeholder="2401.00000"
                  />
                </label>
              </div>

              <div className="form-action-row compact-action-row">
                <button
                  type="button"
                  className="secondary-action"
                  disabled={metadataLoading || !metadataKey(form)}
                  onClick={fetchMetadataForRegistration}
                >
                  {metadataLoading ? "Fetching..." : "Fetch from DOI/arXiv fields"}
                </button>
              </div>

              <label>
                Title
                <input
                  value={form.title}
                  onChange={(event) => updateForm("title", event.currentTarget.value)}
                  placeholder="Paper title"
                />
              </label>

              <label>
                Authors
                <input
                  value={form.authors}
                  onChange={(event) => updateForm("authors", event.currentTarget.value)}
                  placeholder="First Author, Second Author"
                />
              </label>

              <div className="field-row">
                <label>
                  Year
                  <input
                    inputMode="numeric"
                    value={form.year}
                    onChange={(event) => updateForm("year", event.currentTarget.value)}
                    placeholder="2026"
                  />
                </label>
                <label>
                  Status
                  <select
                    value={form.status}
                    onChange={(event) => updateForm("status", event.currentTarget.value)}
                  >
                    <option value="unread">Unread</option>
                    <option value="reading">Reading</option>
                    <option value="done">Done</option>
                  </select>
                </label>
              </div>

              <label>
                Journal / publication
                <input
                  value={form.publication}
                  onChange={(event) => updateForm("publication", event.currentTarget.value)}
                  placeholder="Phys. Rev. X"
                />
              </label>

              <div className="field-row">
                <label>
                  Volume
                  <input
                    value={form.volume}
                    onChange={(event) => updateForm("volume", event.currentTarget.value)}
                  />
                </label>
                <label>
                  Issue
                  <input
                    value={form.issue}
                    onChange={(event) => updateForm("issue", event.currentTarget.value)}
                  />
                </label>
              </div>

              <div className="field-row">
                <label>
                  Pages
                  <input
                    value={form.pages}
                    onChange={(event) => updateForm("pages", event.currentTarget.value)}
                  />
                </label>
                <label>
                  Number of pages
                  <input
                    inputMode="numeric"
                    value={form.numpages}
                    onChange={(event) => updateForm("numpages", event.currentTarget.value)}
                  />
                </label>
              </div>

              <div className="field-row">
                <label>
                  Month
                  <input
                    value={form.month}
                    onChange={(event) => updateForm("month", event.currentTarget.value)}
                  />
                </label>
                <label>
                  Publisher
                  <input
                    value={form.publisher}
                    onChange={(event) => updateForm("publisher", event.currentTarget.value)}
                  />
                </label>
              </div>

              <label>
                URL
                <input
                  value={form.url}
                  onChange={(event) => updateForm("url", event.currentTarget.value)}
                  placeholder="https://..."
                />
              </label>

              <label>
                Abstract
                <textarea
                  value={form.abstract_text}
                  onChange={(event) => updateForm("abstract_text", event.currentTarget.value)}
                  placeholder="Abstract"
                />
              </label>

              <label>
                Tags
                <input
                  value={form.tags}
                  onChange={(event) => updateForm("tags", event.currentTarget.value)}
                  placeholder="magnetism, spintronics"
                />
              </label>
            </div>
          </details>

        </form>
      </section>
    </main>
  ) : (
    <main className="app-shell">
      <section className="workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">Phase 3 list search and detail editing</p>
            <h1>Legra</h1>
          </div>
          <div className="status-pill">
            {status?.data_file_exists ? `${allPapers.length} papers` : "No saved data"}
          </div>
        </header>

        <section className="panel bridge-panel">
          <div className="storage-info">
            <h2>Storage</h2>
            <p className="path-text">
              {data?.settings.workspace_root
                ? `Workspace: ${pathDisplayName(data.settings.workspace_root)}`
                : `Managed: ${pathDisplayName(data?.settings.managed_directory)}`}
            </p>
            <p>{message}</p>
          </div>
          <div className="storage-actions">
            <button type="button" className="secondary-action" onClick={chooseManagedDirectory}>
              Set directory
            </button>
            <button type="button" className="secondary-action" onClick={openRegisterPaperWindow}>
              Register
            </button>
            <button type="button" className="secondary-action" onClick={loadSavedData}>
              Reload
            </button>
            <div className="utility-menu" ref={utilityMenuRef}>
              <button
                type="button"
                className="utility-menu-trigger"
                aria-expanded={utilityMenuOpen}
                onClick={() => setUtilityMenuOpen((current) => !current)}
              >
                More
              </button>
              {utilityMenuOpen ? (
                <div className="utility-menu-body">
                <button
                  type="button"
                  className="secondary-action"
                  onClick={() => {
                    setUtilityMenuOpen(false);
                    processExtensionImports();
                  }}
                >
                  Import inbox
                </button>
                <button
                  type="button"
                  className="secondary-action"
                  onClick={() => {
                    setUtilityMenuOpen(false);
                    checkWorkspaceHealth();
                  }}
                >
                  Workspace health
                </button>
                <button
                  type="button"
                  className="secondary-action"
                  onClick={() => {
                    setUtilityMenuOpen(false);
                    createBackup();
                  }}
                >
                  Backup
                </button>
                <button
                  type="button"
                  className="secondary-action"
                  onClick={() => {
                    setUtilityMenuOpen(false);
                    restoreBackup();
                  }}
                >
                  Restore
                </button>
                <button
                  type="button"
                  className="secondary-action"
                  onClick={() => {
                    setUtilityMenuOpen(false);
                    relinkFiles();
                  }}
                >
                  Relink
                </button>
                </div>
              ) : null}
            </div>
          </div>
        </section>

        {error ? <p className="error">Error: {error}</p> : null}

        <section className="content-grid">
          <div className="left-stack">
            <section className="panel paper-index-panel">
              <div className="section-heading">
                <h2>Registered papers</h2>
                <p>
                  {filteredPapers.length} shown, {selectedIds.length} selected.
                </p>
              </div>

              <div className="filters-wrapper">
                <div className="filters compact-filters">
                  <input
                    value={filters.query}
                    onChange={(event) => updateFilters("query", event.currentTarget.value)}
                    placeholder="Search title, author, DOI, arXiv"
                  />
                  <select
                    value={filters.tag}
                    onChange={(event) => updateFilters("tag", event.currentTarget.value)}
                  >
                    <option value="">All tags</option>
                    {availableTags.map((tag) => (
                      <option key={tag} value={tag}>
                        {tag}
                      </option>
                    ))}
                  </select>
                  <select
                    value={filters.category}
                    onChange={(event) => updateFilters("category", event.currentTarget.value)}
                  >
                    <option value="all">All categories</option>
                    <option value="none">No category</option>
                    {availableCategories.map((category) => (
                      <option key={category} value={category}>
                        {category}
                      </option>
                    ))}
                  </select>
                  <select
                    value={filters.status}
                    onChange={(event) => updateFilters("status", event.currentTarget.value)}
                  >
                    <option value="all">All status</option>
                    <option value="unread">Unread</option>
                    <option value="reading">Reading</option>
                    <option value="done">Done</option>
                  </select>
                  <input
                    inputMode="numeric"
                    value={filters.year}
                    onChange={(event) => updateFilters("year", event.currentTarget.value)}
                    placeholder="Year"
                  />
                  <select
                    value={filters.sort}
                    onChange={(event) => updateFilters("sort", event.currentTarget.value)}
                  >
                    <option value="updated_desc">Recently updated</option>
                    <option value="year_desc">Year descending</option>
                    <option value="year_asc">Year ascending</option>
                    <option value="title_asc">Title A-Z</option>
                  </select>
                </div>
              </div>

              <div className="list-toolbar">
                <button type="button" className="text-button" onClick={toggleAllVisible}>
                  Toggle visible
                </button>
                <button type="button" className="text-button" onClick={() => setSelectedIds([])}>
                  Clear
                </button>
                <button
                  type="button"
                  className="danger-action"
                  disabled={selectedIds.length === 0}
                  onClick={() => deletePapers(selectedIds)}
                >
                  Delete selected
                </button>
              </div>

              {allPapers.length === 0 ? (
                <div className="empty-state compact-empty">No papers registered yet.</div>
              ) : filteredPapers.length === 0 ? (
                <div className="empty-state compact-empty">
                  No papers match.
                </div>
              ) : (
                <div className="compact-paper-list">
                  {filteredPapers.map((paper) => (
                    <div
                      className={`compact-paper-row ${
                        focusedPaperId === paper.id ? "active-compact-paper" : ""
                      }`}
                      key={paper.id}
                    >
                      <input
                        aria-label={`Select ${paper.title}`}
                        checked={selectedIds.includes(paper.id)}
                        onChange={() => togglePaperSelection(paper.id)}
                        type="checkbox"
                      />
                      <button
                        type="button"
                        className="compact-paper-title"
                        onClick={() => setFocusedPaperId(paper.id)}
                        onDoubleClick={() => openPaperPdf(paper.id)}
                      >
                        {paper.title}
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </section>
          </div>

          <div className="right-stack">
            <section className="panel bibtex-panel">
              <div className="section-heading">
                <h2>BibTeX Export</h2>
                <p>{selectedIds.length} papers selected.</p>
              </div>
              <div className="bibtex-toolbar">
                <label className="inline-select-label">
                  Journal
                  <select
                    value={bibtexJournalStyle}
                    onChange={(event) => setBibtexJournalStyle(event.currentTarget.value)}
                  >
                    <option value="as_stored">As stored</option>
                    <option value="full">Full name</option>
                    <option value="abbreviation">Abbreviation</option>
                  </select>
                </label>
                <button type="button" onClick={generateSelectedBibtex}>
                  Generate
                </button>
                <button type="button" className="text-button" onClick={copyBibtex}>
                  Copy
                </button>
                <button type="button" className="text-button" onClick={saveBibtexFile}>
                  Save .bib
                </button>
              </div>
              <textarea
                readOnly
                value={bibtexOutput}
                placeholder="Selected papers will be exported here."
              />
            </section>

            <section className="panel paper-list">
              <div className="section-heading">
                <h2>Paper card</h2>
                <p>{focusedPaper ? focusedPaper.title : "Select a paper from the list."}</p>
              </div>

              {!focusedPaper ? (
                <div className="empty-state">Select a paper from the list.</div>
              ) : (
                <article className="paper-item focused-paper-card">
                  <div className="paper-item-header">
                    <button
                      type="button"
                      className="text-button"
                      onClick={() => openEditPaperWindow(focusedPaper.id)}
                    >
                      Edit
                    </button>
                    <button
                      type="button"
                      className="danger-action"
                      onClick={() => deletePapers([focusedPaper.id])}
                    >
                      Delete
                    </button>
                  </div>
                  <div className="paper-title-row">
                    <h3>{focusedPaper.title}</h3>
                    <button
                      type="button"
                      className="pdf-open-button"
                      disabled={!focusedPaper.pdf_path}
                      onClick={() => openPaperPdf(focusedPaper.id)}
                    >
                      Open PDF
                    </button>
                  </div>
                  <p>{focusedPaper.authors.join(", ")}</p>
                  <div className="meta-row">
                    <span className={`status-badge status-${focusedPaper.status ?? "unread"}`}>
                      {focusedPaper.status ?? "unread"}
                    </span>
                  </div>
                  <div className="tag-row">
                    {focusedPaper.tags.length > 0 ? (
                      focusedPaper.tags.map((tag) => <span key={tag}>{tag}</span>)
                    ) : (
                      <span className="empty-tag">No tags</span>
                    )}
                  </div>
                  <div className="card-notes">
                    {(data?.notes ?? []).filter((note) => note.paper_id === focusedPaper.id)
                      .length === 0 ? (
                      <span className="card-note-empty">No notes</span>
                    ) : (
                      (data?.notes ?? [])
                        .filter((note) => note.paper_id === focusedPaper.id)
                        .map((note) => (
                          <button
                            type="button"
                            className="note-open-button"
                            key={note.id}
                            onClick={() => openPaperNote(note.id)}
                          >
                            Open: {note.title}
                          </button>
                        ))
                    )}
                  </div>
                  <dl>
                    <dt>Year</dt>
                    <dd>{focusedPaper.year ?? "-"}</dd>
                    <dt>Status</dt>
                    <dd>{focusedPaper.status ?? "-"}</dd>
                    <dt>Category</dt>
                    <dd>{focusedPaper.folder_category ?? "-"}</dd>
                    <dt>DOI</dt>
                    <dd>{focusedPaper.doi ?? "-"}</dd>
                    <dt>PDF</dt>
                    <dd>{focusedPaper.pdf_path ?? "Metadata only"}</dd>
                  </dl>
                </article>
              )}
            </section>
          </div>
        </section>
      </section>
    </main>
  );
}

export default App;
