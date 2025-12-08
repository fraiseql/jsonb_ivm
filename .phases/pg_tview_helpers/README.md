# pg_tview Helpers Implementation Phases

**Project:** jsonb_ivm v0.3.0 - pg_tview Integration Helpers
**Start Date:** 2025-12-08
**Estimated Duration:** 4 weeks
**Target:** Simplify pg_tview implementation + provide CRUD operations for JSONB arrays

---

## 📋 Phase Overview

| Phase | Title | Duration | Priority | Status |
|-------|-------|----------|----------|--------|
| [Phase 1](phase-1-smart-patch.md) | Smart Patch Dispatcher | 1 week | 🔴 Critical | ⏸️ Not Started |
| [Phase 2](phase-2-array-crud.md) | Array CRUD Operations | 1 week | 🔴 Critical | ⏸️ Not Started |
| [Phase 3](phase-3-deep-merge.md) | Deep Merge & Helpers | 1 week | 🟡 High | ⏸️ Not Started |
| [Phase 4](phase-4-integration.md) | Integration & Benchmarks | 1 week | 🟢 Medium | ⏸️ Not Started |

---

## 🎯 Project Goals

### Primary Goals
1. **Simplify pg_tview implementation** by 40-60%
2. **Complete JSONB array CRUD** operations (INSERT/DELETE missing)
3. **Eliminate re-aggregation** for INSERT/DELETE operations (**3-5× speedup**)
4. **Provide smart dispatcher** to reduce pg_tview complexity

### Secondary Goals
5. Create reusable primitives for other CQRS extensions
6. Maintain jsonb_ivm's performance standards (no regressions)
7. Comprehensive test coverage for all new functions

---

## 📦 Deliverables

### New Functions (6 total)
- ✅ `jsonb_smart_patch()` - Intelligent update dispatcher
- ✅ `jsonb_array_delete_where()` - Delete array element by ID
- ✅ `jsonb_array_insert_where()` - Insert with optional sort order
- ✅ `jsonb_deep_merge()` - Recursive deep merge
- ✅ `jsonb_extract_id()` - Safe ID extraction helper
- ✅ `jsonb_array_contains_id()` - Fast containment check

### Documentation
- ✅ API reference for all 6 functions
- ✅ Performance benchmarks vs. re-aggregation
- ✅ Integration examples for pg_tview
- ✅ Updated README.md with v0.3.0 features

### Tests
- ✅ Unit tests for all functions (edge cases)
- ✅ Integration tests with pg_tview patterns
- ✅ Performance benchmarks
- ✅ Regression tests (ensure existing functions unaffected)

---

## 🏗️ Architecture

### Module Structure
```
src/
├── lib.rs                    # Main module, re-exports
├── merge.rs                  # Existing: jsonb_merge_shallow, jsonb_merge_at_path
├── array.rs                  # Existing: jsonb_array_update_where, _batch, _multi_row
├── array_crud.rs             # NEW: jsonb_array_delete_where, jsonb_array_insert_where
├── deep_merge.rs             # NEW: jsonb_deep_merge (recursive logic)
├── smart_patch.rs            # NEW: jsonb_smart_patch (dispatcher)
├── helpers.rs                # NEW: jsonb_extract_id, jsonb_array_contains_id
└── util.rs                   # Existing utilities
```

### Dependency Graph
```
jsonb_smart_patch()
├─→ jsonb_merge_shallow()        (existing)
├─→ jsonb_merge_at_path()        (existing)
└─→ jsonb_array_update_where()   (existing)

jsonb_array_delete_where()
└─→ find_by_int_id_optimized()   (existing)

jsonb_array_insert_where()
└─→ No dependencies (standalone)

jsonb_deep_merge()
└─→ deep_merge_recursive()       (internal helper)

jsonb_extract_id()
└─→ No dependencies (standalone)

jsonb_array_contains_id()
└─→ find_by_int_id_optimized()   (existing)
```

---

## 🎓 Development Principles

### 1. Performance First
- All new functions must match or exceed existing performance standards
- Use loop unrolling for array operations (follow `find_by_int_id_optimized` pattern)
- Minimize allocations (clone only when necessary)
- Benchmark against SQL re-aggregation baseline

### 2. Error Handling
- Use `strict` attribute where appropriate (NULL handling)
- Use `error!()` macro for invalid inputs (follows pgrx convention)
- Validate all array paths and object keys before mutations
- Return unchanged input on non-fatal errors (e.g., path not found)

### 3. Code Quality
- Comprehensive doc comments with SQL examples
- Unit tests for each function (happy path + edge cases)
- Integration tests simulating pg_tview patterns
- Follow existing jsonb_ivm code style

### 4. Backward Compatibility
- No breaking changes to existing functions
- New functions are additive only
- Performance of existing functions must not regress

---

## 📊 Success Criteria

### Must Have (Phase 1-2)
- [x] `jsonb_smart_patch()` implemented and tested
- [x] `jsonb_array_delete_where()` implemented and tested
- [x] `jsonb_array_insert_where()` implemented and tested
- [x] All existing tests pass (no regressions)
- [x] 3-5× speedup for INSERT/DELETE vs. re-aggregation

### Should Have (Phase 3)
- [x] `jsonb_deep_merge()` implemented and tested
- [x] `jsonb_extract_id()` implemented and tested
- [x] `jsonb_array_contains_id()` implemented and tested
- [x] Documentation updated with new API

### Nice to Have (Phase 4)
- [x] Integration examples showing pg_tview usage
- [x] Performance comparison table in README
- [x] Blog post / announcement draft

---

## 🚀 Getting Started

1. **Read all phase documents** in order (phase-1 through phase-4)
2. **Set up development environment** (see DEVELOPMENT.md)
3. **Run existing tests** to establish baseline:
   ```bash
   cargo pgrx test --release
   ```
4. **Start with Phase 1** (smart_patch.md)
5. **Complete phases sequentially** (each builds on previous)

---

## 📝 Phase Checklist

### Before Starting Each Phase
- [ ] Read phase document completely
- [ ] Understand acceptance criteria
- [ ] Review code examples
- [ ] Set up test database if needed

### During Phase Implementation
- [ ] Write function implementation
- [ ] Write unit tests (aim for 80%+ coverage)
- [ ] Write integration tests
- [ ] Write documentation (doc comments + examples)
- [ ] Run benchmarks if applicable

### After Completing Each Phase
- [ ] All tests pass (`cargo pgrx test`)
- [ ] No performance regressions
- [ ] Code reviewed (self-review minimum)
- [ ] Documentation complete
- [ ] Mark phase as ✅ Complete in this README

---

## 🔄 Iteration Strategy

### Weekly Reviews
- **End of each phase**: Review progress against acceptance criteria
- **Adjust timeline** if needed (phases can be split or merged)
- **Document blockers** and decisions in phase files

### Continuous Integration
- Run tests after each significant change
- Benchmark after completing each new function
- Update documentation incrementally

---

## 📞 Support & Questions

- **Documentation**: See `/docs/PG_TVIEW_HELPERS_PROPOSAL.md` for rationale
- **Code style**: Follow patterns in `src/lib.rs` and `src/array.rs`
- **Performance**: Reference `docs/implementation/BENCHMARK_RESULTS.md`

---

**Next Step:** Read [Phase 1: Smart Patch Dispatcher](phase-1-smart-patch.md)
