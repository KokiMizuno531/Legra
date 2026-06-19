const NATIVE_HOST = "app.legra.importer";

function normalizeDoi(value) {
  if (!value) return "";
  return value
    .trim()
    .replace(/^https?:\/\/(dx\.)?doi\.org\//i, "")
    .replace(/^doi:/i, "")
    .replace(/[?#].*$/, "")
    .replace(/\.$/, "");
}

function normalizeArxiv(value) {
  if (!value) return "";
  return value
    .trim()
    .replace(/^arxiv:/i, "")
    .replace(/^https?:\/\/(www\.)?arxiv\.org\/(abs|pdf)\//i, "")
    .replace(/[?#].*$/, "")
    .replace(/\.pdf$/i, "")
    .replace(/v\d+$/i, "");
}

function extractDoiFromText(text) {
  const match = String(text || "").match(/\b10\.\d{4,9}\/[-._;()/:A-Z0-9]+/i);
  return match ? normalizeDoi(match[0]) : "";
}

function extractArxivFromUrl(url) {
  if (!/arxiv\.org\/(abs|pdf)\//i.test(url || "")) return "";
  return normalizeArxiv(url);
}

function safeFilenamePart(value) {
  return String(value || "paper")
    .replace(/[^a-z0-9_-]+/gi, "_")
    .replace(/^_+|_+$/g, "")
    .slice(0, 120) || "paper";
}

async function activeTab() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (!tab?.id || !tab.url) {
    throw new Error("No active tab was found.");
  }
  return tab;
}

function pageDetector() {
  const normalizeDoiInPage = (value) => {
    if (!value) return "";
    return value
      .trim()
      .replace(/^https?:\/\/(dx\.)?doi\.org\//i, "")
      .replace(/^doi:/i, "")
      .replace(/[?#].*$/, "")
      .replace(/\.$/, "");
  };
  const normalizeArxivInPage = (value) => {
    if (!value) return "";
    return value
      .trim()
      .replace(/^arxiv:/i, "")
      .replace(/^https?:\/\/(www\.)?arxiv\.org\/(abs|pdf)\//i, "")
      .replace(/[?#].*$/, "")
      .replace(/\.pdf$/i, "")
      .replace(/v\d+$/i, "");
  };
  const extractDoiInPage = (text) => {
    const match = String(text || "").match(/\b10\.\d{4,9}\/[-._;()/:A-Z0-9]+/i);
    return match ? normalizeDoiInPage(match[0]) : "";
  };
  const extractArxivInPage = (url) => {
    if (!/arxiv\.org\/(abs|pdf)\//i.test(url || "")) return "";
    return normalizeArxivInPage(url);
  };
  const meta = (name) =>
    document.querySelector(`meta[name="${name}"], meta[property="${name}"]`)?.content?.trim() || "";
  const anchors = Array.from(document.querySelectorAll("a[href]"));
  const hrefs = anchors.map((anchor) => anchor.href).filter(Boolean);
  const title =
    meta("citation_title") ||
    meta("dc.title") ||
    meta("DC.Title") ||
    document.title ||
    "";
  const authors = Array.from(document.querySelectorAll('meta[name="citation_author"]'))
    .map((node) => node.content?.trim())
    .filter(Boolean);
  const yearValue =
    meta("citation_publication_date") ||
    meta("citation_online_date") ||
    meta("dc.date") ||
    meta("DC.Date") ||
    "";
  const year = Number.parseInt(yearValue.slice(0, 4), 10);
  const doi =
    meta("citation_doi") ||
    meta("dc.identifier") ||
    meta("DC.Identifier") ||
    extractDoiInPage(document.body?.innerText || "") ||
    hrefs.map(extractDoiInPage).find(Boolean) ||
    "";
  const arxiv_id =
    extractArxivInPage(location.href) ||
    hrefs.map(extractArxivInPage).find(Boolean) ||
    "";
  const pdf_url =
    meta("citation_pdf_url") ||
    hrefs.find((href) => /\.pdf($|[?#])/i.test(href)) ||
    (location.href.toLowerCase().includes(".pdf") ? location.href : "");

  return {
    title,
    authors,
    year: Number.isFinite(year) ? year : undefined,
    publication: meta("citation_journal_title") || meta("dc.source") || "",
    doi: normalizeDoiInPage(doi),
    arxiv_id: normalizeArxivInPage(arxiv_id),
    pdf_url,
    source_url: location.href,
  };
}

async function detectCurrentTab() {
  const tab = await activeTab();
  let paper = {
    title: tab.title || "",
    authors: [],
    doi: extractDoiFromText(tab.url),
    arxiv_id: extractArxivFromUrl(tab.url),
    pdf_url: /\.pdf($|[?#])/i.test(tab.url) ? tab.url : "",
    source_url: tab.url,
  };

  try {
    const [result] = await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: pageDetector,
    });
    paper = { ...paper, ...(result?.result || {}) };
  } catch (_) {
    // Chrome's PDF viewer and restricted pages may reject script injection.
  }

  if (paper.arxiv_id && !paper.pdf_url) {
    paper.pdf_url = `https://arxiv.org/pdf/${paper.arxiv_id}.pdf`;
  }

  return paper;
}

function waitForDownload(downloadId) {
  return new Promise((resolve, reject) => {
    const listener = async (delta) => {
      if (delta.id !== downloadId || !delta.state?.current) return;
      if (delta.state.current === "complete") {
        chrome.downloads.onChanged.removeListener(listener);
        const [item] = await chrome.downloads.search({ id: downloadId });
        const filename = item?.filename || "";
        if (!filename.toLowerCase().endsWith(".pdf")) {
          await cleanupDownload(downloadId);
          reject(new Error(`Downloaded file was not a PDF: ${filename || "unknown file"}`));
          return;
        }
        resolve({ downloadId, filename });
      }
      if (delta.state.current === "interrupted") {
        chrome.downloads.onChanged.removeListener(listener);
        await cleanupDownload(downloadId);
        reject(new Error("PDF download was interrupted."));
      }
    };
    chrome.downloads.onChanged.addListener(listener);
  });
}

async function cleanupDownload(downloadId) {
  try {
    await chrome.downloads.removeFile(downloadId);
  } catch (_) {
    // The file may already be gone or unavailable.
  }
  try {
    await chrome.downloads.erase({ id: downloadId });
  } catch (_) {
    // History cleanup is best-effort.
  }
}

async function downloadPdf(paper) {
  if (!paper.pdf_url) return null;
  const base = safeFilenamePart(paper.doi || paper.arxiv_id || paper.title || "paper");
  const downloadId = await chrome.downloads.download({
    url: paper.pdf_url,
    filename: `paper-manager-import/${base}.pdf`,
    conflictAction: "uniquify",
    saveAs: false,
  });
  return waitForDownload(downloadId);
}

function sendNativeImport(request) {
  return new Promise((resolve, reject) => {
    chrome.runtime.sendNativeMessage(
      NATIVE_HOST,
      { action: "import_paper", request },
      (response) => {
        if (chrome.runtime.lastError) {
          reject(new Error(chrome.runtime.lastError.message));
          return;
        }
        if (!response?.ok) {
          reject(new Error(response?.message || "Native host rejected the import."));
          return;
        }
        resolve(response);
      },
    );
  });
}

function sendNativeListCategories() {
  return new Promise((resolve, reject) => {
    chrome.runtime.sendNativeMessage(NATIVE_HOST, { action: "list_categories" }, (response) => {
      if (chrome.runtime.lastError) {
        reject(new Error(chrome.runtime.lastError.message));
        return;
      }
      if (!response?.ok) {
        reject(new Error(response?.message || "Native host rejected the category request."));
        return;
      }
      resolve(response);
    });
  });
}

async function importCurrentTab(message) {
  const paper = message.detected || (await detectCurrentTab());
  let pdfPath = "";
  let pdfDownloadId = null;
  const importWarnings = [];
  try {
    const download = await downloadPdf(paper);
    pdfPath = download?.filename || "";
    pdfDownloadId = download?.downloadId ?? null;
  } catch (error) {
    importWarnings.push(`PDF download failed: ${String(error.message || error)}`);
  }
  const request = {
    id: `chrome-import-${Date.now()}`,
    source_url: paper.source_url,
    doi: paper.doi || undefined,
    arxiv_id: paper.arxiv_id || undefined,
    title: paper.title || undefined,
    authors: paper.authors?.length ? paper.authors : undefined,
    year: paper.year,
    publication: paper.publication || undefined,
    pdf_path: pdfPath || undefined,
    suggested_category: message.category || undefined,
    tags: ["chrome-import"],
    import_warnings: importWarnings,
  };
  try {
    await sendNativeImport(request);
  } catch (error) {
    if (pdfDownloadId !== null) {
      await cleanupDownload(pdfDownloadId);
    }
    throw error;
  }
  return {
    ok: true,
    message: pdfPath
      ? "Queued import with downloaded PDF. Open Legra or click Import inbox."
      : importWarnings.length > 0
        ? "PDF download failed, but metadata import was queued."
        : "Queued metadata import. Open Legra or click Import inbox.",
  };
}

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  const run =
    message.action === "detect_current_tab"
      ? detectCurrentTab().then((paper) => ({ ok: true, paper }))
      : message.action === "list_categories"
        ? sendNativeListCategories()
      : message.action === "import_current_tab"
        ? importCurrentTab(message)
        : Promise.resolve({ ok: false, message: "Unknown action." });

  run.then(sendResponse).catch((error) =>
    sendResponse({ ok: false, message: String(error.message || error) }),
  );
  return true;
});
