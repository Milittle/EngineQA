# EngineQA 模块化交付步骤（steps.md）

## 文档状态说明（2026-02-14）
- 当前可运行基线为 Python 后端（`backend-python/`）。
- Rust 后端已完成向量存储重构，向量层使用 LanceDB。
- 实际运行流程以 `README.md` 和 `docs/startup-manual.md` 为准。

## 1. 执行原则（保证可快速验证与小步迭代）
- 每个 Step 只交付一个主模块能力，避免跨模块大改。
- 每个 Step 必须满足：可运行、可验证、可回滚（`git revert`）。
- 每个 Step 完成后立即提交一次，提交信息绑定 Step 编号。
- 每个 Step 提供 5-10 分钟可完成的冒烟验证。

## 2. Git 迭代约定
- 分支建议：`feature/engineqa-rag-mvp`
- Commit 模板：`feat(step-XX): <模块能力>`
- 推荐节奏：
  1. 完成 Step 改动
  2. 本地验证通过
  3. `git add -A && git commit -m "feat(step-XX): ..."`
  4. 可选打标签：`git tag step-XX`
- 回滚方式：`git revert <commit_sha>`（不改历史，便于协作）

## 3. Step-by-Step 交付清单

### Step-01：项目骨架与本地运行基线
- 目标：搭建前后端与基础目录，确保一条命令可启动开发环境。
- 模块：
  - `frontend`（React + Vite + Tailwind）
  - `backend`（axum 基础服务）
  - `deploy`（主机运行脚本与配置模板）
- 交付物：
  - 目录结构、启动脚本、`.env.example`
  - 基础 README（启动方式与端口约定）
- 快速验证：
  - 前端页面可访问
  - 后端 `/health` 返回 200
  - 向量存储配置可被正确加载（Python/Qdrant 或 Rust/LanceDB）
- 建议 Commit：
  - `feat(step-01): bootstrap frontend backend and vector store runtime`

### Step-02：配置中心与启动 fail-fast
- 目标：完成配置加载与必填项校验，避免运行中才暴露配置错误。
- 模块：
  - `backend/config`
- 交付物：
  - 环境变量映射（对应 `plan.md` 第 13 节）
  - 缺失 `INTERNAL_API_BASE_URL`/`INTERNAL_API_TOKEN` 时启动失败并报错
- 快速验证：
  - 缺少必填 env 启动失败
  - 配置完整时服务正常启动
- 建议 Commit：
  - `feat(step-02): add config loader and fail-fast validation`

### Step-03：内部 API Provider（Embedding + Chat）
- 目标：打通公司内部 API 调用链路（OpenAI 兼容协议）。
- 模块：
  - `backend/provider`
- 交付物：
  - `InferenceProvider` trait
  - `InternalApiProvider`（`/v1/embeddings`、`/v1/chat/completions`）
  - Header 注入：`Authorization`、`X-Request-Id`
  - 超时/重试/限流基础能力
- 快速验证：
  - mock 或沙箱环境下 embedding/chat 请求成功
  - 401/429/timeout 能映射为内部错误类型
- 建议 Commit：
  - `feat(step-03): implement internal api provider with retries and timeout`

### Step-04：LanceDB 检索模块
- 目标：实现向量检索与来源元数据返回。
- 模块：
  - `backend/rag/retriever`
- 交付物：
  - `knowledge_chunks` table 访问封装
  - `top_k` 检索 + `score >= 0.3` 过滤
  - 返回 `title/path/snippet/score` 所需字段
- 快速验证：
  - 写入测试向量后可查询到预期 chunk
  - 低分结果会被过滤
- 建议 Commit：
  - `feat(step-04): add lancedb retrieval with score threshold filtering`

### Step-05：`/api/query` 最小可用 RAG 链路
- 目标：上线核心问答接口，先实现“可回答、可引用、可降级”。
- 模块：
  - `backend/api/query`
  - `backend/rag`
- 交付物：
  - 流程：query embedding -> lancedb 检索 -> context 组装 -> chat 生成
  - 返回：`answer/sources/degraded/error_code/trace_id`
  - 无命中时返回“不确定”，避免编造
- 快速验证：
  - 正常问题可返回答案 + sources
  - 无命中问题返回 `degraded=true` 或明确不确定
- 建议 Commit：
  - `feat(step-05): implement /api/query rag pipeline with source attribution`

### Step-06：错误码映射与降级策略固化
- 目标：将上游异常统一映射为业务可解释错误码。
- 模块：
  - `backend/provider/errors`
  - `backend/api/error_mapping`
- 交付物：
  - `UPSTREAM_TIMEOUT`
  - `UPSTREAM_RATE_LIMIT`
  - `UPSTREAM_AUTH`
  - `UPSTREAM_UNAVAILABLE`
  - 降级时返回检索片段（若可用）
- 快速验证：
  - 模拟 401/429/5xx/timeout，各自返回预期 `error_code`
- 建议 Commit：
  - `feat(step-06): standardize upstream error mapping and degrade response`

### Step-07：离线索引器（增量构建）
- 目标：实现 Markdown -> chunk -> embedding -> upsert 的每周增量流程。
- 模块：
  - `backend/indexer`
- 交付物：
  - 文件扫描 + hash 比对（新增/变更/删除）
  - 标题层级解析与切片（800-1200，overlap 100-150）
  - upsert 新 chunk + 删除失效 chunk
- 快速验证：
  - 首次全量建索引成功
  - 修改单个 md 后仅增量更新对应文档
- 建议 Commit：
  - `feat(step-07): build incremental markdown indexer for lancedb`

### Step-08：`/api/reindex` 与索引任务状态
- 目标：提供可运维的索引触发入口与结果可见性。
- 模块：
  - `backend/api/reindex`
  - `backend/indexer/job_state`
- 交付物：
  - `POST /api/reindex` 触发任务（支持 `full=true/false`）
  - 返回任务结果：成功数/失败数/耗时/时间戳
- 快速验证：
  - 调用接口后能触发并观察到任务完成状态
- 建议 Commit：
  - `feat(step-08): expose reindex api and job result reporting`

### Step-09：`/api/status` + 可观测基础
- 目标：给前端和运维提供系统状态与健康信息。
- 模块：
  - `backend/api/status`
  - `backend/observability`
- 交付物：
  - `provider/model/index_size/last_index_time/upstream_health/rate_limit_state`
  - 结构化日志：`trace_id`、总耗时、检索耗时、上游耗时
- 快速验证：
  - `GET /api/status` 字段完整
  - 日志中可串联一次请求全链路 trace_id
- 建议 Commit：
  - `feat(step-09): add status endpoint and structured observability baseline`

### Step-10：反馈闭环（`/api/feedback`）
- 目标：完成反馈采集，为后续知识库优化提供数据。
- 模块：
  - `backend/api/feedback`
  - `backend/store`（SQLite 或 Postgres 二选一）
- 交付物：
  - `POST /api/feedback` 入库
  - 字段：question/answer/rating/comment/error_code/trace_id
- 快速验证：
  - 提交 useful/useless 均可持久化
- 建议 Commit：
  - `feat(step-10): implement feedback api and persistence`

### Step-11：前端问答页（MVP）
- 目标：交付可用问答页面，支持答案与引用展示。
- 模块：
  - `frontend/pages/qa`
- 交付物：
  - 输入问题、展示答案
  - sources 引用列表（title/path/snippet/score）
  - degraded/error_code 提示态
- 快速验证：
  - 页面可发起查询并正确渲染三种状态：成功/无命中/上游异常
- 建议 Commit：
  - `feat(step-11): deliver qa page with answer source and degraded states`

### Step-12：前端状态页 + 反馈交互 + 历史会话
- 目标：补齐产品闭环能力，便于巡检与质量改进。
- 模块：
  - `frontend/pages/status`
  - `frontend/components/feedback`
  - `frontend/pages/history`
- 交付物：
  - 状态页：展示 `/api/status`
  - 反馈交互：有用/无用 + 备注
  - 历史会话（本地存储即可，首期不做跨端同步）
- 快速验证：
  - 状态页数据刷新正常
  - 反馈提交成功
  - 历史提问可回看
- 建议 Commit：
  - `feat(step-12): add status history and feedback interaction in frontend`

### Step-13：测试、压测与上线前验收
- 目标：对齐 `plan.md` 第 15 节验收门槛，形成上线基线。
- 模块：
  - `tests`、`scripts`、`docs`
- 交付物：
  - 功能/异常测试用例
  - 50 并发压测脚本与结果
  - 安全检查（日志脱敏、token 不落盘）
- 快速验证：
  - 关键用例通过
  - P95 达到 1-3s（常见问题集）
  - degraded_ratio 和命中率达到目标
- 建议 Commit：
  - `test(step-13): add acceptance tests load tests and security checks`

### Step-14：部署与灰度文档
- 目标：沉淀可复制部署方案，支持稳定上线与回滚。
- 模块：
  - `deploy/`
  - `docs/runbook.md`
- 交付物：
  - 单机/VM 部署说明（frontend/backend/lancedb/nginx 可选）
  - 灰度策略、回滚策略、常见故障排查
- 快速验证：
  - 新机器按文档可在 30 分钟内完成部署
- 建议 Commit：
  - `docs(step-14): finalize deployment runbook and rollout guide`

## 4. 建议验收节奏（对应 3 周）
- Week 1：Step-01 ~ Step-06（后端在线链路闭环）
- Week 2：Step-07 ~ Step-10（索引与反馈闭环）
- Week 3：Step-11 ~ Step-14（前端完善、压测、上线文档）

## 5. 每步完成 Definition of Done（统一标准）
- 代码：主功能完成且无明显临时占位逻辑。
- 测试：至少 1 条该模块的自动化测试或可复现冒烟脚本。
- 文档：补充接口或运行说明变更点。
- 交付：有独立 commit，可单独回滚，不影响已完成 Step。
