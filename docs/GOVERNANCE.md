# Governance

CalcDex is a single-maintainer project.

- **Decisions.** The maintainer (see
  [MAINTAINERS.md](MAINTAINERS.md)) decides scope,
  architecture and releases. Design rationale is recorded
  in [../DECISIONS.md](../DECISIONS.md); the frozen
  foundation and coverage floors are in
  [../PROJECT_RULES.md](../PROJECT_RULES.md).
- **Contributions** are welcome under the rules in
  [CONTRIBUTING.md](CONTRIBUTING.md); acceptance is at the
  maintainer's discretion, with the project's invariants
  (one registry for window and command line, errors as
  text, plain-text data files) treated as non-negotiable.
- **Releases** are cut from tags by the maintainer after
  the CI gate passes; release contents are recorded in
  [CHANGELOG.md](CHANGELOG.md).
- **License.** MIT for all project code (see
  [../LICENSE](../LICENSE)); third-party components remain
  under their own licenses (see [CREDITS.md](CREDITS.md)).
