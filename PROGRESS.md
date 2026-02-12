# EngineQA 开发进度

## 已完成的 Steps

### ✅ Step-01: 项目骨架与本地运行基线
- Frontend (React + Vite + Tailwind)
- Backend (Axum 基础服务)
- Qdrant Docker Compose 配置
- 启动脚本和冒烟测试

**Commit**: `30a05c7`

---

### ✅ Step-02: 配置中心与启动 fail-fast
- 环境变量映射（对应 plan.md 第 13 节）
- 必填项校验（`INTERNAL_API_BASE_URL`、`INTERNAL_API_TOKEN`）
- 启动失败时清晰错误提示
- 完整的测试用例

**Commit**: `30a05c7`

---

### ✅ Step-03: 内部 API Provider（Embedding + Chat）
- `InferenceProvider` trait 定义
- `InternalApiProvider` 实现（OpenAI 兼容协议）
- Header 注入（Authorization、X-Request-Id）
- 超时/重试机制
- 支持异步调用

**Commit**: `3927422`

---

### ✅ Step-04: Qdrant 检索模块
- `VectorRetriever` 实现
- `knowledge_chunks` collection 封装
- top_k 检索 + score >= 0.3 过滤
- 返回 title/path/snippet/score 字段
- 自动创建 collection

**Commit**: `150f517`

---

### ✅ Step-05: `/api/query` 最小可用 RAG 链路
- 完整的 RAG pipeline
- query embedding → qdrant 检索 → context 组装 → chat 生成
- 返回 answer/sources/degraded/error_code/trace_id
- 无命中时返回"不确定"，避免编造
- 降级策略和错误处理

**Commit**: `b677b62`

---

### ✅ Step-06: 错误码映射与降级策略固化
- `ErrorCode` 枚举定义（8 种错误类型）
- HTTP 状态码映射到业务错误码
- `should_degrade` 判断逻辑
- 可读的错误描述
- 降级时返回检索片段

**Commit**: `3a79eef`

---

### ✅ Step-07: 离线索引器（增量构建）
- Markdown 文件扫描与 hash 比对
- 标题层级解析与切片（800-1200 字符，overlap 100-150）
- 增量更新：仅处理新增/变更文件
- 删除失效文档 chunk
- 并发 embedding（最多 8 并发）
- SHA256 hash 计算

**Commit**: `fa8c9af`

---

### ✅ Step-08: `/api/reindex` 与索引任务状态
- POST /api/reindex 触发索引任务
- GET /api/reindex 查询任务状态
- JobManager 管理（Running/Completed/Failed）
- 异步后台任务
- 返回详细索引结果（成功数、失败数、耗时）

**Commit**: `978aae6`

---

### ✅ Step-09: `/api/status` + 可观测基础
- 推理服务健康状态
- 知识库索引规模和最后索引时间
- 速率限制状态
- Qdrant 连接状态
- 结构化日志：trace_id、请求耗时、检索耗时、上游耗时

**Commit**: `5eed187`

---

### ✅ Step-10: 反馈闭环（`/api/feedback`）
- POST /api/feedback 入库
- FeedbackRating (useful/useless)
- 内存存储（生产环境应替换为数据库）
- 字段：question/answer/rating/comment/error_code/trace_id

**Commit**: `4f89513`

---

### ✅ Step-11: 前端问答页（MVP）
- 输入问题并获取答案
- 显示参考来源和相关度
- 降级模式提示
- 反馈交互（有用/无用）
- 自动保存到历史记录（localStorage）

**Commit**: `cb58b44`

---

### ✅ Step-12: 前端状态页 + 反馈交互 + 历史会话
- 状态页：
  - 推理服务健康状态
  - 知识库索引规模
  - 速率限制状态
  - 触发重新索引
  - 查看索引任务结果
- 历史页：
  - 查看最近的问答历史
  - 展开/收起答案
  - 删除记录
  - 清空所有记录
  - 本地存储
- 导航栏（问答/历史/状态）

**Commit**: `cb58b44`

---

## 待完成的 Steps

### ⏳ Step-13: 测试、压测与上线前验收
- 功能/异常测试用例
- 50 并发压测脚本与结果
- 安全检查（日志脱敏、token 不落盘）
- 准确性：核心 FAQ 集合命中率 >= 85%
- 稳定性：degraded_ratio < 3%
- 性能：在线查询 P95 满足 1-3s

### ⏳ Step-14: 部署与灰度文档
- 单机/VM 部署说明
- Docker Compose 配置
- 灰度策略
- 回滚策略
- 常见故障排查
- Runbook 文档

---

## Git Tags

已为每个完成的 Step 创建 tag：

```bash
git tag
# step-01
# step-02
# ...
# step-12
```

查看特定 Step：

```bash
git show step-05
git diff step-04..step-05
```

---

## 技术栈

**后端：**
- Rust 2024
- Axum 0.8
- Tokio
- Reqwest
- Qdrant Client
- Chrono
- SHA2

**前端：**
- React 18
- Vite
- TypeScript
- Tailwind CSS

**数据库：**
- Qdrant (向量数据库)

**推理：**
- 公司内部 API（OpenAI 兼容协议）

---

## 下一步

继续完成 Step-13（测试、压测与上线前验收）和 Step-14（部署与灰度文档）。
