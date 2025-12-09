# CI Failures Analysis - PR #8

## Summary

CI failures in PR #8 (v0.3.1-quality-improvements) across multiple jobs.

**PR**: https://github.com/fraiseql/jsonb_ivm/pull/8
**Latest Run**: https://github.com/fraiseql/jsonb_ivm/actions/runs/20059204782

## Failure Categories

### 1. PostgreSQL Package Availability Issues ❌

**Affected Jobs**:
- Rust Clippy (postgresql-server-dev-17)
- PostgreSQL 13, 14, 15, 16, 17 (Ubuntu)
- PostgreSQL 17 (macOS)

**Error**:
```
E: Unable to locate package postgresql-17
E: Unable to locate package postgresql-server-dev-17
E: Unable to locate package postgresql-14
E: Unable to locate package postgresql-13
```

**Root Cause**: Ubuntu 24.04 (noble) doesn't have PostgreSQL 13-17 in default repositories.

**Fix Needed**: Add PostgreSQL APT repository before installing.

### 2. Markdown Link Check Failures ❌

**Broken Links**:
1. `docs/implementation/BENCHMARK_RESULTS.md` → Status: 400
2. `CODE_REVIEW_PROMPT.md` → Status: 400

**Root Cause**: These files exist locally but aren't pushed to the repository (or have wrong case/path).

**Fix Needed**: Either push these files or fix/remove the links.

### 3. Pre-commit.ci Failure ❌

**Error**: "checks completed with failures"

**Likely Cause**: Same as local pre-commit run - SQL/markdown linting issues.

---

## Detailed Analysis

### Issue 1: PostgreSQL Repository Not Added

**Problem**: GitHub Actions Ubuntu runners use Ubuntu 24.04 (noble), which doesn't include PostgreSQL 13-17 in default repos.

**Current Code** (`.github/workflows/test.yml`):
```yaml
- name: Install PostgreSQL ${{ matrix.pg-version }}
  run: |
    if [ "$RUNNER_OS" == "Linux" ]; then
      sudo apt-get update
      sudo apt-get install -y \
        postgresql-${{ matrix.pg-version }} \
        postgresql-server-dev-${{ matrix.pg-version }}
```

**Missing**: PostgreSQL APT repository setup.

**Fix**:
```yaml
- name: Install PostgreSQL ${{ matrix.pg-version }}
  run: |
    if [ "$RUNNER_OS" == "Linux" ]; then
      # Add PostgreSQL APT repository
      sudo sh -c 'echo "deb http://apt.postgresql.org/pub/repos/apt $(lsb_release -cs)-pgdg main" > /etc/apt/sources.list.d/pgdg.list'
      wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | sudo apt-key add -

      sudo apt-get update
      sudo apt-get install -y \
        postgresql-${{ matrix.pg-version }} \
        postgresql-server-dev-${{ matrix.pg-version }}
```

**Apply to**:
- `.github/workflows/test.yml`
- `.github/workflows/lint.yml` (clippy job)
- `.github/workflows/benchmark.yml`

### Issue 2: macOS PostgreSQL Path

**Problem**: macOS PostgreSQL 17 path is incorrect.

**Error**:
```
The specified pg_config binary, `/usr/local/opt/postgresql@17/bin/pg_config`, does not exist
```

**Current Path**: `/usr/local/opt/postgresql@17/bin/pg_config`
**Likely Correct**: `/opt/homebrew/opt/postgresql@17/bin/pg_config` (Apple Silicon) or need to find it dynamically.

**Fix**:
```yaml
- name: Initialize pgrx
  run: |
    if [ "$RUNNER_OS" == "Linux" ]; then
      cargo pgrx init --pg${{ matrix.pg-version }}=/usr/lib/postgresql/${{ matrix.pg-version }}/bin/pg_config
    elif [ "$RUNNER_OS" == "macOS" ]; then
      # Find pg_config dynamically
      PG_CONFIG=$(brew --prefix postgresql@${{ matrix.pg-version }})/bin/pg_config
      cargo pgrx init --pg${{ matrix.pg-version }}=$PG_CONFIG
    fi
```

### Issue 3: Markdown Link Check - Broken Links

**Broken Links**:
1. **`docs/implementation/BENCHMARK_RESULTS.md`** (Status: 400)
   - Referenced in: `README.md`
   - File exists as: `docs/implementation/benchmark-results.md` (lowercase!)
   - **Fix**: Update link to use correct case

2. **`CODE_REVIEW_PROMPT.md`** (Status: 400)
   - Referenced in: `changelog.md`
   - File exists as: `code-review-prompt.md` (lowercase!)
   - **Fix**: Update link to use correct case

**Root Cause**: Case-sensitive file systems (Linux CI) vs case-insensitive (local macOS/Windows).

**Fixes**:

In `README.md`:
```markdown
# Before
See [benchmark results](docs/implementation/BENCHMARK_RESULTS.md)

# After
See [benchmark results](docs/implementation/benchmark-results.md)
```

In `changelog.md`:
```markdown
# Before
[CODE_REVIEW_PROMPT.md](CODE_REVIEW_PROMPT.md)

# After
[code-review-prompt.md](code-review-prompt.md)
```

### Issue 4: Cargo Version Mismatch

**Problem**: We updated to `pgrx 0.13.1` in `Cargo.toml` but workflows still reference `0.12.8`.

**Current** (`.github/workflows/*.yml`):
```yaml
- name: Install cargo-pgrx
  run: cargo install --locked cargo-pgrx --version 0.12.8
```

**Fix**:
```yaml
- name: Install cargo-pgrx
  run: cargo install --locked cargo-pgrx --version 0.13.1
```

**Apply to**:
- `.github/workflows/test.yml`
- `.github/workflows/lint.yml`
- `.github/workflows/benchmark.yml`
- `.github/workflows/release.yml`

---

## Fix Implementation Plan

### Phase 1: Update Workflows (High Priority)

**Files to modify**:
1. `.github/workflows/test.yml`
2. `.github/workflows/lint.yml`
3. `.github/workflows/benchmark.yml`
4. `.github/workflows/release.yml`

**Changes needed**:
- [ ] Add PostgreSQL APT repository setup (Ubuntu jobs)
- [ ] Fix macOS pg_config path detection
- [ ] Update cargo-pgrx version 0.12.8 → 0.13.1

### Phase 2: Fix Markdown Links (Medium Priority)

**Files to modify**:
1. `README.md` - Fix `BENCHMARK_RESULTS.md` → `benchmark-results.md`
2. `changelog.md` - Fix `CODE_REVIEW_PROMPT.md` → `code-review-prompt.md`

### Phase 3: Verify Pre-commit.ci (Low Priority)

- Check pre-commit.ci configuration
- Ensure it matches local `.pre-commit-config.yaml`

---

## Quick Fix Script

```bash
# Fix PostgreSQL Repository in all workflows
for file in .github/workflows/{test,lint,benchmark,release}.yml; do
  # Add PostgreSQL APT repository setup before apt-get install
  # Update cargo-pgrx version
  # Fix macOS paths
done

# Fix markdown link case issues
sed -i 's|BENCHMARK_RESULTS.md|benchmark-results.md|g' README.md
sed -i 's|CODE_REVIEW_PROMPT.md|code-review-prompt.md|g' changelog.md

# Commit and push
git add .github/workflows/*.yml README.md changelog.md
git commit -m "fix: resolve CI failures (PostgreSQL repos, markdown links, pgrx version)"
git push
```

---

## Expected Outcome After Fixes

### Passing Jobs:
- ✅ Rust Format Check
- ✅ Security Audit
- ✅ Documentation Coverage
- ✅ Markdown Lint
- ✅ Markdown Link Check (after link fixes)
- ✅ Rust Clippy (after PostgreSQL fixes)
- ✅ All PostgreSQL test matrix jobs (after repository fixes)

### Time Estimate:
- **Fix implementation**: 30 minutes
- **CI re-run**: 5-10 minutes
- **Total**: ~45 minutes to green CI

---

## Lessons Learned

1. **PostgreSQL Availability**: Always add PostgreSQL APT repository on Ubuntu runners
2. **Case Sensitivity**: Use lowercase filenames consistently (Unix-friendly)
3. **Dependency Versions**: Keep workflow cargo-pgrx version in sync with Cargo.toml
4. **macOS Paths**: Use dynamic path detection (`brew --prefix`) instead of hardcoded paths
5. **Pre-commit**: Run locally before pushing to catch issues early

---

## Next Steps

1. **Implement fixes** (all 4 workflows + 2 markdown files)
2. **Test locally** where possible (pre-commit, markdown linting)
3. **Push and monitor** CI run
4. **If still failing**: Check logs and iterate

**Priority**: HIGH - Blocking PR merge
**Estimated Fix Time**: 30-45 minutes
**Complexity**: Low (mostly configuration updates)
