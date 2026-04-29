# Git Hooks

This folder contains versioned hook wrappers for local development.

Configure this repo to use them with:

```powershell
git config core.hooksPath tools/git-hooks
```

The `pre-push` hook runs the public boundary scanner. If the environment
variable `RUSTY_XR_PRIVATE_BOUNDARY_CONFIG` points at an additional private JSON
config, the hook runs that too without committing the private term list to the
public repo.
