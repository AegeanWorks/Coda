# Coda

[![Built with AIKit](https://img.shields.io/badge/built%20with-AIKit-6f42c1)](https://github.com/AegeanWorks/AIKit)

Coda is a deterministic, dependency-free Rust sound compiler that turns a small textual score into reproducible stereo WAV audio, with seeded arrangement styles for locally generated music.

## ⚡ 30-Second Cold-Start Quick Reference

- **Runtime Prerequisites**: <FILL: each runtime this project needs and its minimum version>
- **Canonical Regression Shield Gate**:
  ```bash
  cargo test
  ```

### 🛠️ AIKit Commands (run from the AIKit folder)
| Role | Command |
|---|---|
| Status, task and gates | `python aikit.py status coda` |
| Phase 4: every gate | `python aikit.py check coda` |
| Repair memory and map, then check | `python aikit.py fix coda` |
| Resume an interrupted task | `python aikit.py resume coda` |

## 📦 What's Inside

```text
coda/
├── .gitattributes            <- LF line endings on every platform
├── .gitignore                <- transient AIKit state excluded
├── LICENSE                   <- Aegean License 1.1 (AL-1.1): free personal use
├── COMMERCIAL.md             <- Commercial licensing terms
├── LICENSES/                 <- REUSE license texts
├── REUSE.toml                <- Machine-readable SPDX metadata
├── README.md
├── inventory.py              <- AI-compat inventory: kind|name|locator lines (ADR-315)
├── demo.py                   <- AI-compat scripted drive: WORLD-DELTA lines (ADR-315)
└── .aikit/                   <- this project's memory: record, ledger, decisions
```

## 📜 License

Free for personal use under the Aegean License 1.1 (AL-1.1); see [LICENSE](LICENSE). Any commercial use needs the Aegean Commercial License 1.0 (ACL-1.0); see [COMMERCIAL.md](COMMERCIAL.md).
