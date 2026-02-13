# EngineQA

广告引擎现网维优 QA 问答系统（Internal API 推理版）。

## 当前实现状态（2026-02-13）
- 默认可运行后端：`backend-python/`（FastAPI）。
- Rust 后端：`backend/` 当前存在已知运行问题，暂不作为运行基线。
- 当前文档和脚本均以 Python 后端路径为准。

## 目录结构
- `frontend/`: React + Vite + Tailwind 前端。
- `backend-python/`: Python FastAPI 后端（默认）。
- `backend/`: Rust Axum 后端（暂不作为运行基线）。
- `docs/`: 启动、部署、验收、运维文档。
- `scripts/`: 启动、冒烟、验收、压测、安全检查脚本。
- `knowledge/`: Markdown 知识库目录。

## 端口约定
- Frontend: `5173`
- Backend: `8080`
- Qdrant HTTP（remote 模式）: `6333`
- Qdrant gRPC（remote 模式）: `6334`

## 快速启动（Host-Run，推荐）
1. 安装前端依赖：
```bash
npm install --prefix frontend
```

2. 安装 Python 后端依赖（首次）：
```bash
python3 -m venv .venv-backend-python
.venv-backend-python/bin/pip install -r backend-python/requirements.txt
```

3. 初始化环境变量：
```bash
cp .env.example .env
```

4. 至少配置以下必填项：
- `INTERNAL_API_BASE_URL`
- `INTERNAL_API_TOKEN`

如 chat/embed 地址不同，建议配置：
- `INTERNAL_API_CHAT_BASE_URL`
- `INTERNAL_API_EMBED_BASE_URL`
- `INTERNAL_API_CHAT_PATH`
- `INTERNAL_API_EMBED_PATH`
- `EMBEDDING_VECTOR_SIZE`

5. 启动（默认就是 Python 后端）：
```bash
make dev
```

等价命令：
```bash
BACKEND_RUNTIME=python make dev
```

## 运行验证
健康检查：
```bash
curl -fsS http://127.0.0.1:8080/health
curl -fsS http://127.0.0.1:5173 >/dev/null
```

Step-01 冒烟：
```bash
SKIP_QDRANT=1 ./scripts/smoke-step-01.sh
```

## API 清单
- `GET /health`
- `POST /api/query`
- `GET /api/status`
- `POST /api/feedback`
- `POST /api/reindex`
- `GET /api/reindex`

## 常用脚本
- `scripts/dev.sh`: 统一入口（默认 `BACKEND_RUNTIME=python`）。
- `scripts/dev-python.sh`: 启动 Python 后端 + 前端。
- `scripts/dev-rust.sh`: 启动 Rust 后端 + 前端（当前不推荐）。
- `scripts/run-qdrant.sh`: 启动本机 qdrant 二进制（remote 模式时可选）。
- `scripts/smoke-step-01.sh`: 基础冒烟。
- `scripts/smoke-step-13.sh`: Step-13 冒烟。
- `scripts/acceptance-test.sh`: 验收测试。
- `scripts/security-check.sh`: 安全检查。

## Rust 后端说明
Rust 代码仍保留在仓库中，但当前运行基线是 Python 后端。Rust 后端修复前，文档、验收和启动流程默认不依赖 `backend/`。
