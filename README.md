# EngineQA

广告引擎现网维优 QA 问答系统（Internal API 推理版）。

## 当前实现状态（2026-02-14）
- 默认运行基线：`backend-python/`（FastAPI + Qdrant embedded）。
- Rust 后端：`backend/` 已完成向量存储重构，检索与索引链路使用 LanceDB 本地存储。
- 统一入口脚本：`scripts/dev.sh`，通过 `BACKEND_RUNTIME` 切换运行时，或者使用`make dev`、`make dev-python`、`make dev-rust`来启动后端

## 目录结构
- `frontend/`: React + Vite + Tailwind 前端。
- `backend-python/`: Python FastAPI 后端（默认）。
- `backend/`: Rust Axum 后端（LanceDB 向量存储）。
- `docs/`: 启动、部署、验收、运维文档。
- `scripts/`: 启动、冒烟、验收、压测、安全检查脚本。
- `knowledge/`: Markdown 知识库目录。

## 端口约定
- Frontend: `5173`
- Backend: `8080`

## 快速启动（Host-Run，推荐）
1. 安装前端依赖：
```bash
npm install --prefix frontend
```

2. 初始化环境变量：
```bash
cp .env.example .env
```

3. 至少配置以下必填项：
- `INTERNAL_API_BASE_URL`
- `INTERNAL_API_TOKEN`

4. 按运行时准备依赖：

Python（默认）
```bash
python3 -m venv .venv-backend-python
.venv-backend-python/bin/pip install -r backend-python/requirements.txt
make dev
```

Rust（LanceDB）
```bash
cargo build --manifest-path backend/Cargo.toml
BACKEND_RUNTIME=rust make dev
```

5. 等价命令（统一入口）：
```bash
make dev                # 默认 python
make dev-python
make dev-rust
```

## 运行验证
健康检查：
```bash
curl -fsS http://127.0.0.1:8080/health
curl -fsS http://127.0.0.1:5173 >/dev/null
curl -fsS http://127.0.0.1:8080/api/status
```

Step-01 冒烟：
```bash
BACKEND_RUNTIME=python ./scripts/smoke-step-01.sh
BACKEND_RUNTIME=rust ./scripts/smoke-step-01.sh
```

## 关键环境变量
通用：
- `INTERNAL_API_BASE_URL`
- `INTERNAL_API_TOKEN`
- `INTERNAL_API_CHAT_PATH`
- `INTERNAL_API_EMBED_PATH`
- `EMBEDDING_VECTOR_SIZE`

Rust + LanceDB：
- `VECTOR_STORE=lancedb`
- `LANCEDB_URI=./.lancedb`
- `LANCEDB_TABLE=knowledge_chunks`
- `VECTOR_SCORE_THRESHOLD=0.3`

Python + Qdrant：
- `QDRANT_LOCAL_PATH=./.qdrant-local`
- `QDRANT_COLLECTION=knowledge_chunks`

## API 清单
- `GET /health`
- `POST /api/query`
- `GET /api/status`
- `POST /api/feedback`
- `POST /api/reindex`
- `GET /api/reindex`

## 常用脚本
- `scripts/dev.sh`: 统一入口（根据 `BACKEND_RUNTIME` 分发）。
- `scripts/dev-python.sh`: 启动 Python 后端 + 前端。
- `scripts/dev-rust.sh`: 启动 Rust 后端 + 前端（LanceDB）。
- `scripts/smoke-step-01.sh`: 基础冒烟（运行时感知）。
- `scripts/smoke-step-13.sh`: Step-13 冒烟（状态接口感知向量存储）。
- `scripts/acceptance-test.sh`: 验收测试。
- `scripts/security-check.sh`: 安全检查。
