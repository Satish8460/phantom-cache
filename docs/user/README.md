# PhantomCache user documentation

PhantomCache is a research prototype of an in-memory HTTP caching gateway with request coalescing. Read [limitations](limitations.md) before relying on it.

| Guide | Purpose |
|---|---|
| [install.md](install.md) | Build from source or install a release bundle; verify checksums |
| [configuration.md](configuration.md) | Every configuration key, CLI flag, precedence, validation rules |
| [run.md](run.md) | Start the gateway, exercise the API, enable authentication and TLS |
| [operate.md](operate.md) | Health, readiness, shutdown, exit codes, metrics, runbooks |
| [troubleshoot.md](troubleshoot.md) | Startup errors and runtime symptoms |
| [security.md](security.md) | Security model, controls, hardening checklist |
| [limitations.md](limitations.md) | What is not established or not implemented |

Evidence labels used everywhere: **measured** (read from a live counter), **simulated** (synthetic, from the model or `PHANTOM_SIMULATION=true`), **inferred**, **unavailable**. Simulated values are never measurements.
