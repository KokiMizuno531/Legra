function shortErrorMessage(error) {
  const message = String(error?.message || error || "Unknown error").trim();
  return message.length > 140 ? `${message.slice(0, 137)}...` : message;
}

function hasCategoryDiagnostics(result) {
  return (
    typeof result?.host_version === "string" ||
    typeof result?.category_count === "number" ||
    typeof result?.category_source === "string" ||
    typeof result?.managed_directory === "string"
  );
}

function categoryStatusMessage(result) {
  const categories = Array.isArray(result?.categories) ? result.categories : [];
  if (categories.length > 0) {
    return `Loaded ${categories.length} categories.`;
  }
  if (!hasCategoryDiagnostics(result)) {
    return "Native Host may need reinstall from Legra Settings.";
  }
  return "No categories found from Native Host.";
}

function renderCategories(categorySelectEl, categories) {
  categorySelectEl.replaceChildren();

  const ownerDocument = categorySelectEl.ownerDocument;
  const emptyOption = ownerDocument.createElement("option");
  emptyOption.value = "";
  emptyOption.textContent = "No category";
  categorySelectEl.appendChild(emptyOption);

  for (const category of categories) {
    const option = ownerDocument.createElement("option");
    option.value = category;
    option.textContent = category;
    categorySelectEl.appendChild(option);
  }
}

function initializePopup({ document, chrome, window }) {
  let detected = null;
  let categoryStatus = "";

  const statusEl = document.getElementById("status");
  const titleEl = document.getElementById("title");
  const doiEl = document.getElementById("doi");
  const arxivEl = document.getElementById("arxiv");
  const pdfEl = document.getElementById("pdf");
  const categorySelectEl = document.getElementById("categorySelect");
  const categoryNewEl = document.getElementById("categoryNew");
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

  async function loadCategories() {
    try {
      const result = await chrome.runtime.sendMessage({ action: "list_categories" });
      if (!result.ok) {
        throw new Error(result.message);
      }
      renderCategories(categorySelectEl, result.categories || []);
      categoryStatus = categoryStatusMessage(result);
      setStatus(categoryStatus);
    } catch (error) {
      categoryStatus = `Could not load categories: ${shortErrorMessage(error)}`;
      setStatus(categoryStatus);
    }
  }

  function selectedCategory() {
    return categoryNewEl.value.trim() || categorySelectEl.value.trim();
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
      setStatus(categoryStatus || "Ready to import.");
    } catch (error) {
      setStatus(shortErrorMessage(error));
    }
  }

  async function importCurrentTab() {
    importButton.disabled = true;
    setStatus("Importing...");
    try {
      const result = await chrome.runtime.sendMessage({
        action: "import_current_tab",
        category: selectedCategory(),
        detected,
      });
      if (!result.ok) {
        throw new Error(result.message);
      }
      setStatus(result.message);
      window.setTimeout(() => window.close(), 250);
    } catch (error) {
      setStatus(shortErrorMessage(error));
    } finally {
      importButton.disabled = false;
    }
  }

  importButton.addEventListener("click", importCurrentTab);
  categorySelectEl.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" || importButton.disabled) {
      return;
    }

    event.preventDefault();
    importCurrentTab();
  });
  categoryNewEl.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" || importButton.disabled) {
      return;
    }

    event.preventDefault();
    importCurrentTab();
  });

  loadCategories();
  detectCurrentTab();

  return {
    loadCategories,
    detectCurrentTab,
    importCurrentTab,
  };
}

if (typeof module !== "undefined") {
  module.exports = {
    categoryStatusMessage,
    hasCategoryDiagnostics,
    renderCategories,
    shortErrorMessage,
  };
}

if (typeof document !== "undefined" && typeof chrome !== "undefined") {
  initializePopup({ document, chrome, window });
}
