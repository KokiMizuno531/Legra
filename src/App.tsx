import { useEffect, useMemo, useState } from "react";
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
    cloud_sync_expected: boolean;
    created_at: string;
    updated_at: string;
  };
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

type Filters = {
  query: string;
  tag: string;
  category: string;
  status: string;
  year: string;
  sort: string;
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

function isReadyToRegister(form: PaperForm) {
  return form.title.trim().length > 0 && parseList(form.authors).length > 0;
}

function metadataToForm(current: PaperForm, metadata: PaperMetadata): PaperForm {
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
    doi: metadata.doi ?? current.doi,
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

function App() {
  const currentWindowLabel = getCurrentWebviewWindow().label;
  const isRegisterWindow = currentWindowLabel === "register-paper";
  const editWindowPaperId = currentWindowLabel.startsWith("edit-paper-")
    ? currentWindowLabel.slice("edit-paper-".length)
    : null;
  const isEditWindow = editWindowPaperId !== null;
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [data, setData] = useState<AppData | null>(null);
  const [form, setForm] = useState<PaperForm>(initialForm);
  const [editForm, setEditForm] = useState<PaperForm>(initialForm);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [focusedPaperId, setFocusedPaperId] = useState<string | null>(null);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [filters, setFilters] = useState<Filters>(initialFilters);
  const [bibtexOutput, setBibtexOutput] = useState("");
  const [metadataLoading, setMetadataLoading] = useState(false);
  const [message, setMessage] = useState("Loading saved papers...");
  const [error, setError] = useState<string | null>(null);

  const allPapers = data?.papers ?? [];
  const editingPaper = allPapers.find((paper) => paper.id === editingId) ?? null;
  const focusedPaper = allPapers.find((paper) => paper.id === focusedPaperId) ?? null;

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
            .filter((category): category is string => Boolean(category)),
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
        const matchesCategory =
          filters.category === "all" ||
          (filters.category === "none" && !paper.folder_category) ||
          paper.folder_category === filters.category;
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
    const [nextStatus, nextData] = await Promise.all([
      invoke<AppStatus>("get_app_status"),
      invoke<AppData>("load_app_data"),
    ]);
    setStatus(nextStatus);
    setData(nextData);
    setMessage(`Loaded ${nextData.papers.length} papers.`);
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

    const managedDirectory = data?.settings.managed_directory;
    if (!managedDirectory) {
      setError("Set managed directory before selecting a category.");
      return;
    }

    const normalizedManaged = managedDirectory.replace(/\/+$/, "");
    const normalizedSelected = selected.replace(/\/+$/, "");
    if (normalizedSelected === normalizedManaged) {
      if (target === "register") {
        updateForm("folder_category", "");
      } else {
        updateEditForm("folder_category", "");
      }
      return;
    }

    const prefix = `${normalizedManaged}/`;
    if (!normalizedSelected.startsWith(prefix)) {
      setError("Select a category folder inside the managed directory.");
      return;
    }

    const category = normalizedSelected.slice(prefix.length);
    if (target === "register") {
      updateForm("folder_category", category);
    } else {
      updateEditForm("folder_category", category);
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

  async function registerPaper(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    if (!isReadyToRegister(form)) {
      setError("Title and at least one author are required.");
      setMessage("Registration failed.");
      return;
    }

    try {
      const nextData = await invoke<AppData>("register_paper", {
        input: formToInput(form),
      });
      setData(nextData);
      await refreshStatus();
      setForm(initialForm);
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

  async function generateSelectedBibtex() {
    setError(null);
    try {
      const content = await invoke<string>("generate_bibtex", {
        input: { paper_ids: selectedIds },
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
    if (isRegisterWindow || isEditWindow) {
      return;
    }

    let unlisten = () => {};
    listen("paper-manager:data-updated", () => {
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
  }, []);

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
    if (!focusedPaperId) {
      return;
    }

    if (!filteredPapers.some((paper) => paper.id === focusedPaperId)) {
      setFocusedPaperId(null);
    }
  }, [filteredPapers, focusedPaperId]);

  return isEditWindow ? (
    <main className="app-shell">
      <section className="workspace register-workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">Edit paper window</p>
            <h1>paper-manager</h1>
          </div>
          <div className="status-pill">
            {editingPaper ? "Editing paper" : "No paper selected"}
          </div>
        </header>

        <section className="panel bridge-panel">
          <div className="storage-info">
            <h2>Storage</h2>
            <p className="path-text">Data: {status?.data_file ? status.data_file.split('/').pop() : "..."}</p>
            <p className="path-text">
              Managed: {data?.settings.managed_directory ? data.settings.managed_directory.split('/').pop() : "Not set"}
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
            <h1>paper-manager</h1>
          </div>
          <div className="status-pill">
            {status?.data_file_exists ? `${allPapers.length} papers` : "No saved data"}
          </div>
        </header>

        <section className="panel bridge-panel">
          <div className="storage-info">
            <h2>Storage</h2>
            <p className="path-text">Data: {status?.data_file ? status.data_file.split('/').pop() : "..."}</p>
            <p className="path-text">
              Managed: {data?.settings.managed_directory ? data.settings.managed_directory.split('/').pop() : "Not set"}
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
            <p>Fetch metadata when needed, then register and organize the paper in one step.</p>
          </div>

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

          <div className="form-action-row">
            <button
              type="button"
              className="secondary-action"
              disabled={metadataLoading || !metadataKey(form)}
              onClick={fetchMetadataForRegistration}
            >
              {metadataLoading ? "Fetching..." : "Fetch metadata"}
            </button>
            <button type="submit" className="primary-action">
              Register paper
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

        </form>
      </section>
    </main>
  ) : (
    <main className="app-shell">
      <section className="workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">Phase 3 list search and detail editing</p>
            <h1>paper-manager</h1>
          </div>
          <div className="status-pill">
            {status?.data_file_exists ? `${allPapers.length} papers` : "No saved data"}
          </div>
        </header>

        <section className="panel bridge-panel">
          <div className="storage-info">
            <h2>Storage</h2>
            <p className="path-text">Data: {status?.data_file ? status.data_file.split('/').pop() : "..."}</p>
            <p className="path-text">
              Managed: {data?.settings.managed_directory ? data.settings.managed_directory.split('/').pop() : "Not set"}
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
            <button type="button" className="secondary-action" onClick={createBackup}>
              Backup
            </button>
            <button type="button" className="secondary-action" onClick={restoreBackup}>
              Restore
            </button>
            <button type="button" className="secondary-action" onClick={relinkFiles}>
              Relink
            </button>
            <button type="button" className="secondary-action" onClick={loadSavedData}>
              Reload
            </button>
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
