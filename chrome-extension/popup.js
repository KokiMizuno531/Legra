let detected = null;

const statusEl = document.getElementById("status");
const titleEl = document.getElementById("title");
const doiEl = document.getElementById("doi");
const arxivEl = document.getElementById("arxiv");
const pdfEl = document.getElementById("pdf");
const categoryEl = document.getElementById("category");
const importButton = document.getElementById("import");

function setStatus(message) {
  statusEl.textContent = message;
}

function renderDetection(result) {
  detected = result;
  titleEl.textContent = result.title || "-";
  doiEl.textContent = result.doi || "-";
  arxivEl.textContent = result.arxiv_id || "-";
  pdfEl.textContent = result.pdf_url ? "available" : "-";
  importButton.disabled = !(result.doi || result.arxiv_id || result.pdf_url);
}

async function detectCurrentTab() {
  importButton.disabled = true;
  setStatus("Inspecting current tab...");
  try {
    const result = await chrome.runtime.sendMessage({ action: "detect_current_tab" });
    if (!result.ok) {
      throw new Error(result.message);
    }
    renderDetection(result.paper);
    setStatus("Ready to import.");
  } catch (error) {
    setStatus(String(error.message || error));
  }
}

async function importCurrentTab() {
  importButton.disabled = true;
  setStatus("Importing...");
  try {
    const result = await chrome.runtime.sendMessage({
      action: "import_current_tab",
      category: categoryEl.value.trim(),
      detected,
    });
    if (!result.ok) {
      throw new Error(result.message);
    }
    setStatus(result.message);
    window.setTimeout(() => window.close(), 250);
  } catch (error) {
    setStatus(String(error.message || error));
  } finally {
    importButton.disabled = false;
  }
}

importButton.addEventListener("click", importCurrentTab);
categoryEl.addEventListener("keydown", (event) => {
  if (event.key !== "Enter" || importButton.disabled) {
    return;
  }

  event.preventDefault();
  importCurrentTab();
});

detectCurrentTab();
