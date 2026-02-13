# 广告引擎现网维优 QA 问答系统完整方案（Internal API 推理版）

## 文档状态说明（2026-02-13）
- 当前可运行基线：`backend-python/`（FastAPI）。
- `backend/`（Rust）当前存在已知运行问题，暂不作为默认运行路径。
- 本文档保留目标架构设计（含 Rust 方案），实际启动与运维以 `README.md`、`docs/startup-manual.md` 为准。

## 1. 目标与背景
- 目标：建设一个面向广告引擎现网维优场景的中文 QA 系统，支持基于 Markdown 知识库的 RAG（检索增强生成）问答。
- 核心诉求：
  - 线上排障/维优问题可快速获取可引用答案。
  - 答案可追溯到具体文档片段，避免“黑盒结论”。 
  - 生成与向量化能力统一通过公司内部 API 提供，不再依赖本地模型推理。
- 性能目标：在线问答 P95 延迟 1-3s（在已建索引、常见问题场景下）。

## 2. 需求范围
### 2.1 In Scope
- 前端：TypeScript 实现问答交互、引用展示、反馈与系统状态。
- 后端：Rust 实现检索、提示词构建、内部 API 调用、结果编排与可观测。
- 数据：Markdown 知识库每周更新并增量重建向量索引。
- 推理：LLM 生成与 Embedding 均通过公司内部 API。

### 2.2 Out of Scope
- 多租户 RBAC 权限体系。
- 对话长期记忆（跨会话语义记忆库）。
- 流式输出（SSE）首期不实现。

## 3. 关键设计决策（已锁定）
- 模式：RAG + LLM。
- 数据源：本地 Markdown 文档目录。
- 部署：单机/VM。
- 系统入口鉴权：无鉴权（仅内网访问）。
- 推理方式：公司内部 API（OpenAI 兼容协议）。
- 上游 API 鉴权：Service Token（环境变量注入）。
- 不可用降级：返回可解释错误 + 检索来源片段，不编造答案。
- 更新频率：每周。
- 语言：中文。

## 4. 总体架构
```text
[Browser/Frontend (TS)]
          |
          v
[Backend API (Rust/Axum)]
   |          |            \
   |          |             -> [Feedback Store (SQLite/Postgres 可选)]
   |          v
   |      [Qdrant Vector DB]
   |
   -> [Company Internal API]
       |- /v1/embeddings
       |- /v1/chat/completions

[Offline Indexer (Rust Job)]
  Markdown -> Parse -> Chunk -> Embedding(API) -> Upsert Qdrant
```

## 5. 系统模块设计
### 5.1 前端（TypeScript）
- 技术栈：React + Vite + Tailwind。
- 页面与能力：
  - 问答页：输入问题、展示答案、来源片段、降级提示。
  - 历史会话：按时间查看近期提问。
  - 状态页：模型提供方、索引规模、最新索引时间、上游健康状态。
  - 反馈：有用/无用 + 可选备注。
- 与后端契约：仅调用 `/api/query`、`/api/status`、`/api/feedback`。

### 5.2 后端（Rust）
- 技术栈：axum + tokio + reqwest + serde + tracing。
- 模块划分：
  - `api`：HTTP 路由和 DTO。
  - `rag`：检索、上下文拼装、回答后处理。
  - `provider`：`InferenceProvider` trait 与 `InternalApiProvider` 实现。
  - `indexer`：Markdown 解析、切片、向量化、增量入库。
  - `observability`：日志、指标、trace_id。
  - `config`：配置加载与启动校验（fail-fast）。

### 5.3 向量检索层
- 引擎：Qdrant。
- Collection：`knowledge_chunks`。
- 向量：由内部 API embeddings 生成。
- 检索策略：TopK + score 阈值过滤 + 元数据回传。

### 5.4 内部 API 接入层
- 协议：OpenAI 兼容 JSON。
- 接口：
  - `POST /v1/embeddings`
  - `POST /v1/chat/completions`
- Header：
  - `Authorization: Bearer ${INTERNAL_API_TOKEN}`
  - `X-Request-Id: <trace_id>`
- 可靠性：超时、重试、限流、错误码映射、降级返回。

## 6. 核心数据流
### 6.1 在线问答流程
1. 前端调用 `POST /api/query`。
2. 后端调用 embedding API 生成 query 向量。
3. 在 Qdrant 检索 `top_k=6`，过滤低相关（score < 0.3）。
4. 组装上下文（控制总 token，保留来源元数据）。
5. 调用 chat API 生成中文答案。
6. 返回 `answer + sources + degraded + error_code + trace_id`。
7. 前端渲染答案、来源、降级提示（若有）。

### 6.2 离线建索引流程（每周）
1. 扫描 Markdown 目录并计算文件哈希。
2. 仅处理新增/变更文件。
3. 解析 Markdown 标题层级与正文。
4. 按策略切片并调用 embedding API。
5. Upsert 到 Qdrant；删除失效文档 chunk。
6. 写入索引作业结果（成功数、失败数、耗时）。

## 7. 文档处理与切片策略
- 输入目录：`KNOWLEDGE_DIR`（默认 `/data/knowledge`）。
- 解析规则：
  - 保留标题路径（h1/h2/h3）用于来源定位。
  - 可过滤超长代码块、无意义日志段。
- 切片规则：
  - chunk size：800-1200 中文字。
  - overlap：100-150 中文字。
- 元数据字段：
  - `doc_id`、`path`、`title_path`、`section`、`updated_at`、`hash`。

## 8. 对外 API 设计（Frontend <-> Backend）
### 8.1 `POST /api/query`
- Request
```json
{
  "question": "为什么广告请求QPS突然下降？",
  "top_k": 6
}
```
- Response
```json
{
  "answer": "可能原因包括...",
  "sources": [
    {
      "title": "竞价链路排障手册",
      "path": "docs/ops/bidding-debug.md",
      "snippet": "当QPS下降时，先看...",
      "score": 0.82
    }
  ],
  "degraded": false,
  "error_code": null,
  "trace_id": "req_20260211_xxx"
}
```

### 8.2 `GET /api/status`
- Response
```json
{
  "provider": "internal_api",
  "model": "ad-qa-chat-v1",
  "index_size": 128734,
  "last_index_time": "2026-02-10T02:10:00Z",
  "upstream_health": "ok",
  "rate_limit_state": {
    "rpm_limit": 120,
    "current_rpm": 43
  }
}
```

### 8.3 `POST /api/feedback`
- Request
```json
{
  "question": "...",
  "answer": "...",
  "rating": "useful",
  "comment": "定位很快",
  "error_code": null,
  "trace_id": "req_20260211_xxx"
}
```
- Response
```json
{
  "ok": true
}
```

### 8.4 `POST /api/reindex`
- 作用：触发离线索引任务（内网管理接口）。

## 9. 内部 API 契约与调用策略
### 9.1 Chat 调用参数
- `model=INTERNAL_API_CHAT_MODEL`（默认 `ad-qa-chat-v1`）
- `temperature=0.2`
- `max_tokens=512`
- `messages`：system + context + user。

### 9.2 Embedding 调用参数
- `model=INTERNAL_API_EMBED_MODEL`（默认 `ad-embed-v1`）
- `input`：query 或 chunk 文本。

### 9.3 稳定性策略
- 超时：
  - chat：2200ms
  - embedding：5000ms
- 重试：
  - chat：1 次（429/5xx/timeout）
  - embedding：3 次（离线任务）
- 限流：
  - 全局令牌桶，默认 `120 RPM`、`burst=10`
- 并发：
  - 对外调用并发上限 `OUTBOUND_MAX_CONCURRENCY=8`

### 9.4 错误码映射
- `UPSTREAM_TIMEOUT`
- `UPSTREAM_RATE_LIMIT`
- `UPSTREAM_AUTH`
- `UPSTREAM_UNAVAILABLE`

## 10. Prompt 与回答策略
- System Prompt 约束：
  - 必须基于提供的参考资料回答。
  - 无充分证据时明确说明“不确定”。
  - 给出排查建议时按步骤化输出。
- 回答后处理：
  - 去除模型无依据推测语句。
  - 将引用来源映射到 `sources`。

## 11. 可观测性设计
- 日志（结构化）：
  - `trace_id`、请求耗时、检索耗时、上游耗时、错误码。
- 指标：
  - `qa_qps`
  - `qa_latency_p95_ms`
  - `upstream_latency_ms`
  - `upstream_4xx_total`
  - `upstream_5xx_total`
  - `degraded_ratio`
  - `retrieval_hit_ratio`
- 追踪：
  - 前端请求 ID 贯穿后端与上游调用。

## 12. 安全与合规
- 系统入口无鉴权，但必须部署在内网并结合 IP 白名单。
- Service Token 仅通过环境变量注入，禁止写入代码与日志。
- 日志脱敏：不记录完整 prompt 与敏感字段，仅记录长度与摘要哈希。
- 网络策略：仅允许后端出网到 `INTERNAL_API_BASE_URL` 与 Qdrant。

## 13. 配置清单（最终默认值）
- `INFER_PROVIDER=internal_api`
- `INTERNAL_API_BASE_URL=<required>`
- `INTERNAL_API_TOKEN=<required>`
- `INTERNAL_API_CHAT_PATH=/v1/chat/completions`
- `INTERNAL_API_EMBED_PATH=/v1/embeddings`
- `INTERNAL_API_CHAT_MODEL=ad-qa-chat-v1`
- `INTERNAL_API_EMBED_MODEL=ad-embed-v1`
- `LLM_TIMEOUT_MS=2200`
- `EMBED_TIMEOUT_MS=5000`
- `OUTBOUND_MAX_CONCURRENCY=8`
- `CHAT_RATE_LIMIT_RPM=120`
- `CHAT_BURST=10`
- `RETRY_CHAT_MAX=1`
- `RETRY_EMBED_MAX=3`
- `KNOWLEDGE_DIR=/data/knowledge`

## 14. 部署方案（单机/VM）
- 进程/容器：
  - `frontend`
  - `backend`
  - `qdrant`
  - `nginx`（可选，反向代理）
- 不再部署本地 LLM 服务容器。
- 推荐使用 `docker-compose` 管理并配置健康检查。

## 15. 测试方案与验收标准
### 15.1 功能测试
- 正常问题返回有效答案与来源。
- 无命中问题返回“不确定”且无虚构结论。
- 反馈接口可记录结果。

### 15.2 异常测试
- 上游 401：返回 `UPSTREAM_AUTH` 且 `degraded=true`。
- 上游 429：触发重试后失败返回 `UPSTREAM_RATE_LIMIT`。
- 上游超时/5xx：返回 `UPSTREAM_TIMEOUT` 或 `UPSTREAM_UNAVAILABLE`。

### 15.3 性能测试
- 50 并发下 P95 保持 1-3s（常见问题集）。
- 索引任务在每周窗口内可完成。

### 15.4 安全测试
- 检查日志中不存在 token 和敏感全文。
- 校验仅允许内部网络访问。

### 15.5 验收门槛
- 准确性：核心 FAQ 集合命中率 >= 85%。
- 稳定性：`degraded_ratio < 3%`（稳定期）。
- 性能：在线查询 P95 满足目标。

## 16. 实施里程碑（建议 3 周）
- Week 1：
  - 后端 Provider 抽象与 Internal API 接入。
  - 完成 `/api/query`、`/api/status` 基本联调。
- Week 2：
  - 完成离线索引任务与增量更新。
  - 完成前端引用展示、错误态与反馈。
- Week 3：
  - 完成压测、灰度、监控看板与上线文档。

## 17. 风险与应对
- 风险：内部 API 限流导致高峰失败。
  - 应对：本地限流 + 退避重试 + 降级返回。
- 风险：知识库质量不稳定导致答案偏差。
  - 应对：文档规范治理 + 反馈闭环 + 周期重建。
- 风险：索引过大造成检索变慢。
  - 应对：切片优化、元数据过滤、必要时分库。

## 18. 最终结论
- 本方案在保持 MVP 快速上线的前提下，将推理能力统一迁移至公司内部 API，降低模型运维复杂度，并通过可观测、限流、降级和验收机制确保线上可用性与可迭代性。
