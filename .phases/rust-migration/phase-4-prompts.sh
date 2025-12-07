#!/bin/bash
# Phase 4: Execute tasks via local vLLM model
# This script contains prompts for each task that can be sent to the local LLM

# vLLM endpoint
VLLM_URL="http://localhost:8000/v1/chat/completions"

# Helper function to call local LLM
call_llm() {
    local task_name="$1"
    local prompt_file="$2"

    echo "🤖 Executing Task: $task_name"

    curl -s "$VLLM_URL" \
        -H "Content-Type: application/json" \
        -d @"$prompt_file" \
        | jq -r '.choices[0].message.content'
}

# Task 1: Update test.yml
cat > /tmp/task1_prompt.json << 'EOF'
{
  "model": "/data/models/fp16/Ministral-3-8B-Instruct-2512",
  "messages": [{
    "role": "system",
    "content": "You are a YAML expert. Output only the exact file content requested, no explanations."
  }, {
    "role": "user",
    "content": "Output the complete content for .github/workflows/test.yml file.\n\nRequirements:\n- Name: Test\n- Trigger on push/PR to main branch\n- Matrix: pg-version [17]\n- Steps: checkout, install rust, install pgrx, run cargo pgrx test pg17\n- Uses: actions/checkout@v4, dtolnay/rust-toolchain@stable\n- Include cargo cache\n\nProvide complete, valid GitHub Actions YAML."
  }],
  "temperature": 0,
  "max_tokens": 1500
}
EOF

echo "Task 1: Update .github/workflows/test.yml"
echo "=========================================="
echo ""
echo "Manual approach (recommended for accuracy):"
echo "Copy content from: .phases/rust-migration/phase-4-detailed-tasks.md"
echo "Section: Task 1 - Complete YAML"
echo "Paste into: .github/workflows/test.yml"
echo ""
echo "OR use local LLM (may need verification):"
echo "call_llm \"Task 1\" /tmp/task1_prompt.json > .github/workflows/test.yml"
echo ""

# Task 2: Update lint.yml
echo "Task 2: Update .github/workflows/lint.yml"
echo "=========================================="
echo ""
echo "Manual approach (recommended):"
echo "Copy content from: .phases/rust-migration/phase-4-detailed-tasks.md"
echo "Section: Task 2 - Complete YAML"
echo "Paste into: .github/workflows/lint.yml"
echo ""

# Task 3: Create release.yml
echo "Task 3: Create .github/workflows/release.yml"
echo "============================================"
echo ""
echo "Manual approach (recommended):"
echo "Copy content from: .phases/rust-migration/phase-4-detailed-tasks.md"
echo "Section: Task 3 - Complete YAML"
echo "Create file: .github/workflows/release.yml"
echo ""

# Task 4-8: Documentation updates
echo "Tasks 4-8: Update Documentation"
echo "================================"
echo ""
echo "These tasks involve section replacements in existing files:"
echo ""
echo "Task 4: README.md - Installation section"
echo "Task 5: README.md - Badges"
echo "Task 6: CHANGELOG.md - v0.1.0-alpha1 entry"
echo "Task 7: README.md - Requirements section"
echo "Task 8: Create DEVELOPMENT.md"
echo ""
echo "Recommendation: Do these manually with careful copy-paste"
echo "All content is in: .phases/rust-migration/phase-4-detailed-tasks.md"
echo ""

# Summary
echo "=========================================="
echo "RECOMMENDED APPROACH FOR PHASE 4"
echo "=========================================="
echo ""
echo "Given the complexity of YAML and Markdown section replacements,"
echo "the SAFEST and FASTEST approach is:"
echo ""
echo "1. Open .phases/rust-migration/phase-4-detailed-tasks.md"
echo "2. For each task (1-8):"
echo "   - Find the 'Complete YAML' or 'Complete Markdown' section"
echo "   - Copy the exact content"
echo "   - Paste into the target file"
echo "   - Verify using the checklist"
echo ""
echo "Estimated time: 15-20 minutes for all 8 tasks"
echo ""
echo "This avoids potential LLM hallucination on complex YAML/Markdown structure"
echo "and ensures 100% accuracy."
echo ""

# Alternative: Automated approach with verification
echo "=========================================="
echo "ALTERNATIVE: Semi-Automated Approach"
echo "=========================================="
echo ""
echo "If you want to use local LLM for practice:"
echo ""
echo "1. Generate each file with LLM"
echo "2. Verify output matches expected structure"
echo "3. Fix any issues manually"
echo "4. Run verification: yamllint, markdownlint"
echo ""
echo "This is slower but good for learning LLM capabilities/limits."
echo ""
