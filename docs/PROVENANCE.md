# Provenance Metadata

Rusty XR tracks lightweight public provenance for extracted utilities. The goal
is an audit trail that explains why a utility belongs in the public repo without
leaking private project context.

Public provenance lives in:

- `provenance/public-utilities.toml`

Detailed local evidence, exact private repo paths, and private implementation
notes stay outside the public repository.

## Required Fields

Each utility entry should record:

- `crate`: public crate or tool name.
- `source_family`: broad source category, not an exact private project path.
- `extraction_date`: date the public utility was added or reviewed.
- `public_safe_rationale`: why the concept is reusable and safe for public use.
- `license_assumptions`: licensing/provenance assumptions for the public shape.
- `scrubbed_private_dependencies`: private behavior intentionally left out.
- `review_status`: current review state.

## Boundary Rule

Use broad source categories such as `local Quest diagnostics`, `public Polar
references`, or `Makepad-style UI concepts`. Do not publish private package
identity, generated captures, local filesystem paths, signing details, or
private project-specific behavior in provenance metadata.
