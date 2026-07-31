# GitHub Actions CI/CD Pre-Merge Protection & Visual Non-Regression Mechanics

## 1. Executive Summary

This document specifies the exact execution lifecycle, status check mechanisms, and branch protection rules governing Pull Requests and Merges into `master` within `rust-firework`.

---

## 2. CI/CD Workflow Trigger Lifecycle

In `.github/workflows/ci.yml`, the workflow trigger and execution conditions are defined as:

```yaml
on:
  push:
    branches: ["**"]
  pull_request:
    branches: [master, develop]
```

### Job Execution Sequence:
1. **Pull Request Opened / Updated**:
   - As soon as a PR is created or updated with new commits, GitHub Actions triggers the `build-and-test` job.
   - The job runs in the background while the PR remains open.
   - **3-Keyframe Fast Test**: `task ci:coverage` runs `cargo test` including `tests/visual_regression_test.rs` (< 1 second runtime).

2. **Pre-Merge Master Validation (`task test-visual-full`)**:
   - When a Pull Request targets `master` (`github.event_name == 'pull_request' && github.base_ref == 'master'`), Step 7 invokes:
     ```yaml
     - name: Run Full 120-Frame Visual Non-Regression (Pre-Merge Master)
       if: github.event_name == 'pull_request' && github.base_ref == 'master'
       run: task test-visual-full
     ```
   - This step executes `scripts/run_visual_regression_full.sh` under `xvfb-run -a` and verifies all 120 frames per reference video.

---

## 3. Pull Request Blocking & Merge Protection Mechanism

### Scenario A: Test Suite Success (All Checks Green 🟢)
- All 120-frame visual match comparisons pass (`exit code 0`).
- GitHub Actions marks the `build-and-test` status check as **`PASSED (Green Checkmark 🟢)`**.
- The GitHub UI displays `No conflicts with base branch` and enables the **`[Rebase and merge]`** button.

### Scenario B: Visual Regression or GLFW Display Failure (Check Failed ❌)
- If any frame differs beyond tolerance or a display initialization error occurs, the script exits with `exit status 1`.
- GitHub Actions marks `build-and-test` as **`FAILED (Red Cross ❌)`**.
- **GitHub UI Behavior**:
  - The status box turns RED: `1 failing check`.
  - Under GitHub Branch Protection Rules (*Require status checks to pass before merging*), the **`[Rebase and merge]`** button is **GRAYED OUT & DISABLED**.
  - GitHub displays: `Merging is blocked. Required status check "build-and-test" failed.`

---

## 4. Headless Xvfb Display Requirement

Commands initializing GLFW windows (such as `fireworks_sim`) in headless CI environments MUST be wrapped with `xvfb-run -a` (or `{{.XVFB}}` in Taskfile) to prevent `GLFW Error: X11: Failed to open display` panics.
