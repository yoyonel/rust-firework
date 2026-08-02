# Universal Quality Canvas - Dashboard
*Last Updated: 2026-08-02*

## 1. Toolchain & Environment (Container Context)
| Tool / Compiler | Version / OS | Status | Execution Context |
| :--- | :--- | :--- | :--- |
| **Base OS** | Ubuntu 24.04 LTS | `[ OK ]` | `canvas-env:latest` |
| **Rust** | Cargo 1.97.1 | `[ OK ]` | Built-in container |
| **Python** | Python 3 + pip | `[ OK ]` | Built-in container |

## 2. Core Infrastructure
| Component | Status | CLI Command | Description |
| :--- | :--- | :--- | :--- |
| **Root Orchestrator** | `[ OK ]` | `task --list` | go-task API exposing all commands. |
| **Container Engine** | `[ OK ]` | `task env:sync` | Builds/Syncs the Docker image locally. |
| **Execution Wrapper** | `[ OK ]` | Self-Reentrant Tasks | Auto-reentrant container task execution. |
| **Project Initializer** | `[ OK ]` | `./scripts/init_project.sh` | Template instantiation for Rust workspace. |

## 3. QA & Testing Pipeline
| Test Typology | Status | Target CLI | Infrastructure Needs |
| :--- | :--- | :--- | :--- |
| **Unit Testing** | `[ OK ]` | `task test:unit` | Cargo unit test execution under Xvfb. |
| **Visual Regression** | `[ OK ]` | `task test:visual` | Headless Python multiprocessing pipeline (`verify_baselines.py`). |
| **Chaos / Stress** | `[ TODO ]` | `task test:chaos` | Fuzzing & stress test execution. |

## 4. CI/CD & Remote Pipelines
| Workflow | Status | Platform | Description |
| :--- | :--- | :--- | :--- |
| **Container Build CI** | `[ TODO ]` | GitHub Actions | Automated build and push to GHCR. |
| **PR Verification** | `[ TODO ]` | GitHub Actions | Format, Lint, and Unit Tests on PR. |
| **Visual CI** | `[ TODO ]` | GitHub Actions | Headless rendering validation with artifact upload. |
