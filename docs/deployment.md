# EngineQA 部署指南

## 1. 说明
本文基于当前可运行实现（Python 后端）编写。

当前基线（2026-02-13）：
- `backend-python/` 可运行并作为部署基线
- `backend/`（Rust）暂不纳入部署路径

## 2. 部署拓扑
- 前端：`frontend`（5173，或构建后由 Nginx 托管）
- 后端：`backend-python`（8080）
- 向量库：Qdrant embedded（默认）或 remote（可选）
- 上游：Internal API

## 3. 前置条件
- Linux 主机（推荐 Ubuntu 22.04+）
- Node.js 18+
- Python 3.10+
- 网络可访问 Internal API
- 可选：qdrant 二进制（仅 remote 模式需要本机进程）

## 4. 部署步骤
### 4.1 准备代码与依赖
```bash
cd /opt
git clone <repo-url> engineqa
cd engineqa

npm install --prefix frontend
python3 -m venv .venv-backend-python
.venv-backend-python/bin/pip install -r backend-python/requirements.txt
```

### 4.2 配置环境变量
```bash
cp .env.example .env
```

必填：
- `INTERNAL_API_BASE_URL`
- `INTERNAL_API_TOKEN`

建议：
- `BACKEND_RUNTIME=python`
- `QDRANT_MODE=embedded`（默认）
- `QDRANT_LOCAL_PATH=./.qdrant-local`

### 4.3 启动服务
开发方式（前后端同启）：
```bash
BACKEND_RUNTIME=python make dev
```

生产推荐分开进程：
```bash
# backend
.venv-backend-python/bin/python -m uvicorn app.main:app \
  --app-dir backend-python --host 0.0.0.0 --port 8080

# frontend（可二选一）
# 1) 开发预览
npm run dev --prefix frontend -- --host 0.0.0.0 --port 5173

# 2) 构建并交给静态服务（推荐）
npm run build --prefix frontend
```

### 4.4 可选：Qdrant remote 模式
若使用 remote：
```dotenv
QDRANT_MODE=remote
QDRANT_URL=http://127.0.0.1:6333
```

本机 qdrant 启动：
```bash
./scripts/run-qdrant.sh
```

## 5. 部署验证
```bash
curl -fsS http://127.0.0.1:8080/health
curl -fsS http://127.0.0.1:8080/api/status
curl -fsS http://127.0.0.1:5173 >/dev/null
```

首次索引：
```bash
curl -s -X POST http://127.0.0.1:8080/api/reindex \
  -H 'Content-Type: application/json' \
  -d '{}'

curl -s http://127.0.0.1:8080/api/reindex
```

## 6. 可选反向代理（Nginx）
建议：
- `/` -> 前端静态资源
- `/api/` 与 `/health` -> `http://127.0.0.1:8080`

上线前确认：
- `client_max_body_size` 满足业务需求
- 超时设置匹配后端 `LLM_TIMEOUT_MS` / `EMBED_TIMEOUT_MS`

## 7. 回滚策略
- 代码回滚：
```bash
git revert <commit_sha>
```
- 配置回滚：保留 `.env` 历史版本
- 回滚后执行：
```bash
SKIP_QDRANT=1 ./scripts/smoke-step-01.sh
./scripts/security-check.sh
```

## 8. 风险与已知问题
- Rust 后端当前不稳定，部署时请固定 Python 路径。
- `INTERNAL_API_*` 配置不一致时会导致 `UPSTREAM_*` 降级。
- 向量维度变更时需同步更新 `EMBEDDING_VECTOR_SIZE` 并重建索引。
