# Phantom Cache Flagship Demo Guide

This guide details how to launch and run the Phantom Cache flagship product demonstration.

---

## 1. Prerequisites
Ensure the following binaries are available in your system path:
* **Node.js** (v18.0+) & **npm**
* **Rust / Cargo** (for building the simulation binary)

---

## 2. Quick Start Launch
1. Open the project root directory.
2. Double-click the file:
   `Start-Phantom-Demo.bat`
3. The launcher will verify dependencies, resolve ports, boot background servers, and automatically open your default browser.

---

## 3. Demo Modes

### Showcase Mode
A guided incident progression showcasing steady-state, flash crowds, coalescing suppression, key replication, node crashes, and automatic policy rollbacks.

### Explorer Mode
Allows the presenter to trigger events (Normal, Flash Crowd, Node failure) in any order using the control dashboard buttons.

---

## 4. Diagnostics & Troubleshooting
* If the API fails to initialize, verify that port `8080` is not occupied by an external application.
* Detailed logs are generated in `api.log` (backend) and `ui/ui.log` (frontend).
