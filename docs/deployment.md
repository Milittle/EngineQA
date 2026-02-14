# EngineQA 部署指南

## 1. 说明
本文覆盖两条可部署路径：
- Python 后端 + Qdrant（当前默认运行基线）
- Rust 后端 + LanceDB（向量存储重构后路径）

日期基线：2026-02-14。

## 2. 部署拓扑
通用：
- 前端：`frontend`（5173，或构建后由 Nginx 托管）
- 后端：`backend-python` 或 `backend`
- 上游：Internal API

Python 路径：
- 向量库：Qdrant embedded（默认）或 remote（可选）

Rust 路径：
- 向量库：LanceDB 本地目录（默认 `./.lancedb`）

## 3. 前置条件
- Linux 主机（推荐 Ubuntu 22.04+）
- Node.js 18+
- 网络可访问 Internal API

Python 路径额外：
- Python 3.10+
- 可选：qdrant 二进制（仅 Qdrant remote 需要）

Rust 路径额外：
- Rust toolchain（建议 stable）

## 4. 通用准备
```bash
cd /opt
git clone <repo-url> engineqa
cd engineqa

npm install --prefix frontend
cp .env.example .env
```

必填：
- `INTERNAL_API_BASE_URL`
- `INTERNAL_API_TOKEN`

## 5. Python 后端部署（Qdrant）
### 5.1 安装依赖
```bash
python3 -m venv .venv-backend-python
.venv-backend-python/bin/pip install -r backend-python/requirements.txt
```

### 5.2 推荐配置
```dotenv
BACKEND_RUNTIME=python
QDRANT_MODE=embedded
QDRANT_LOCAL_PATH=./.qdrant-local
```

### 5.3 启动
```bash
BACKEND_RUNTIME=python make dev
```

仅后端：
```bash
.venv-backend-python/bin/python -m uvicorn app.main:app \
  --app-dir backend-python --host 0.0.0.0 --port 8080
```

### 5.4 可选：Qdrant remote
```dotenv
QDRANT_MODE=remote
QDRANT_URL=http://127.0.0.1:6333
```

```bash
./scripts/run-qdrant.sh
```

## 6. Rust 后端部署（LanceDB）
### 6.1 构建
```bash
cargo build --manifest-path backend/Cargo.toml --release
```

### 6.2 推荐配置
```dotenv
BACKEND_RUNTIME=rust
VECTOR_STORE=lancedb
LANCEDB_URI=./.lancedb
LANCEDB_TABLE=knowledge_chunks
VECTOR_SCORE_THRESHOLD=0.3
EMBEDDING_VECTOR_SIZE=1536
```

### 6.3 启动
```bash
BACKEND_RUNTIME=rust make dev
```

仅后端：
```bash
cargo run --manifest-path backend/Cargo.toml
```

## 7. 部署验证
```bash
curl -fsS http://127.0.0.1:8080/health
curl -fsS http://127.0.0.1:8080/api/status
curl -fsS http://127.0.0.1:5173 >/dev/null
```

首次索引（建议全量）：
```bash
curl -s -X POST http://127.0.0.1:8080/api/reindex \
  -H 'Content-Type: application/json' \
  -d '{"full": true}'

curl -s http://127.0.0.1:8080/api/reindex
```

## 8. 可选反向代理（Nginx）
建议：
- `/` -> 前端静态资源
- `/api/` 与 `/health` -> `http://127.0.0.1:8080`

上线前确认：
- `client_max_body_size` 满足业务需求
- 超时设置匹配 `LLM_TIMEOUT_MS` / `EMBED_TIMEOUT_MS`

## 9. 回滚策略
- 代码回滚：
```bash
git revert <commit_sha>
```
- 配置回滚：保留 `.env` 历史版本
- 回滚后执行：
```bash
BACKEND_RUNTIME=python ./scripts/smoke-step-01.sh
BACKEND_RUNTIME=rust ./scripts/smoke-step-01.sh
./scripts/security-check.sh
```

## 10. 风险与已知注意点
- Internal API 配置不一致会导致 `UPSTREAM_*` 降级。
- 向量维度变更需同步 `EMBEDDING_VECTOR_SIZE` 并重建索引。
- Rust 路径下 LanceDB 数据目录需有写权限。
