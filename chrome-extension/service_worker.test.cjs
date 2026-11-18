const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const serviceWorkerSource = fs.readFileSync(
  path.join(__dirname, "service_worker.js"),
  "utf8",
);

function detectPage({ url, meta = {}, authors = [], links = [], title = "", bodyText = "" }) {
  const moduleStub = { exports: {} };
  const document = {
    title,
    body: { innerText: bodyText },
    querySelector(selector) {
      const name = selector.match(/meta\[name="([^"]+)"\]/)?.[1];
      return name && Object.hasOwn(meta, name) ? { content: meta[name] } : null;
    },
    querySelectorAll(selector) {
      if (selector === 'meta[name="citation_author"]') {
        return authors.map((content) => ({ content }));
      }
      if (selector === "a[href]") {
        return links.map((link) => typeof link === "string" ? { href: link } : link);
      }
      return [];
    },
  };

  vm.runInNewContext(serviceWorkerSource, {
    document,
    location: { href: url },
    module: moduleStub,
  });
  return moduleStub.exports.pageDetector();
}

test("publisher references are not mistaken for the current paper's arXiv ID", () => {
  const paper = detectPage({
    url: "https://www.nature.com/articles/s41567-026-03337-w",
    meta: {
      citation_doi: "10.1038/s41567-026-03337-w",
      citation_pdf_url: "https://www.nature.com/articles/s41567-026-03337-w.pdf",
      citation_title: "Altermagnetic spintronics",
    },
    links: [
      "https://arxiv.org/abs/2309.02355",
      "https://arxiv.org/abs/2411.19928",
    ],
  });

  assert.equal(paper.doi, "10.1038/s41567-026-03337-w");
  assert.equal(paper.arxiv_id, "");
  assert.equal(paper.arxiv_source, "");
  assert.equal(
    paper.pdf_url,
    "https://www.nature.com/articles/s41567-026-03337-w.pdf",
  );
});

test("an explicit arXiv metadata field remains supported", () => {
  const paper = detectPage({
    url: "https://publisher.example/paper",
    meta: { citation_arxiv_id: "arXiv:2501.01234v2" },
  });

  assert.equal(paper.arxiv_id, "2501.01234");
  assert.equal(paper.arxiv_source, "metadata");
});

test("the current arXiv page URL remains authoritative", () => {
  const paper = detectPage({
    url: "https://arxiv.org/abs/2507.02793v3",
    links: ["https://arxiv.org/abs/1111.11111"],
  });

  assert.equal(paper.arxiv_id, "2507.02793");
  assert.equal(paper.arxiv_source, "page_url");
});

function startWorker({ tabUrl, detectedPage, mime = "application/pdf" }) {
  let listener;
  const requests = [];
  const downloads = [];
  const removed = [];
  const filename = "/downloads/paper-manager-import/paper.pdf";
  const chrome = {
    tabs: { query: async () => [{ id: 1, url: tabUrl }] },
    scripting: {
      executeScript: async () => {
        if (!detectedPage) throw new Error("PDF viewer blocks scripts");
        return [{ result: detectedPage }];
      },
    },
    downloads: {
      download: async (options) => {
        downloads.push(options);
        return 42;
      },
      search: async () => [{ id: 42, filename, mime, state: "complete" }],
      onChanged: { addListener() {}, removeListener() {} },
      removeFile: async (id) => removed.push(id),
      erase: async () => {},
    },
    runtime: {
      onMessage: { addListener: (handler) => { listener = handler; } },
      sendNativeMessage(_host, payload, reply) {
        requests.push(payload.request);
        reply({ ok: true });
      },
    },
  };
  vm.runInNewContext(serviceWorkerSource, { chrome, URL });
  return {
    send: (message) => new Promise((resolve) => listener(message, {}, resolve)),
    requests,
    downloads,
    removed,
    filename,
  };
}

test("APS article pages download the article PDF and queue the chosen category", async () => {
  const doi = "10.1103/PhysRevX.14.040501";
  const tabUrl = `https://journals.aps.org/prx/abstract/${doi}?source=test`;
  const detectedPage = detectPage({
    url: tabUrl,
    meta: { citation_doi: doi },
    links: ["https://journals.aps.org/supplement/figures.pdf"],
  });
  const worker = startWorker({ tabUrl, detectedPage });

  const result = await worker.send({
    action: "import_current_tab",
    category: "Physics/Scattering",
  });

  assert.equal(result.ok, true);
  assert.equal(worker.downloads[0].url, `https://journals.aps.org/prx/pdf/${doi}`);
  assert.equal(worker.requests[0].doi, doi);
  assert.equal(worker.requests[0].pdf_path, worker.filename);
  assert.equal(worker.requests[0].suggested_category, "Physics/Scattering");
});

test("the APS PDF viewer URL is detected when the page script cannot run", async () => {
  const tabUrl = "https://link.aps.org/pdf/10.1103/PhysRevLett.93.135503";
  const worker = startWorker({ tabUrl });

  const result = await worker.send({ action: "detect_current_tab" });

  assert.equal(result.ok, true);
  assert.equal(result.paper.pdf_url, tabUrl);
});

test("an HTML response with a .pdf filename is discarded before queueing", async () => {
  const worker = startWorker({
    tabUrl: "https://journals.aps.org/prb/abstract/10.1103/PhysRevB.103.184423",
    mime: "text/html",
  });

  const result = await worker.send({ action: "import_current_tab" });

  assert.equal(result.ok, true);
  assert.deepEqual(worker.removed, [42]);
  assert.equal(worker.requests[0].pdf_path, undefined);
  assert.match(worker.requests[0].import_warnings[0], /not a PDF/);
});

test("JPSJ selects the PDF Download URL from the article and eReader pages", async () => {
  const doi = "10.7566/JPSJ.94.064705";
  const base = "https://journals.jps.jp/doi";
  for (const tabUrl of [`${base}/${doi}`, `${base}/epdf/${doi}`]) {
    const worker = startWorker({ tabUrl });
    const result = await worker.send({
      action: "import_current_tab",
      category: "Physics/JPSJ",
    });

    assert.equal(result.ok, true);
    assert.equal(worker.downloads[0].url, `${base}/pdf/${doi}?download=true`);
    assert.equal(worker.requests[0].doi, doi);
    assert.equal(worker.requests[0].pdf_path, worker.filename);
    assert.equal(worker.requests[0].suggested_category, "Physics/JPSJ");
  }
});

test("an article PDF link with a matching DOI takes precedence over other PDFs", () => {
  const doi = "10.1088/1367-2630/add7fd";
  const articleUrl = `https://iopscience.iop.org/article/${doi}`;
  const paper = detectPage({
    url: articleUrl,
    links: [
      { href: "https://iopscience.iop.org/supplement.pdf", textContent: "Supplement" },
      `${articleUrl}/pdf`,
    ],
  });
  assert.equal(paper.doi, doi);
  assert.equal(paper.pdf_url, `${articleUrl}/pdf`);
});

test("IOP and AIP PDF viewers keep the article DOI separate from the PDF path", async () => {
  const articles = [
    ["10.1088/1367-2630/add7fd", "https://iopscience.iop.org/article/10.1088/1367-2630/add7fd/pdf"],
    ["10.1063/5.0281124", "https://pubs.aip.org/aip/jap/article-pdf/doi/10.1063/5.0281124/20700266/113303_1_5.0281124.pdf"],
  ];
  for (const [doi, tabUrl] of articles) {
    const worker = startWorker({ tabUrl });
    const result = await worker.send({ action: "detect_current_tab" });
    assert.equal(result.paper.doi, doi);
    assert.equal(result.paper.pdf_url, tabUrl);
  }
});

test("a publisher's relative PDF metadata resolves against the article page", async () => {
  const tabUrl = "https://www.nature.com/articles/s41467-024-45951-3";
  const detectedPage = detectPage({
    url: tabUrl,
    meta: { citation_pdf_url: "/articles/s41467-024-45951-3.pdf" },
  });
  const worker = startWorker({ tabUrl, detectedPage });

  const result = await worker.send({ action: "detect_current_tab" });

  assert.equal(result.paper.pdf_url, `${tabUrl}.pdf`);
});

test("a supplemental PDF by itself is not selected for an identified article", () => {
  const paper = detectPage({
    url: "https://example.org/article",
    meta: { citation_doi: "10.1234/article" },
    links: ["https://example.org/appendix.pdf"],
  });
  assert.equal(paper.pdf_url, "");
});
