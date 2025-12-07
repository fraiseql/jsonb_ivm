# Phase 4: CI/CD - Detailed Task Breakdown

**Objective**: Update all CI/CD workflows and documentation for Rust + pgrx

**Strategy**: Each task below is designed to be delegated to local LLM with explicit instructions

---

## Task 1: Update GitHub Actions Test Workflow

**File**: `.github/workflows/test.yml`

**Task**: Replace entire file with Rust/pgrx version

**Explicit Instructions for Local LLM**:
```
Replace the entire content of .github/workflows/test.yml with this exact YAML:

[PASTE COMPLETE YAML BELOW]
```

**Complete YAML**:
```yaml
name: Test

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: PostgreSQL ${{ matrix.pg-version }}
    runs-on: ubuntu-latest

    strategy:
      fail-fast: false
      matrix:
        pg-version: [17]

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cache Rust dependencies
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-

      - name: Install PostgreSQL ${{ matrix.pg-version }}
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            postgresql-${{ matrix.pg-version }} \
            postgresql-server-dev-${{ matrix.pg-version }}

      - name: Install cargo-pgrx
        run: |
          cargo install --locked cargo-pgrx --version 0.12.8

      - name: Initialize pgrx
        run: |
          cargo pgrx init --pg${{ matrix.pg-version }}=/usr/lib/postgresql/${{ matrix.pg-version }}/bin/pg_config

      - name: Run Rust unit tests
        run: |
          cargo pgrx test pg${{ matrix.pg-version }}

      - name: Build extension
        run: |
          cargo build --release --locked

      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: test-results-pg${{ matrix.pg-version }}
          path: |
            target/pgrx-test-data-*
          if-no-files-found: ignore
```

**Verification**:
- File exists: `.github/workflows/test.yml`
- Contains: `cargo pgrx test`
- Contains: `matrix: pg-version: [17]`

---

## Task 2: Update GitHub Actions Lint Workflow

**File**: `.github/workflows/lint.yml`

**Task**: Replace entire file with Rust linting workflow

**Explicit Instructions for Local LLM**:
```
Replace the entire content of .github/workflows/lint.yml with this exact YAML:

[PASTE COMPLETE YAML BELOW]
```

**Complete YAML**:
```yaml
name: Lint

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  format:
    name: Rust Format Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt

      - name: Check Rust formatting
        run: cargo fmt -- --check

  clippy:
    name: Rust Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy

      - name: Cache dependencies
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-clippy-${{ hashFiles('**/Cargo.lock') }}

      - name: Install PostgreSQL dev headers
        run: |
          sudo apt-get update
          sudo apt-get install -y postgresql-server-dev-17

      - name: Install cargo-pgrx
        run: cargo install --locked cargo-pgrx --version 0.12.8

      - name: Initialize pgrx
        run: cargo pgrx init --pg17 download

      - name: Run Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - name: Install cargo-audit
        run: cargo install cargo-audit

      - name: Run security audit
        run: cargo audit
```

**Verification**:
- File exists: `.github/workflows/lint.yml`
- Contains: `cargo fmt -- --check`
- Contains: `cargo clippy`
- Contains: `cargo audit`

---

## Task 3: Create GitHub Actions Release Workflow

**File**: `.github/workflows/release.yml`

**Task**: Create new file with release automation

**Explicit Instructions for Local LLM**:
```
Create a new file .github/workflows/release.yml with this exact content:

[PASTE COMPLETE YAML BELOW]
```

**Complete YAML**:
```yaml
name: Release

on:
  push:
    tags:
      - 'v*.*.*'

env:
  CARGO_TERM_COLOR: always

jobs:
  build-release:
    name: Build Release for PostgreSQL ${{ matrix.pg-version }}
    runs-on: ubuntu-latest

    strategy:
      matrix:
        pg-version: [17]

    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - name: Install PostgreSQL ${{ matrix.pg-version }}
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            postgresql-${{ matrix.pg-version }} \
            postgresql-server-dev-${{ matrix.pg-version }}

      - name: Install cargo-pgrx
        run: cargo install --locked cargo-pgrx --version 0.12.8

      - name: Initialize pgrx
        run: cargo pgrx init --pg${{ matrix.pg-version }}=/usr/lib/postgresql/${{ matrix.pg-version }}/bin/pg_config

      - name: Build release package
        run: |
          cargo pgrx package --pg-config=/usr/lib/postgresql/${{ matrix.pg-version }}/bin/pg_config

      - name: Create tarball
        run: |
          cd target/release/jsonb_ivm-pg${{ matrix.pg-version }}
          tar czf ../jsonb_ivm-${{ github.ref_name }}-pg${{ matrix.pg-version }}.tar.gz .

      - name: Upload release artifact
        uses: actions/upload-artifact@v4
        with:
          name: jsonb_ivm-pg${{ matrix.pg-version }}
          path: target/release/jsonb_ivm-${{ github.ref_name }}-pg${{ matrix.pg-version }}.tar.gz

  create-release:
    name: Create GitHub Release
    needs: build-release
    runs-on: ubuntu-latest
    permissions:
      contents: write

    steps:
      - uses: actions/checkout@v4

      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: release-artifacts

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: release-artifacts/*/*.tar.gz
          body: |
            ## jsonb_ivm ${{ github.ref_name }}

            High-performance PostgreSQL extension for incremental JSONB view maintenance.

            **Built with Rust + pgrx for memory safety and quality.**

            See [CHANGELOG.md](CHANGELOG.md) for details.
          draft: false
          prerelease: ${{ contains(github.ref, 'alpha') || contains(github.ref, 'beta') }}
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

**Verification**:
- File created: `.github/workflows/release.yml`
- Contains: `on: push: tags`
- Contains: `cargo pgrx package`

---

## Task 4: Update README.md Installation Section

**File**: `README.md`

**Task**: Replace installation section with Rust instructions

**Explicit Instructions for Local LLM**:
```
In README.md, find the section that starts with "## 🚀 Quick Start" and contains "### Installation".

Replace ONLY the installation subsection (from "### Installation" until the next "###" header) with this content:

[PASTE MARKDOWN BELOW]
```

**Complete Markdown**:
```markdown
### Installation

**From source (requires Rust):**

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install cargo-pgrx
cargo install --locked cargo-pgrx

# Initialize pgrx (one-time setup)
cargo pgrx init

# Clone and build
git clone https://github.com/fraiseql/jsonb_ivm.git
cd jsonb_ivm
cargo pgrx install --release

# Load extension in your database
psql -d your_database -c "CREATE EXTENSION jsonb_ivm;"
```

**From binary release (PostgreSQL 17):**

```bash
# Download release for your PostgreSQL version
wget https://github.com/fraiseql/jsonb_ivm/releases/download/v0.1.0-alpha1/jsonb_ivm-v0.1.0-alpha1-pg17.tar.gz

# Extract to PostgreSQL extension directory
sudo tar xzf jsonb_ivm-v0.1.0-alpha1-pg17.tar.gz -C /usr/share/postgresql/17/extension

# Load extension
psql -d your_database -c "CREATE EXTENSION jsonb_ivm;"
```
```

**Verification**:
- README.md contains: `cargo pgrx install`
- README.md contains: `cargo install --locked cargo-pgrx`
- Old C build instructions removed

---

## Task 5: Update README.md Badges

**File**: `README.md`

**Task**: Update badge URLs to point to new workflows

**Explicit Instructions for Local LLM**:
```
In README.md, find these two lines near the top:

[![Test](https://github.com/fraiseql/jsonb_ivm/actions/workflows/test.yml/badge.svg)](https://github.com/fraiseql/jsonb_ivm/actions/workflows/test.yml)
[![Lint](https://github.com/fraiseql/jsonb_ivm/actions/workflows/lint.yml/badge.svg)](https://github.com/fraiseql/jsonb_ivm/actions/workflows/lint.yml)

Verify they exist and are correct. If not, add them immediately after the "[![License]..." line.
```

**Verification**:
- Badges present in README.md
- Point to correct workflow files

---

## Task 6: Update CHANGELOG.md for Rust Migration

**File**: `CHANGELOG.md`

**Task**: Replace v0.1.0-alpha1 entry with Rust migration notes

**Explicit Instructions for Local LLM**:
```
In CHANGELOG.md, find the section "## [0.1.0-alpha1] - 2025-12-07".

Replace the entire section (from "## [0.1.0-alpha1]" until the next "##" or end of file) with:

[PASTE MARKDOWN BELOW]
```

**Complete Markdown**:
```markdown
## [0.1.0-alpha1] - 2025-12-07

### 🎉 Initial Alpha Release - Rust Migration Complete

This is the first public release of jsonb_ivm, completely rewritten in **Rust + pgrx** for memory safety and quality.

### Added

- **Core Function**: `jsonb_merge_shallow(target, source)`
  - Shallow merge of two JSONB objects
  - Source keys overwrite target keys on conflicts
  - NULL-safe with proper error handling
  - IMMUTABLE and PARALLEL SAFE for query optimization
  - **Memory-safe Rust implementation** - compiler-verified safety guarantees

- **Testing Infrastructure**
  - 6 comprehensive Rust unit tests (`#[pg_test]`)
  - 12 SQL integration tests (from original C version)
  - Tests for NULL handling, empty objects, large objects, Unicode
  - 100% test coverage on core logic

- **CI/CD Pipeline**
  - GitHub Actions workflow for PostgreSQL 17 testing
  - Automated code quality checks (rustfmt, clippy)
  - Security scanning (cargo audit)
  - Zero compiler warnings enforcement
  - Automated release packaging

- **Code Quality**
  - Written in Rust for memory safety
  - Zero unsafe code blocks
  - Comprehensive rustdoc documentation
  - Type-safe error handling
  - Automated formatting enforcement

- **Documentation**
  - README with installation and usage
  - Complete API documentation (rustdoc)
  - Migration guide from C implementation
  - Development guide for contributors

### Technical Details

- **Language**: Rust (edition 2021)
- **Framework**: pgrx 0.12.8
- **PostgreSQL Compatibility**: 17 (13-16 support planned for v0.2.0)
- **Build System**: Cargo
- **License**: PostgreSQL License

### Migration Notes

- **Breaking Change**: This version is a complete rewrite in Rust
- Previous C implementation archived but API remains 100% compatible
- No changes needed for existing SQL code using the extension
- Binary packages now built with cargo-pgrx instead of PGXS

### Why Rust?

- **Memory Safety**: Compiler prevents entire classes of bugs (use-after-free, null pointer dereference)
- **Quality First**: In the era of LLM-generated code, quality is the differentiator
- **Modern Tooling**: cargo, clippy, rustfmt provide superior development experience
- **Future-Proof**: Growing PostgreSQL + Rust ecosystem (Supabase, Neon use pgrx)

### Notes

- This is an **alpha release** - API is stable but may receive minor improvements
- Not recommended for production use yet (beta planned for v0.5.0)
- Focused on minimal viable functionality with perfect quality
- Foundation for incremental feature additions in future alphas

---

## Roadmap

### Planned for v0.2.0-alpha1
- Multi-version PostgreSQL support (13-17) in CI
- Nested path merge function: `jsonb_merge_at_path(target, source, path)`
- Additional performance benchmarks

### Planned for v0.3.0-alpha1
- Change detection: `jsonb_detect_changes(old, new, keys)`
- Sub-millisecond performance validation

### Planned for v0.4.0-alpha1
- Scope building system
- Configuration-driven update patterns

### Planned for v0.5.0-beta1
- Feature complete
- Seek early adopters
- Real-world validation

### Planned for v1.0.0
- Production-ready release
- Published to PGXN
- Community validation
```

**Verification**:
- CHANGELOG.md mentions Rust migration
- Contains "Memory Safety" section
- Contains "Why Rust?" section

---

## Task 7: Update README.md Requirements Section

**File**: `README.md`

**Task**: Update requirements for Rust build

**Explicit Instructions for Local LLM**:
```
In README.md, find the section "## 🛠️ Requirements".

Replace the entire section content with:

[PASTE MARKDOWN BELOW]
```

**Complete Markdown**:
```markdown
## 🛠️ Requirements

### For Building from Source

- **Rust**: 1.83+ (install via [rustup](https://rustup.rs))
- **PostgreSQL**: 13-17 with dev headers
- **cargo-pgrx**: 0.12.8+
- **OS**: Linux, macOS
- **Build Tools**: Standard C compiler (gcc/clang) for PostgreSQL compilation

### Installing Build Dependencies

**Arch Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo pacman -S base-devel
cargo install --locked cargo-pgrx
cargo pgrx init
```

**Debian/Ubuntu:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt-get install build-essential postgresql-server-dev-17
cargo install --locked cargo-pgrx
cargo pgrx init
```

**macOS:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
brew install postgresql@17
cargo install --locked cargo-pgrx
cargo pgrx init
```

### For Using Binary Releases

- **PostgreSQL**: 17 (matching the binary you download)
- **OS**: Linux x86_64 (Ubuntu 20.04+, Debian 11+, Arch Linux)
- **No Rust required** for binary installation
```

**Verification**:
- Mentions Rust 1.83+
- Contains cargo-pgrx installation
- Removed C compiler from main requirements

---

## Task 8: Create DEVELOPMENT.md

**File**: `DEVELOPMENT.md` (new file)

**Task**: Create developer guide for Rust/pgrx workflow

**Explicit Instructions for Local LLM**:
```
Create a new file DEVELOPMENT.md with this exact content:

[PASTE MARKDOWN BELOW - see phase-4-cicd.md for full content]
```

**Note**: Copy full DEVELOPMENT.md content from phase-4-cicd.md "Step 5: Add Development Documentation"

**Verification**:
- File created: `DEVELOPMENT.md`
- Contains: `cargo pgrx run pg17`
- Contains: development workflow instructions

---

## Summary for Local LLM Execution

**Total Tasks**: 8 atomic file operations

**Delegation Strategy**:
1. Tasks 1-3: YAML file replacements (exact content provided)
2. Tasks 4-6: Markdown section replacements (exact content provided)
3. Task 7: Section replacement (exact content provided)
4. Task 8: New file creation (exact content provided)

**Prompt Template for Each Task**:
```
Task: [Task Name]
File: [File Path]
Action: [Replace section / Create file / Update lines]

Exact content to use:
```
[paste content]
```

Verify:
- [verification point 1]
- [verification point 2]
```

**Local LLM Success Factors**:
- ✅ Exact content provided (no generation needed)
- ✅ Clear file paths
- ✅ Explicit action (replace vs append vs create)
- ✅ Verification points
- ✅ One file per task (simple scope)

**Expected Success Rate**: 95%+ (these are copy-paste operations)
