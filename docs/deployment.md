# EngineQA 部署指南

## 1. 说明
本文覆盖两条可部署路径：
- Python 后端 + Qdrant embedded（当前默认运行基线）
- Rust 后端 + LanceDB（向量存储重构后路径）

日期基线：2026-02-14。

## 2. 部署拓扑
通用：
- 前端：`frontend`（5173，或构建后由 Nginx 托管）
- 后端：`backend-python` 或 `backend`
- 上游：Internal API

Python 路径：
- 向量库：Qdrant embedded（默认且唯一支持）

Rust 路径：
- 向量库：LanceDB 本地目录（默认 `./.lancedb`）

## 3. 前置条件
- Linux 主机（推荐 Ubuntu 22.04+）
- Node.js 18+
- 网络可访问 Internal API

Python 路径额外：
- Python 3.10+

Rust 路径额外：
- Rust toolchain（建议 stable）

## 4. 打包发布产物运行依赖（目标机器）
本节适用于已生成并分发的运行包（`engineqa-python-backend-*` 或 `enginqa-rust-backend-*`）。

- 平台一致：运行包与目标机器 `os/arch` 必须一致。
- Nginx：目标机器已安装 `nginx` 且命令可通过 `PATH` 直接调用。
- 网络可达：目标机器可访问 Internal API 地址。
- 必填环境变量：
  - `INTERNAL_API_BASE_URL`
  - `INTERNAL_API_TOKEN`
- 端口可用：
  - `FRONTEND_PORT`（默认 `5173`）
  - `APP_PORT`（默认 `8080`）
- 目录权限：
  - 可读写 `data/`、`logs/`、`run/`
  - 可读 `knowledge/`
- 向量数据目录默认值：
  - Rust 路径：`LANCEDB_URI=${ROOT_DIR}/data/.lancedb`
  - Python 路径：`QDRANT_LOCAL_PATH=${ROOT_DIR}/data/.qdrant-local`
- 资源说明：当前项目未定义硬性 CPU/内存最低规格，建议按并发量与知识库规模做容量评估。

补充：
- Python 运行包由 PyInstaller 生成，目标机通常不需要预装 Python 解释器。

## 5. 通用准备
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

## 6. Python 后端部署（Qdrant）
### 6.1 安装依赖
```bash
python3 -m venv .venv-backend-python
.venv-backend-python/bin/pip install -r backend-python/requirements.txt
```

### 6.2 推荐配置
```dotenv
BACKEND_RUNTIME=python
QDRANT_LOCAL_PATH=./.qdrant-local
QDRANT_COLLECTION=knowledge_chunks
```

### 6.3 启动
```bash
BACKEND_RUNTIME=python make dev
```

仅后端：
```bash
.venv-backend-python/bin/python -m uvicorn app.main:app \
  --app-dir backend-python --host 0.0.0.0 --port 8080
```

## 7. Rust 后端部署（LanceDB）
### 7.1 构建
```bash
cargo build --manifest-path backend/Cargo.toml --release
```

### 7.2 推荐配置
```dotenv
BACKEND_RUNTIME=rust
VECTOR_STORE=lancedb
LANCEDB_URI=./.lancedb
LANCEDB_TABLE=knowledge_chunks
VECTOR_SCORE_THRESHOLD=0.3
EMBEDDING_VECTOR_SIZE=1536
```

### 7.3 启动
```bash
BACKEND_RUNTIME=rust make dev
```

仅后端：
```bash
cargo run --manifest-path backend/Cargo.toml
```

## 8. 部署验证
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

## 9. 可选反向代理（Nginx）
建议：
- `/` -> 前端静态资源
- `/api/` 与 `/health` -> `http://127.0.0.1:8080`

上线前确认：
- `client_max_body_size` 满足业务需求
- 超时设置匹配 `LLM_TIMEOUT_MS` / `EMBED_TIMEOUT_MS`

## 10. 回滚策略
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

## 11. 风险与已知注意点
- Internal API 配置不一致会导致 `UPSTREAM_*` 降级。
- 向量维度变更需同步 `EMBEDDING_VECTOR_SIZE` 并重建索引。
- Rust 路径下 LanceDB 数据目录需有写权限。
