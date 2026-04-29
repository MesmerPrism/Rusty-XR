import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs";

const nav = document.querySelector("#diagram-nav");
const title = document.querySelector("#diagram-title");
const description = document.querySelector("#diagram-description");
const stage = document.querySelector("#diagram-stage");
const sourcePanel = document.querySelector("#source-panel");
const sourceCode = document.querySelector("#source-code");
const sourceLink = document.querySelector("#source-link");
const toggleSource = document.querySelector("#toggle-source");

const configResponse = await fetch("mermaid.config.json");
const config = configResponse.ok ? await configResponse.json() : {};
mermaid.initialize({ startOnLoad: false, securityLevel: "loose", ...config });

const manifestResponse = await fetch("manifest.json");
const diagrams = manifestResponse.ok ? await manifestResponse.json() : [];

let current = null;

function setActive(id) {
  for (const button of nav.querySelectorAll("button")) {
    button.classList.toggle("active", button.dataset.id === id);
  }
}

async function loadDiagram(item) {
  current = item;
  setActive(item.id);
  title.textContent = item.title;
  description.textContent = item.description;
  sourceLink.href = item.source;
  sourceLink.textContent = item.source;
  const response = await fetch(item.source);
  const source = await response.text();
  sourceCode.textContent = source;
  sourcePanel.hidden = true;
  stage.hidden = false;
  stage.innerHTML = "";
  const { svg } = await mermaid.render(`diagram-${item.id}`, source);
  stage.innerHTML = svg;
  window.location.hash = item.id;
}

for (const item of diagrams) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "nav-item";
  button.dataset.id = item.id;
  button.textContent = item.title;
  button.addEventListener("click", () => loadDiagram(item));
  nav.append(button);
}

toggleSource.addEventListener("click", () => {
  if (!current) return;
  const showingSource = !sourcePanel.hidden;
  sourcePanel.hidden = showingSource;
  stage.hidden = !showingSource;
  toggleSource.textContent = showingSource ? "Source" : "Diagram";
});

const initialId = window.location.hash.slice(1);
const initial = diagrams.find((item) => item.id === initialId) ?? diagrams[0];
if (initial) {
  await loadDiagram(initial);
}
