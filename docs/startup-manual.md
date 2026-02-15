# EngineQA 启动手册

## 1. 适用范围
本手册用于本机（Host-Run）启动 EngineQA。

日期基线：2026-02-14。

支持两种后端运行时：
- `backend-python/`（FastAPI + Qdrant）
- `backend/`（Axum + LanceDB）

## 2. 前置条件
- Node.js + npm
- `curl`

Python 路径需要：
- Python 3.10+

Rust 路径需要：
- Rust toolchain

## 3. 初始化
```bash
npm install --prefix frontend
cp .env.example .env
```

Python 依赖：
```bash
python3 -m venv .venv-backend-python
.venv-backend-python/bin/pip install -r backend-python/requirements.txt
```

## 4. 环境变量
必须配置：
- `INTERNAL_API_BASE_URL`
- `INTERNAL_API_TOKEN`

通用建议：
- `EMBEDDING_VECTOR_SIZE` 与 embedding 模型维度一致
- `INTERNAL_API_CHAT_PATH` / `INTERNAL_API_EMBED_PATH` 与上游一致

Python + Qdrant：
- `BACKEND_RUNTIME=python`
- `QDRANT_LOCAL_PATH=./.qdrant-local`
- `QDRANT_COLLECTION=knowledge_chunks`

Rust + LanceDB：
- `BACKEND_RUNTIME=rust`
- `VECTOR_STORE=lancedb`
- `LANCEDB_URI=./.lancedb`
- `LANCEDB_TABLE=knowledge_chunks`
- `VECTOR_SCORE_THRESHOLD=0.3`

## 5. 启动方式
统一入口（默认 Python）：
```bash
make dev
```

显式指定 Python：
```bash
BACKEND_RUNTIME=python make dev
```

显式指定 Rust：
```bash
BACKEND_RUNTIME=rust make dev
```

## 6. 启动后验证
```bash
curl -fsS http://127.0.0.1:8080/health
curl -fsS http://127.0.0.1:8080/api/status
curl -fsS http://127.0.0.1:5173 >/dev/null
```

基础冒烟：
```bash
BACKEND_RUNTIME=python ./scripts/smoke-step-01.sh
BACKEND_RUNTIME=rust ./scripts/smoke-step-01.sh
```

## 7. 向量存储模式说明
### 7.1 Python（Qdrant）
- embedded（默认且唯一支持）：`QDRANT_LOCAL_PATH=./.qdrant-local`

### 7.2 Rust（LanceDB）
- 使用本地目录，不需要独立向量数据库进程
- 数据目录：`LANCEDB_URI`（默认 `./.lancedb`）
- 表名：`LANCEDB_TABLE`（默认 `knowledge_chunks`）

## 8. 常见问题
### 8.1 Python 依赖缺失
```bash
.venv-backend-python/bin/pip install -r backend-python/requirements.txt
```

### 8.2 Rust 构建失败
```bash
cargo check --manifest-path backend/Cargo.toml
```

### 8.3 启动时报缺少必填环境变量
检查 `.env` 是否包含：
- `INTERNAL_API_BASE_URL`
- `INTERNAL_API_TOKEN`

### 8.4 查询返回 `UPSTREAM_*`
优先检查：
- Token 是否有效
- chat/embed 地址和 path 是否正确
- 上游服务可达性

### 8.5 检索失败或向量存储连接失败
Python 路径检查：
- `QDRANT_LOCAL_PATH` 可写

Rust 路径检查：
- `LANCEDB_URI` 目录可写
- `EMBEDDING_VECTOR_SIZE` 与历史索引向量维度一致

## 9. 关键文件
- `scripts/dev.sh`
- `scripts/dev-python.sh`
- `scripts/dev-rust.sh`
- `scripts/smoke-step-01.sh`
- `.env.example`
- `README.md`
