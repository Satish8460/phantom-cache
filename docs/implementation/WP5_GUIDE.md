> Notice (2026-10-30): Historical work-package specification from the first (simulation-era) phase. "WP1" to "WP5" here are not the "WP-nn" work packages of `docs/research/work_packages.md`. The plan describes intended work, not verified current behaviour: for example, PIE canary rollouts and Count-Min sketch estimation are described here, but `src/control/pie.rs` is a legacy threshold stub (`pie: None` at `src/main.rs:433`) and `CountMinSketch` is used only by a unit test (`tests/unit/wp2_tests.rs`). Current status and evidence: [docs/STATUS.md](../STATUS.md), [handoff CORRECTIONS](../handoff/CORRECTIONS.md).
>
> Original text below is unchanged.

# WP5 Developer Guide — Packaging & Distribution

This guide details how to build and package the Phantom Cache flagship demo from a clean checkout.

---

## 1. Local Build Commands
To compile the Rust server binary and bundle React assets for production:

```bash
# Build Rust release binary
cargo build --release

# Build React production bundle
cd ui
npm install
npm run build
```

---

## 2. Packaging the Distributable
Pack the following directories into a `.zip` archive for distribution:
* `target/release/phantom-cache-demo.exe`
* `ui/dist/`
* `Start-Phantom-Demo.bat`
* `docs/demo/README.md`
