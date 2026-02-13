# EngineQA 启动手册

## 1. 适用范围
本手册用于本机（Host-Run）启动 EngineQA。

当前基线（2026-02-13）：
- 前端：`frontend/`（Vite，默认 `5173`）
- 后端：`backend-python/`（FastAPI，默认 `8080`）
- Rust 后端当前有已知运行问题，暂不纳入启动路径

## 2. 前置条件
- Node.js + npm
- Python 3.10+
- `curl`

可选（仅 Qdrant remote 模式）：
- `qdrant` 二进制（用于 `scripts/run-qdrant.sh`）

## 3. 初始化
```bash
npm install --prefix frontend
python3 -m venv .venv-backend-python
.venv-backend-python/bin/pip install -r backend-python/requirements.txt
cp .env.example .env
```

## 4. 环境变量
必须配置：
- `INTERNAL_API_BASE_URL`
- `INTERNAL_API_TOKEN`

建议检查：
- `BACKEND_RUNTIME=python`
- `QDRANT_MODE=embedded`（默认）
- `QDRANT_LOCAL_PATH=./.qdrant-local`
- `EMBEDDING_VECTOR_SIZE` 与 embedding 模型维度一致

若 chat/embed 地址不同：
- `INTERNAL_API_CHAT_BASE_URL`
- `INTERNAL_API_EMBED_BASE_URL`
- `INTERNAL_API_CHAT_PATH`
- `INTERNAL_API_EMBED_PATH`

## 5. 启动方式
统一入口（默认 Python）：
```bash
make dev
```

显式指定 Python：
```bash
BACKEND_RUNTIME=python make dev
```

## 6. 启动后验证
```bash
curl -fsS http://127.0.0.1:8080/health
curl -fsS http://127.0.0.1:5173 >/dev/null
```

基础冒烟（Python embedded 模式建议加 `SKIP_QDRANT=1`）：
```bash
SKIP_QDRANT=1 ./scripts/smoke-step-01.sh
```

## 7. Qdrant 模式说明
### 7.1 Embedded（默认，推荐）
- `QDRANT_MODE=embedded`
- 不依赖独立 qdrant 进程
- 数据目录：`QDRANT_LOCAL_PATH`

### 7.2 Remote（可选）
- `QDRANT_MODE=remote`
- 需要可访问的 `QDRANT_URL`

如需本机起 qdrant：
```bash
./scripts/run-qdrant.sh
```

## 8. 常见问题
### 8.1 Python 依赖缺失
按提示安装：
```bash
.venv-backend-python/bin/pip install -r backend-python/requirements.txt
```

### 8.2 启动时报缺少必填环境变量
检查 `.env` 是否包含：
- `INTERNAL_API_BASE_URL`
- `INTERNAL_API_TOKEN`

### 8.3 查询返回 `UPSTREAM_*`
优先检查：
- Token 是否有效
- chat/embed 地址和 path 是否正确
- 上游服务可达性

### 8.4 检索失败或 `qdrant_connected=false`
检查：
- embedded 模式下 `QDRANT_LOCAL_PATH` 可写
- remote 模式下 `QDRANT_URL` 可访问
- `EMBEDDING_VECTOR_SIZE` 与现有 collection 向量维度一致

## 9. 关键文件
- `scripts/dev.sh`
- `scripts/dev-python.sh`
- `.env.example`
- `backend-python/app/main.py`
- `README.md`
