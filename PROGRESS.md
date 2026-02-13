# EngineQA 开发进度

## 当前状态（2026-02-13）
- 默认运行基线：Python 后端（`backend-python/`）+ Frontend（`frontend/`）。
- Rust 后端（`backend/`）目前存在运行问题，暂不作为交付和验收基线。
- 文档和脚本已切换为 Python 路径优先。

## Step 完成概览
### Step-01 ~ Step-12
- 项目骨架、配置中心、Provider、检索、问答链路、错误映射、索引任务、状态接口、反馈闭环、前端问答/历史/状态页均已完成。

### Step-13
- 已提供：
  - `scripts/acceptance-test.sh`
  - `scripts/load-test.sh`
  - `scripts/security-check.sh`
  - `scripts/smoke-step-13.sh`
  - `docs/acceptance-criteria.md`

### Step-14
- 已提供：
  - `docs/deployment.md`
  - `docs/runbook.md`
  - 启动与运维说明

## 当前可用交付物
### 前端
- 问答页：提问、答案、来源、反馈
- 历史页：查看/删除/清空本地历史
- 状态页：系统状态、索引任务触发与进度查看

### 后端（Python）
- `GET /health`
- `POST /api/query`
- `GET /api/status`
- `POST /api/feedback`
- `POST /api/reindex`
- `GET /api/reindex`

### 启动与验证
- `make dev`（默认 Python）
- `SKIP_QDRANT=1 ./scripts/smoke-step-01.sh`
- `./scripts/smoke-step-13.sh`

## 已知问题
- Rust 后端当前不稳定，待后续专项修复。

## 下一步建议
1. 若继续推进 Rust：建立独立修复分支，先恢复 `cargo run` 基线。
2. 增加 Python 后端自动化测试覆盖（query/reindex/status 主链路）。
3. 对 `scripts/load-test.sh` 与 CI 集成做标准化，减少手工步骤。
