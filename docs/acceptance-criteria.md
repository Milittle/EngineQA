# EngineQA 验收标准（当前基线）

## 1. 验收范围
本标准覆盖当前仓库两条运行路径：
- Python 后端（`backend-python/`）+ Qdrant
- Rust 后端（`backend/`）+ LanceDB

## 2. 验收前置条件
- `.env` 已正确配置 `INTERNAL_API_BASE_URL`、`INTERNAL_API_TOKEN`
- 服务已启动（`BACKEND_RUNTIME=python make dev` 或 `BACKEND_RUNTIME=rust make dev`）
- 对应运行时向量存储配置正确

## 3. 自动化验收入口
### 3.1 Step-01 冒烟（5-10 分钟）
```bash
BACKEND_RUNTIME=python ./scripts/smoke-step-01.sh
BACKEND_RUNTIME=rust ./scripts/smoke-step-01.sh
```

### 3.2 Step-13 冒烟
```bash
./scripts/smoke-step-13.sh
```

### 3.3 验收脚本
```bash
./scripts/acceptance-test.sh
```

### 3.4 安全检查
```bash
./scripts/security-check.sh
```

### 3.5 压测
`load-test.sh` 依赖 `scripts/load-test-payload.json`，建议在 `scripts/` 目录执行：
```bash
cd scripts
./load-test.sh
```

## 4. 功能验收标准
### 4.1 健康检查
- `GET /health` 返回 `{"status":"ok"}`

### 4.2 查询接口
- `POST /api/query` 返回字段：
  - `answer`
  - `sources`
  - `degraded`
  - `error_code`
  - `trace_id`
- 正常命中时 `sources` 非空，`degraded=false`
- 无命中时 `error_code=NO_MATCH`，且不得编造答案

### 4.3 反馈接口
- `POST /api/feedback` 返回 `ok=true` 与 `id`

### 4.4 状态接口
- `GET /api/status` 返回：
  - `provider`
  - `model`
  - `index_size`
  - `rate_limit_state`
  - Python 路径：`qdrant_connected`
  - Rust 路径：`vector_store`、`vector_store_connected`（兼容字段 `qdrant_connected` 仍可存在）

### 4.5 索引接口
- `POST /api/reindex` 可触发任务并返回 `job_id`
- `GET /api/reindex` 可查看任务状态（`running/completed/failed`）

## 5. 可靠性与降级验收
- 上游异常（401/429/timeout/5xx）时：
  - 接口仍返回可解释结果
  - `degraded=true`
  - `error_code` 符合映射规则（如 `UPSTREAM_AUTH`、`UPSTREAM_TIMEOUT`）

- 单次任务异常不得导致服务进程退出。

## 6. 性能验收（目标）
- 在线查询 P95：1-3s（常见问题集）
- 压测时关注：
  - P95 是否 <= 3000ms
  - 错误率与 `degraded` 比例是否可接受

说明：性能结果受上游 Internal API 与知识库规模影响，应在稳定网络和固定测试集下评估。

## 7. 安全验收
- 不提交 `.env` 与真实 Token
- 日志中不泄露 `INTERNAL_API_TOKEN`
- 代码中无硬编码密钥

## 8. 人工抽检建议
- 抽样 20-50 个核心 FAQ 问题
- 记录命中率与可用性
- 重点检查：
  - 答案是否引用来源
  - 无依据时是否明确“不确定”
  - 排障建议是否可执行

## 9. 通过门槛（建议）
- 功能接口全部通过
- 安全检查通过
- 压测 P95 达标或有可解释偏差说明
- FAQ 命中率 >= 85%（人工评审）
