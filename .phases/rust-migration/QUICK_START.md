# Quick Start: Rust Migration

**TL;DR**: Complete guide to migrate jsonb_ivm from C to Rust in 4 phases (~9 hours)

---

## 🚀 Execute Now (Copy-Paste Commands)

### Phase 1: Setup (2 hours)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install pgrx
cargo install --locked cargo-pgrx

# Initialize pgrx (downloads PostgreSQL 13-17, takes ~30min)
cargo pgrx init --pg13 download --pg14 download --pg15 download --pg16 download --pg17 download

# Archive old C files
cd /home/lionel/code/jsonb_ivm
mkdir .archive-c-implementation
mv jsonb_ivm.c jsonb_ivm.control jsonb_ivm--0.1.0.sql Makefile .archive-c-implementation/

# Create Cargo.toml (see phase-1-setup.md for content)
# Create src/lib.rs (see phase-1-setup.md for content)
# Create .cargo/config.toml (see phase-1-setup.md for content)

# Verify
cargo build --release
cargo pgrx schema
```

**Acceptance**: `cargo build` succeeds with zero warnings

---

### Phase 2: Implement (3 hours)

```bash
# Replace src/lib.rs with full implementation
# (see phase-2-implement.md for complete code)

# Build
cargo build --release

# Test
cargo pgrx test pg17

# Lint
cargo fmt
cargo clippy --all-targets -- -D warnings

# Manual test
cargo pgrx run pg17
# In psql:
# SELECT jsonb_merge_shallow('{"a":1}', '{"b":2}');
```

**Acceptance**: All Rust unit tests pass, zero warnings

---

### Phase 3: Test (2 hours)

```bash
# Install extension
cargo pgrx install --release

# Run SQL tests
psql -d postgres << 'SQL'
DROP DATABASE IF EXISTS test_jsonb_ivm;
CREATE DATABASE test_jsonb_ivm;
\c test_jsonb_ivm
CREATE EXTENSION jsonb_ivm;
\i test/sql/01_merge_shallow.sql
SQL

# Create test runner (see phase-3-test.md for script)
./run_tests.sh
```

**Acceptance**: All 12 SQL tests pass

---

### Phase 4: CI/CD (2 hours)

```bash
# Update GitHub Actions workflows
# (see phase-4-cicd.md for complete YAML files)

# Update .github/workflows/test.yml
# Update .github/workflows/lint.yml  
# Create .github/workflows/release.yml

# Update README.md with Rust instructions
# Update CHANGELOG.md
# Update .gitignore for Rust

# Test locally
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo audit
cargo pgrx test pg13 pg14 pg15 pg16 pg17

# Commit and push
git add .
git commit -m "feat: migrate to Rust + pgrx [v0.1.0-alpha1]"
git push origin main

# Create release
git tag -a v0.1.0-alpha1 -m "Release v0.1.0-alpha1"
git push origin v0.1.0-alpha1
```

**Acceptance**: GitHub Actions all green, release created

---

## 📊 Progress Tracker

```
Phase 1: Setup          [                    ] 0%
Phase 2: Implement      [                    ] 0%
Phase 3: Test           [                    ] 0%
Phase 4: CI/CD          [                    ] 0%
Overall Progress:       [                    ] 0%
```

Update this as you complete each phase!

---

## ✅ Checklist

### Phase 1
- [ ] Rust installed (`rustc --version`)
- [ ] pgrx installed (`cargo pgrx --version`)
- [ ] pgrx initialized (PostgreSQL 13-17)
- [ ] C files archived
- [ ] Cargo.toml created
- [ ] src/lib.rs created
- [ ] `cargo build` succeeds

### Phase 2
- [ ] Full implementation in src/lib.rs
- [ ] `cargo build --release` - zero warnings
- [ ] `cargo pgrx test pg17` - all pass
- [ ] `cargo clippy` - zero warnings
- [ ] `cargo fmt --check` - passes
- [ ] Manual testing works

### Phase 3
- [ ] Extension installed
- [ ] All 12 SQL tests pass
- [ ] NULL handling verified
- [ ] Error handling verified
- [ ] Unicode support verified
- [ ] Performance acceptable

### Phase 4
- [ ] test.yml updated
- [ ] lint.yml updated
- [ ] release.yml created
- [ ] README updated
- [ ] CHANGELOG updated
- [ ] All CI checks pass
- [ ] Release created

---

## 🆘 Troubleshooting

### Rust won't install
```bash
# Manual install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

### pgrx init fails
```bash
# Use system PostgreSQL instead of download
cargo pgrx init --pg17 /usr/bin/pg_config
```

### Compilation errors
```bash
# Clean and rebuild
cargo clean
cargo build --release
```

### Tests fail
```bash
# Verbose output
cargo pgrx test pg17 -- --nocapture

# Check logs
tail -f ~/.pgrx/data-17/postgresql.log
```

---

## 📚 Quick Reference

| Command | Purpose |
|---------|---------|
| `cargo build` | Compile extension |
| `cargo pgrx test pg17` | Run tests |
| `cargo pgrx run pg17` | Start psql with extension |
| `cargo pgrx install` | Install to system |
| `cargo pgrx schema` | Generate SQL file |
| `cargo fmt` | Format code |
| `cargo clippy` | Lint code |
| `cargo audit` | Security scan |

---

## 🎯 Expected Results

### After Phase 1
- Rust toolchain installed
- Empty project compiles
- pgrx configured

### After Phase 2  
- `jsonb_merge_shallow` implemented
- Rust unit tests passing
- Zero warnings

### After Phase 3
- SQL tests passing
- 100% compatibility with C version
- Performance validated

### After Phase 4
- CI/CD automated
- Release published
- Ready for users

---

## ⏱️ Time Estimates

- **Phase 1**: 2 hours (mostly pgrx init waiting)
- **Phase 2**: 3 hours (implementation + debugging)
- **Phase 3**: 2 hours (SQL testing)
- **Phase 4**: 2 hours (CI/CD setup)
- **Total**: 9 hours (can span 2-3 days)

---

## 🎉 Success = v0.1.0-alpha1 Released

When done:
- Rust implementation complete
- All tests passing
- CI/CD automated
- GitHub release published
- Ready for PostgreSQL community announcement

**Let's build something great!** 🦀
