# Package 04: browser and device matrix

**Status:** Draft for owner review. **Audience:** a tester running the matrix by hand, and the owner. **Last reviewed:** 2026-10-30 18:17 IST (machine clock, from `date`). **Index and rules:** `00_README.md`. **Scope source:** DEC-008: Chrome current stable, Firefox current stable, at least one real mobile phone. No WebKit or Safari automation and no cloud service. No new automation tooling is approved: this matrix is run by a person.

**Build under test (fill first):** commit or tree `[FILL]`; UI `[FILL: dev server or production build]`; gateway `[FILL: release or debug]`.

## 1. What this establishes
That, on the recorded versions, the listed pages and states work (or do not) in each browser and on one real phone. It does not cover Safari or WebKit (an iPhone using any iOS browser runs WebKit underneath, so an iPhone cannot satisfy a Chrome or Firefox row; it would be a Safari-engine result that DEC-008 puts out of automation scope, see section 7), or any other browser version or device.

Earlier evidence is Chrome 156.0.8078.79 headless on one Windows 11 machine, driven by scripts. Nothing has been checked in Firefox or on a real phone (`r5_coverage_matrix.md`, K05 and K09: NOT COVERED).

## 2. The cells
Three columns: **Chrome stable (desktop)**, **Firefox stable (desktop)**, **real phone**. Rows are the checks below. Fill every cell with PASS, FINDING (with ID), NOT REACHABLE, or NOT DONE, plus a note.

For every cell record, at the top of the column: browser name and **exact version** (from the browser's About page), OS and version, screen size and pixel ratio, theme setting, extensions disabled (yes/no), date and tester code. Take a screenshot or short note for any FINDING.

### What to record per cell
| Item | Detail |
|---|---|
| Versions | Browser exact version; OS exact version; for the phone also make, model, OS version, browser (Chrome, Firefox, or the default) and its version |
| Rendering | Layout intact; no clipped, overlapping or missing controls; fonts load; light and dark both correct; icons present |
| Function | The action works; the state changes; no console errors (open developer tools, Console, and record red errors; note that expected "connection refused" errors appear when the gateway is stopped) |
| Streaming | Live data updates arrive (telemetry stream); the run page updates while a run is going; reconnect works after the gateway restarts |
| Storage | Only the gateway address is saved; **no token anywhere** (developer tools, Application/Storage: local storage, session storage, cookies; and the address bar) |
| Dialogs | The cancel confirmation opens as a modal, traps focus, closes with Escape |
| Network | In the Network panel: no failed requests other than expected ones; no CORS errors |
| Performance feel | Note only "smooth" or "janky" and the machine; no performance claim comes from this package |

### Checks (rows)
| ID | Check | How |
|---|---|---|
| B1 | Load and orient | Open the UI with the gateway running; Overview shows Live |
| B2 | Navigate every page | Overview, Traffic, Cache, Freshness, Origin, Phantom Lab (all four tabs), Simulation, Cluster, Intelligence |
| B3 | Deep links | Paste `#/lab/runs`, `#/lab/evidence` and a run id URL into the address bar; an unknown route falls back with a notice |
| B4 | Theme | Light, dark, system; reload keeps the choice |
| B5 | Start, follow and complete a short run | Quick check; progress updates; result appears |
| B6 | Cancel a run | Native dialog, Escape, confirm; cancelled page shows no verdict |
| B7 | Disconnect and recover | Stop the gateway; status changes within a few seconds; restart; status returns without reloading |
| B8 | Connection popover and address | Open it; Test connection; change address to the real gateway from a wrong one |
| B9 | Evidence pages | Open the M6 and Scenario B retained results |
| B10 | Charts and tables | Chart/Table toggle shows identical values |
| B11 | Reduced motion and transparency | Set the OS options; B2 and B5 still complete |
| B12 | Forced or high contrast | Firefox: Settings, Colors, and "Override"/forced colours; Chrome: OS contrast theme |
| B13 | Zoom 200 percent | Browser zoom; no page-level sideways scroll |
| B14 | Console and network | No unexpected errors during B1 to B10 |
| B15 | Storage | As in the table above; token never persisted |
| B16 | Keyboard essentials | Tab to skip link, nav, a form; use Enter/Space; Escape on dialog |
| B17 | Touch (phone only) | Tap targets, scrolling, sticky bar overlap, menu, pinch zoom |
| B18 | Orientation (phone only) | Portrait and landscape |
| B19 | Screen size (phone) | At the phone's actual width (do not use the desktop emulator for the phone column) |

### Matrix to fill
| Check | Chrome stable | Firefox stable | Real phone |
|---|---|---|---|
| Version and OS | `[FILL]` | `[FILL]` | `[FILL]` |
| B1 | | | |
| B2 | | | |
| B3 | | | |
| B4 | | | |
| B5 | | | |
| B6 | | | |
| B7 | | | |
| B8 | | | |
| B9 | | | |
| B10 | | | |
| B11 | | | |
| B12 | | | |
| B13 | | | |
| B14 | | | |
| B15 | | | |
| B16 | | | |
| B17 | n/a | n/a | |
| B18 | n/a | n/a | |
| B19 | n/a | n/a | |

Finding log (IDs `BDM-001...`): `ID | cell (browser, check) | what happened | expected | severity | evidence | owner`.

## 3. How to open the dev build and the gateway
(Same as `00_README.md` section 7; repeated so this file can be used alone. Commands come from project docs and were not re-run when this was written.)

**Gateway.** Build `cargo build --release --locked -j 1`. Run with `PHANTOM_LAB_ENABLED=true`, `PHANTOM_LAB_DIR=<prepared folder>`, `PHANTOM_MODE=development`. It listens on `127.0.0.1:8085`. In this mode it only answers requests whose host name is `localhost`, `127.0.0.1` or `[::1]` with its own port (other host names get HTTP 421), and cross-origin requests must come from an origin in `PHANTOM_CORS_ORIGINS` (the development defaults include `http://localhost:3000` and `http://127.0.0.1:3000`). Check the configuration first with `--check-config`.

**UI.** `cd ui`, `npm ci`, `npm run dev`; open `http://localhost:3000`. To point at a gateway on another port use `?gateway=http://localhost:<port>`. For a production build: `npm run build`, serve `ui/dist` with a static server you trust (the earlier runs never used a production build).

**Firefox.** Same URLs on the same machine. If Firefox is configured to block local network or mixed content, note it.

## 4. Phone: the connectivity problem (open item)
A phone cannot open `http://localhost:3000` meaning the laptop. And the anonymous gateway refuses any host name that is not loopback, and refuses to listen on a network address without JWT and TLS (`docs/user/configuration.md`). So the phone cannot simply be pointed at the laptop's address. Options, none verified here:

| Option | Idea | Notes |
|---|---|---|
| A. Android with USB port forwarding | Connect an Android phone by cable, enable debugging, and use Chrome's remote-debugging port forwarding (or `adb reverse tcp:3000 tcp:3000` and `tcp:8085 tcp:8085`) so the phone's `localhost:3000` and `localhost:8085` reach the laptop. The page then sees `localhost`, so the gateway's loopback guard passes | Likely the cheapest; needs a cable, developer mode and the Android debugging tools; the owner has to install anything missing (not approved by this document) |
| B. Run the UI and gateway in the secure configuration | Gateway with a JWT secret, TLS and a network bind; UI served over a trusted origin; the phone enters a minted token | Heavy; needs TLS certificates the phone trusts and care with secrets (see package 05 for token handling); tests different code paths (token field) than option A |
| C. iPhone | Any iOS browser uses WebKit; this is Safari-engine behaviour, which DEC-008 excludes from automation and does not list as required | Do not use as the "real mobile phone" unless the owner decides it counts |

`[FILL: owner chooses the phone, OS and option]`. Whichever is chosen, say in the results which option was used, because with option B the token field becomes part of the test.

## 5. Known gaps and risks specific to browsers (from the coverage matrix and code)
- The UI relies on `EventSource` (live data), fetch streaming (the run stream), and the native `<dialog>` modal for the cancel confirmation. The browser baseline is undefined; `r5_coverage_matrix.md` row K09 says non-Chrome behaviour is NOT COVERED.
- Firefox forced-colours handling and the way it draws focus rings differ from Chrome; check B12 and B16 especially.
- `EventSource` does not send an Authorization header, so live telemetry on a JWT-protected gateway cannot work from the browser (BD-05); the UI plans to say so honestly. Test this only if option B is used.
- The earlier validation noted that mobile emulation modes were inconsistent between runs; do not substitute emulation for the real phone.
- Safari and WebKit: not tested, not planned (DEC-008). State this in any report. Do not describe the product as "cross-browser".

## 6. Acceptance criteria
**Complete** when each cell has a result and a note, versions are recorded exactly, and the log in `00_README.md` names the tester and date.

**Passes** if, as recorded: in Chrome stable and Firefox stable, B1 to B10 and B14 to B16 are PASS or have only Low findings the owner accepts; on the phone, B1, B2, B4 to B7, B17 and B19 are PASS (B8 if the popover exists); no cell shows a token stored or displayed (B15); no cell shows a verdict on a cancelled run (B6).

Passing means "worked on these exact versions at this time". It is not cross-browser compatibility in general.

## 7. Sign-off
Testers `[FILL]`; dates `[FILL]`; versions as in the matrix; result `[FILL]`; owner acceptance by finding ID `[FILL]`. Until recorded, browser status stays **TESTED (headless Chrome 156 only; Firefox and phone not covered)**.
