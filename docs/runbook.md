# EngineQA Runbook

## 1. 当前运行基线
- 前端：`frontend`（Vite dev，`5173`）
- 后端：`backend-python`（FastAPI，`8080`）
- 向量库：Qdrant embedded（默认）或 remote（可选）
- 上游：Internal API（chat + embedding）

说明：Rust 后端当前有已知运行问题，本文不覆盖 Rust 线上排障路径。

## 2. 快速健康检查
```bash
curl -fsS http://127.0.0.1:8080/health
curl -fsS http://127.0.0.1:8080/api/status
curl -fsS http://127.0.0.1:5173 >/dev/null
```

可选冒烟：
```bash
SKIP_QDRANT=1 ./scripts/smoke-step-01.sh
./scripts/smoke-step-13.sh
```

## 3. 常见故障排查
### 3.1 后端无法启动
检查项：
- `.env` 中是否有 `INTERNAL_API_BASE_URL`、`INTERNAL_API_TOKEN`
- Python 依赖是否已安装（`backend-python/requirements.txt`）
- `QDRANT_MODE` 是否为 `embedded` 或 `remote`

快速验证：
```bash
python3 -c "import fastapi,uvicorn,httpx,qdrant_client,dotenv"
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
- 上游 embedding 能否正常返回
- `EMBEDDING_VECTOR_SIZE` 与模型维度是否一致

任务状态：
```bash
curl -s http://127.0.0.1:8080/api/reindex
```

触发重建：
```bash
curl -s -X POST http://127.0.0.1:8080/api/reindex \
  -H 'Content-Type: application/json' \
  -d '{}'
```

### 3.4 Qdrant 连接异常
embedded 模式：
- 检查 `QDRANT_LOCAL_PATH` 目录权限与磁盘空间

remote 模式：
- 检查 `QDRANT_URL` 可达性
- 检查远端 Qdrant `/healthz`

### 3.5 前端无法访问或接口报错
检查：
- 前端服务是否在 `5173`
- `VITE_API_BASE_URL` 是否指向正确后端地址
- 浏览器网络请求是否命中 `http://localhost:8080`

## 4. 日志关键字
Python 后端关键日志（`backend-python/app`）：
- `query_embed_failed`
- `query_chat_failed`
- `upstream_embed_failed`
- `upstream_chat_failed`
- `reindex_failed`

建议使用：
```bash
# 如果在前台启动，直接看终端输出
# 若重定向到文件：
rg "query_|upstream_|reindex_" <your_log_file>
```

## 5. 应急动作
### 5.1 上游故障时
- 保持服务在线，允许降级响应
- 暂停高频批量请求
- 在状态页关注 `rate_limit_state` 与错误码分布

### 5.2 索引损坏或配置错误
- 修正 `.env` 中向量维度与 Qdrant 参数
- 重新触发 `/api/reindex`
- 验证 `index_size` 是否恢复增长

### 5.3 快速回滚
- 使用 Git 回滚最近一次提交：
```bash
git revert <commit_sha>
```
- 回滚后重新执行启动和冒烟检查

## 6. 日常巡检建议
- 每日：`/health` 与 `/api/status`
- 每周：重建索引并抽样验证问答质量
- 每次变更后：执行 `scripts/smoke-step-01.sh`、`scripts/security-check.sh`
