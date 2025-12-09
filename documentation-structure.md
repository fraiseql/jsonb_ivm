# Documentation Structure

This document explains the organization of documentation in the jsonb_ivm project.

## Directory Structure

```
jsonb_ivm/
├── docs/
│   ├── implementation/           # Current implementation documentation
│   │   ├── BENCHMARK_RESULTS.md         # Performance benchmark results
│   │   ├── IMPLEMENTATION_SUCCESS.md    # Implementation details & verification
│   │   └── PGRX_INTEGRATION_ISSUE.md    # pgrx SQL generation troubleshooting
│   ├── archive/                 # Historical documentation
│   │   ├── phases/                      # Phase plan history
│   │   │   ├── fix-pgrx-sql-generation.md
│   │   │   ├── fix-pgrx-sql-generation-v2.md
│   │   │   └── fraiseql-mutation-optimization-analysis.md
│   │   ├── POC_IMPLEMENTATION_PLAN.md   # Original POC planning
│   │   └── POC_QUICKSTART.md            # POC quickstart guide
│   └── planning/                # Future planning (empty)
├── .phases/                     # Active phase plans
│   └── README.md                        # Phase status (POC complete)
├── test/                        # Tests and benchmarks
│   ├── sql/                             # SQL test files
│   ├── fixtures/                        # Test data generators
│   └── benchmark_*.sql                  # Performance benchmarks
├── README.md                    # Main project documentation
├── CHANGELOG.md                 # Version history
├── DEVELOPMENT.md               # Development guide
└── CODE_REVIEW_PROMPT.md        # Code review guidelines
```

## Documentation Categories

### Active Documentation (Root Level)

- **README.md** - Main project overview, API reference, quick start
- **CHANGELOG.md** - Version history and release notes
- **DEVELOPMENT.md** - Development setup and guidelines
- **CODE_REVIEW_PROMPT.md** - Code review standards

### Implementation Documentation (docs/implementation/)

Current implementation details and results:

1. **BENCHMARK_RESULTS.md** - Complete performance analysis
   - Benchmark results (1.45× to 2.66× speedup)
   - Performance breakdown and bottlenecks
   - Real-world impact calculations
   - Recommendations for optimization

2. **IMPLEMENTATION_SUCCESS.md** - Technical implementation details
   - Option A implementation (bare types + strict)
   - Root cause analysis (pgrx issue #268)
   - Verification steps and test results
   - Lessons learned

3. **PGRX_INTEGRATION_ISSUE.md** - Troubleshooting guide
   - SQL generation issue analysis
   - pgrx 0.12.8 constraints
   - Solution approaches evaluated
   - Implementation recommendations

### Archived Documentation (docs/archive/)

Historical POC planning and phase plans:

1. **POC_IMPLEMENTATION_PLAN.md** - Original POC strategy and goals
2. **POC_QUICKSTART.md** - POC development quickstart
3. **phases/** - Phase plan history
   - Implementation approaches evaluated
   - Decision rationale and trade-offs

### Active Phases (.phases/)

Currently empty - POC is complete.

**README.md** explains:
- POC complete status
- Links to archived phase plans
- Next steps (alpha release)

## Finding Documentation

### "How do I use this extension?"
→ **README.md** - API reference and examples

### "How does it perform?"
→ **docs/implementation/BENCHMARK_RESULTS.md**

### "How was it implemented?"
→ **docs/implementation/IMPLEMENTATION_SUCCESS.md**

### "Why this approach?"
→ **docs/archive/phases/** - Phase plan history

### "How do I contribute?"
→ **DEVELOPMENT.md** - Development guide

### "What changed?"
→ **CHANGELOG.md** - Version history

## Maintenance Guidelines

### When to Update

- **README.md** - API changes, new features, version updates
- **CHANGELOG.md** - Every release
- **BENCHMARK_RESULTS.md** - Performance improvements
- **DEVELOPMENT.md** - Build process changes

### When to Archive

Move to `docs/archive/` when:
- Documentation becomes historical reference
- Planning docs are superseded by implementation
- POC/prototype documentation is complete

### What NOT to Archive

Keep in root or `docs/implementation/`:
- Active API documentation
- Current performance benchmarks
- Implementation details of shipped features
- Development/contribution guides

## Status

**Last Updated**: 2025-12-08
**POC Status**: ✅ Complete
**Next Phase**: Alpha release preparation

---

*This structure supports clean, professional documentation ready for open-source alpha release.*
