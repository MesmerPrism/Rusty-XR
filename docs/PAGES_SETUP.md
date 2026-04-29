# GitHub Pages Setup

Rusty XR's public documentation site is a static site under `docs/`.

Recommended GitHub Pages setting:

- Source: deploy from a branch
- Branch: `main`
- Folder: `/docs`

The site intentionally publishes public architecture, crate roles, workflow
guidance, and Mermaid diagrams only. Do not publish non-public planning notes,
raw graphify outputs, generated captures, package identities, signing data, or
downstream app-specific behavior.

Local preview options:

```powershell
python -m http.server 8000 -d docs
```

Then open `http://127.0.0.1:8000/`.
