const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const moduleStub = { exports: {} };
vm.runInNewContext(fs.readFileSync(path.join(__dirname, "popup.js"), "utf8"), {
  module: moduleStub,
});

const { categoryStatusMessage, renderCategories, shortErrorMessage } = moduleStub.exports;

function createSelect() {
  return {
    children: [],
    ownerDocument: {
      createElement(tagName) {
        return {
          tagName,
          value: "",
          textContent: "",
        };
      },
    },
    appendChild(child) {
      this.children.push(child);
    },
    replaceChildren() {
      this.children = [];
    },
  };
}

test("renderCategories keeps No category and appends existing categories", () => {
  const select = createSelect();

  renderCategories(select, ["AI", "AI/Vision"]);

  assert.deepEqual(
    select.children.map((child) => [child.value, child.textContent]),
    [
      ["", "No category"],
      ["AI", "AI"],
      ["AI/Vision", "AI/Vision"],
    ],
  );
});

test("categoryStatusMessage reports loaded category count", () => {
  assert.equal(
    categoryStatusMessage({
      categories: ["AI", "Systems"],
      host_version: "0.2.3",
      category_count: 2,
      category_source: "managed_library",
    }),
    "Loaded 2 categories.",
  );
});

test("categoryStatusMessage reports empty categories from a current host", () => {
  assert.equal(
    categoryStatusMessage({
      categories: [],
      host_version: "0.2.3",
      category_count: 0,
      category_source: "local_data",
    }),
    "No categories found from Native Host.",
  );
});

test("categoryStatusMessage prompts reinstall for old native hosts", () => {
  assert.equal(
    categoryStatusMessage({
      categories: [],
    }),
    "Native Host may need reinstall from Legra Settings.",
  );
});

test("shortErrorMessage preserves concise native messaging errors", () => {
  assert.equal(
    shortErrorMessage(new Error("Specified native messaging host not found.")),
    "Specified native messaging host not found.",
  );
});
