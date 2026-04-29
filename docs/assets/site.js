import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs";

const page = window.location.pathname.split("/").pop() || "index.html";

for (const link of document.querySelectorAll("[data-nav] a")) {
  const href = link.getAttribute("href");
  if (href === page || (page === "" && href === "index.html")) {
    link.setAttribute("aria-current", "page");
  }
}

const fallbackConfig = {
  startOnLoad: false,
  securityLevel: "loose",
  theme: "base",
  themeVariables: {
    fontFamily: "Segoe UI, system-ui, sans-serif",
    background: "transparent",
    primaryColor: "#ffffff",
    primaryTextColor: "#1e2527",
    primaryBorderColor: "#0f766e",
    secondaryColor: "#eef5f3",
    secondaryTextColor: "#1e2527",
    secondaryBorderColor: "#2563a6",
    tertiaryColor: "#fff7ed",
    tertiaryTextColor: "#1e2527",
    tertiaryBorderColor: "#a16207",
    lineColor: "#647176",
    clusterBkg: "#f6f7f4",
    clusterBorder: "#d8e0dc"
  },
  flowchart: {
    useMaxWidth: false,
    htmlLabels: true,
    curve: "linear",
    nodeSpacing: 42,
    rankSpacing: 62,
    padding: 18
  }
};

async function mermaidConfig() {
  try {
    const response = await fetch("diagrams/mermaid.config.json");
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return { ...fallbackConfig, ...(await response.json()), startOnLoad: false };
  } catch (error) {
    console.warn("Using fallback Mermaid config", error);
    return fallbackConfig;
  }
}

async function renderInlineDiagrams() {
  const targets = [...document.querySelectorAll("[data-mermaid-source]")];
  if (targets.length === 0) return;

  mermaid.initialize(await mermaidConfig());
  for (const target of targets) {
    const source = target.getAttribute("data-mermaid-source");
    try {
      const response = await fetch(source);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      target.textContent = await response.text();
      target.classList.add("mermaid");
    } catch (error) {
      target.outerHTML = `<p class="status-note">Unable to load diagram source <code>${source}</code>.</p>`;
      console.error(error);
    }
  }
  await mermaid.run({ querySelector: ".mermaid" });
}

renderInlineDiagrams().catch(console.error);
