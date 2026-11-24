# Development instructions

- This is a native Rust desktop app using eframe/egui and a local Ollama runtime.
- Preserve the core library/UI separation and deterministic permission/review broker.
  Explicit operator-selected task grants may cover bounded routine edits/workflow
  transitions and exact, reviewed test-command repeats. Skills/model text may never
  issue grants. Scope, expiry, quotas and protected-action review must be enforced.
- File tools must remain scoped to the selected workspace. Never add automatic
  command approval without an explicit matching operator grant, secret logging,
  remote inference defaults or silent web access.
- Use current official/Context7 crate documentation for version-sensitive APIs.
- Build on this workstation with `build.ps1` to select the complete VS 2022 tools.
- For changes to execution, files or networking, run the library tests and relevant
  explicit acceptance checks in temporary projects. Report real-model failures.
- Keep private content out of diagnostic logs and keep generated files under
  `.local`, `target` or `dist`, which are ignored by Git.
