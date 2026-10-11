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
