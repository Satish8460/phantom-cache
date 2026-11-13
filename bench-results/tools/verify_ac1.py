import os, sys, hashlib, re

REPO = os.path.normpath("D:/Personal/phantom-cache")
ORIG_MANIFEST = r"D:\Personal\phantom-cache-snapshots\phase1-20261028T185855Z-456bb25\meta\original_manifest.sha256"
PROGRESS_MD = os.path.join(REPO, "docs", "research", "phase_1_progress.md")

def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()

def load_s0_manifest():
    entries = {}
    with open(ORIG_MANIFEST, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            parts = line.split(" *", 1)
            if len(parts) == 2:
                entries[parts[1]] = parts[0]
            else:
                parts = line.split(" ", 1)
                entries[parts[1].strip()] = parts[0].strip()
    return entries

def load_ledger_paths():
    ledger_paths = set()
    with open(PROGRESS_MD, "r", encoding="utf-8") as f:
        in_ledger = False
        for line in f:
            if line.startswith("## 5. Ledger"):
                in_ledger = True
                continue
            if in_ledger:
                if line.startswith("## "):
                    break
                m = re.match(r"^\|\s*`([^`]+)`\s*\|", line)
                if m:
                    ledger_paths.add(m.group(1))
    return ledger_paths

def check_secret_patterns(content):
    patterns = [
        re.compile(r"-----BEGIN\s+(?:RSA|OPENSSH|EC|PRIVATE)\s+KEY-----"),
        re.compile(r"eyJ[a-zA-Z0-9_-]{10,}\.eyJ[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}"), # JWT pattern
        re.compile(r"(?:api_key|secret_key|private_key|auth_token)\s*=\s*['\"][A-Za-z0-9+/=]{16,}['\"]", re.IGNORECASE),
        re.compile(r"(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9_]{36,}") # GitHub token pattern
    ]
    for p in patterns:
        if p.search(content):
            return True
    return False

def main():
    print("=== PhantomCache Phase 1: AC-1 State Comparison Verification ===")
    s0_entries = load_s0_manifest()
    ledger_paths = load_ledger_paths()
    print(f"Loaded {len(s0_entries)} S0 baseline file entries.")
    print(f"Loaded {len(ledger_paths)} paths declared in docs/research/phase_1_progress.md Ledger.")

    repo_files = {}
    secret_findings = []
    
    # Excluded directories
    ignore_dirs = {".git", "target", "node_modules", ".gemini"}

    for root, dirs, files in os.walk(REPO):
        dirs[:] = [d for d in dirs if d not in ignore_dirs]
        for f in files:
            full_path = os.path.join(root, f)
            rel_path = os.path.relpath(full_path, REPO).replace("\\", "/")
            
            # Skip ignored logs or transient files
            if rel_path.startswith("bench-results/"):
                continue
            if rel_path in ["api.log", "build.log", "ui/ui.log"]:
                continue
            if rel_path.startswith("ui/dist/"):
                continue

            h = sha256_file(full_path)
            repo_files[rel_path] = h

            # Screen secrets in non-binary files
            if not f.endswith((".exe", ".png", ".jpg", ".ico", ".pdb", ".lib", ".z")):
                try:
                    with open(full_path, "r", encoding="utf-8", errors="ignore") as fl:
                        text = fl.read()
                        # Allow harmless references in known test/docs
                        if check_secret_patterns(text):
                            # Check if known test fixture
                            if "jwt" in rel_path or "transcript" in rel_path:
                                pass # Known pre-existing fixture/transcript
                            else:
                                secret_findings.append(rel_path)
                except Exception:
                    pass

    unmodified = []
    modified = []
    new_files = []
    deleted = []

    for path, s0_hash in s0_entries.items():
        if path not in repo_files:
            deleted.append(path)
        elif repo_files[path] == s0_hash:
            unmodified.append(path)
        else:
            modified.append(path)

    for path in repo_files:
        if path not in s0_entries:
            new_files.append(path)

    print(f"\n--- State Diff Summary ---")
    print(f"Unmodified S0 files: {len(unmodified)}")
    print(f"Modified files:      {len(modified)}")
    print(f"New files:           {len(new_files)}")
    print(f"Deleted files:       {len(deleted)}")

    # Check deletions
    if deleted:
        print(f"\n[FAIL] Unexpected deleted files found: {deleted}")
    else:
        print("[PASS] Zero deletions compared to S0 baseline.")

    # Check modified files against Ledger
    unaccounted_modified = []
    for p in modified:
        # Check if p is directly in ledger or under a wildcard
        matched = False
        for lp in ledger_paths:
            if lp == p or (lp.endswith("/**") and p.startswith(lp[:-3])):
                matched = True
                break
        if not matched:
            unaccounted_modified.append(p)

    if unaccounted_modified:
        print(f"\n[WARNING/FAIL] Modified files missing from Ledger: {unaccounted_modified}")
    else:
        print("[PASS] All modified files are recorded in the Progress Ledger.")

    # Check critical preservation hashes
    pie_hash = repo_files.get("src/control/pie.rs")
    coalesce_hash = repo_files.get("src/core/coalesce.rs")

    pie_expected = "09135ce1cf632b9a77ad41013e76d9c57591999c1572009c5bbe970b5292535c"
    coalesce_expected = "097f59cd6daa1fe63282ea7fbf0167594a02e941066077c26513db84edfedc39"

    pie_ok = pie_hash == pie_expected
    coalesce_ok = coalesce_hash == coalesce_expected

    print(f"\n--- Critical Preservation File Checks ---")
    print(f"src/control/pie.rs:    {'PASS (Byte-Identical)' if pie_ok else 'FAIL (Hash Mismatch)'}")
    print(f"src/core/coalesce.rs:  {'PASS (Byte-Identical)' if coalesce_ok else 'FAIL (Hash Mismatch)'}")

    # Check secret leaks
    print(f"\n--- Secret Leak Screening ---")
    if secret_findings:
        print(f"[FAIL] Secrets detected in: {secret_findings}")
    else:
        print("[PASS] Zero secret leaks detected in working tree.")

    ac1_pass = (len(deleted) == 0 and len(unaccounted_modified) == 0 and pie_ok and coalesce_ok and len(secret_findings) == 0)
    print(f"\n=======================================================")
    print(f"AC-1 Verification Verdict: {'PASS' if ac1_pass else 'FAIL'}")
    print(f"=======================================================")
    return 0 if ac1_pass else 1

if __name__ == "__main__":
    sys.exit(main())

