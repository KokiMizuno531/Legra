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
        return links.map((href) => ({ href }));
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
