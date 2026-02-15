# EngineQA Runbook

## 1. 运行基线
日期基线：2026-02-14。

支持两条运行路径：
- Python 后端：`backend-python`（FastAPI + Qdrant）
- Rust 后端：`backend`（Axum + LanceDB）

通用：
- 前端：`frontend`（Vite dev，`5173`）
- 后端：`8080`
- 上游：Internal API（chat + embedding）

## 2. 快速健康检查
```bash
curl -fsS http://127.0.0.1:8080/health
curl -fsS http://127.0.0.1:8080/api/status
curl -fsS http://127.0.0.1:5173 >/dev/null
```

可选冒烟：
```bash
BACKEND_RUNTIME=python ./scripts/smoke-step-01.sh
BACKEND_RUNTIME=rust ./scripts/smoke-step-01.sh
./scripts/smoke-step-13.sh
```

## 3. 常见故障排查
### 3.1 后端无法启动
检查项：
- `.env` 是否包含 `INTERNAL_API_BASE_URL`、`INTERNAL_API_TOKEN`
- 运行时依赖是否就绪（Python 依赖或 Rust toolchain）
- 运行时向量存储配置是否匹配

快速验证：
```bash
# Python 运行时
python3 -c "import fastapi,uvicorn,httpx,qdrant_client,dotenv"

# Rust 运行时
cargo check --manifest-path backend/Cargo.toml
```

### 3.2 查询请求全部降级（`degraded=true`）
先看响应 `error_code`：
- `UPSTREAM_AUTH`: token 无效或权限不足
- `UPSTREAM_TIMEOUT`: 上游超时
- `UPSTREAM_RATE_LIMIT`: 上游限流
- `UPSTREAM_UNAVAILABLE`: 上游不可达或 5xx
- `NO_MATCH`: 知识库无相关内容

排查命令：
```bash
curl -s http://127.0.0.1:8080/api/status
```

### 3.3 索引任务失败
检查：
- `KNOWLEDGE_DIR` 是否存在且包含 `.md`
- 上游 embedding 是否可用
- `EMBEDDING_VECTOR_SIZE` 与模型维度是否一致

任务状态：
```bash
curl -s http://127.0.0.1:8080/api/reindex
```

触发重建（建议全量）：
```bash
curl -s -X POST http://127.0.0.1:8080/api/reindex \
  -H 'Content-Type: application/json' \
  -d '{"full": true}'
```

### 3.4 向量存储连接异常
Python 路径（Qdrant）：
- 检查 `QDRANT_LOCAL_PATH` 目录权限与磁盘空间
- 检查 `QDRANT_COLLECTION` 与实际集合是否一致

Rust 路径（LanceDB）：
- 检查 `LANCEDB_URI` 目录权限与磁盘空间
- 检查 `LANCEDB_TABLE` 名称是否与配置一致
- 检查向量维度配置 `EMBEDDING_VECTOR_SIZE`

### 3.5 前端无法访问或接口报错
检查：
- 前端服务是否在 `5173`
- `VITE_API_BASE_URL` 是否指向正确后端
- 浏览器网络请求是否命中 `http://localhost:8080`

## 4. 日志关键字
Python 后端（`backend-python/app`）：
- `query_embed_failed`
- `query_chat_failed`
- `upstream_embed_failed`
- `upstream_chat_failed`
- `reindex_failed`

Rust 后端（`backend/src`）：
- `received query request`
- `embedding failed`
- `retrieval failed`
- `started reindex job`
- `reindex job completed`

建议使用：
```bash
rg "query_|upstream_|reindex_|retrieval" <your_log_file>
```

## 5. 应急动作
### 5.1 上游故障时
- 保持服务在线，允许降级响应
- 暂停高频批量请求
- 关注 `rate_limit_state` 与错误码分布

### 5.2 索引损坏或配置错误
- 修正 `.env` 中向量存储参数与向量维度
- 重新触发 `/api/reindex`
- 验证 `index_size` 恢复增长

### 5.3 快速回滚
```bash
git revert <commit_sha>
```

回滚后重新执行启动和冒烟检查。

## 6. 日常巡检建议
- 每日：`/health` 与 `/api/status`
- 每周：重建索引并抽样验证问答质量
- 每次变更后：`scripts/smoke-step-01.sh`、`scripts/security-check.sh`
