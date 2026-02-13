# EngineQA 验收标准文档

## 验收门槛

根据 `plan.md` 第 15 节，以下是上线前必须满足的验收标准。

---

## 1. 准确性验收

### 1.1 核心FAQ集合命中率 ≥ 85%

**测试方法：**
1. 准备核心FAQ问题集（建议20-50个常见问题）
2. 逐一提交到 `/api/query` 接口
3. 人工或自动评估答案质量
4. 计算命中率（有效答案数 / 总问题数）

**有效答案标准：**
- 答案直接回答了问题
- 答案基于知识库内容，非编造
- 答案准确且可操作（如适用）

**验收命令：**
```bash
./scripts/acceptance-test.sh
```

**手动评估表：**
| 问题ID | 问题 | 是否命中 | 答案质量 | 备注 |
|--------|------|----------|----------|------|
| 1 | 为什么QPS下降？ | ✓ | 高 | - |
| 2 | 广告请求超时怎么办？ | ✓ | 中 | 需要更详细步骤 |
| ... | ... | ... | ... | ... |

**计算：**
```
命中率 = (命中数 / 总问题数) × 100%
目标：命中率 ≥ 85%
```

---

## 2. 稳定性验收

### 2.1 Degraded Ratio < 3%

**测试方法：**
1. 在稳定负载下运行系统（如50 RPM）
2. 记录1000次查询请求
3. 统计 degraded=true 的响应数
4. 计算降级比例

**验收命令：**
```bash
# 运行稳定性测试
./scripts/stability-test.sh
```

**计算：**
```
degraded_ratio = (degraded响应数 / 总响应数) × 100%
目标：degraded_ratio < 3%
```

**正常范围：**
- 初期（上线1周内）：degraded_ratio < 5%
- 稳定后：degraded_ratio < 3%

---

## 3. 性能验收

### 3.1 在线查询 P95 满足 1-3s

**测试方法：**
1. 准备常见问题集（10-20个）
2. 使用50并发进行压测
3. 收集P95、P99延迟数据
4. 验证是否满足目标

**验收命令：**
```bash
# 运行压测
./scripts/load-test.sh
```

**性能目标：**
| 指标 | 目标 | 说明 |
|------|------|------|
| P50 | < 1s | 中位延迟 |
| P95 | 1-3s | 95分位延迟 |
| P99 | < 5s | 99分位延迟 |
| RPS | ≥ 50 | 每秒请求数 |

**测试场景：**
- 场景1：缓存命中（常见问题）
- 场景2：正常查询（一般问题）
- 场景3：冷启动（首次查询）

---

## 4. 功能验收

### 4.1 正常问题返回有效答案与来源

**测试用例：**
```bash
# 测试用例1：正常查询
curl -X POST http://localhost:8080/api/query \
  -H "Content-Type: application/json" \
  -d '{
    "question": "为什么广告请求QPS突然下降？",
    "top_k": 6
  }'

# 验证点：
# - status: 200
# - answer 非空
# - sources 数量 >= 1
# - degraded: false
# - error_code: null
```

**预期结果：**
- 返回结构化答案
- 包含参考来源
- sources 中每个元素包含：title, path, snippet, score
- score >= 0.3

---

### 4.2 无命中问题返回"不确定"且无虚构结论

**测试用例：**
```bash
# 测试用例2：无相关内容的问题
curl -X POST http://localhost:8080/api/query \
  -H "Content-Type: application/json" \
  -d '{
    "question": "如何使用区块链技术优化广告系统？",
    "top_k": 6
  }'

# 验证点：
# - status: 200
# - answer 包含"不确定"或"未找到"
# - sources 数量 = 0
# - degraded: true
# - error_code: "NO_MATCH"
```

**预期结果：**
- 不编造答案
- 明确说明知识库中没有相关信息
- 建议用户尝试其他问题或联系技术团队

---

### 4.3 反馈接口可记录结果

**测试用例：**
```bash
# 测试用例3：提交反馈
curl -X POST http://localhost:8080/api/feedback \
  -H "Content-Type: application/json" \
  -d '{
    "question": "为什么QPS下降？",
    "answer": "可能原因包括...",
    "rating": "useful",
    "comment": "定位很快",
    "error_code": null,
    "trace_id": "req_test_123"
  }'

# 验证点：
# - status: 200
# - ok: true
# - id 非空
```

---

## 5. 异常验收

### 5.1 上游401返回UPSTREAM_AUTH且degraded=true

**测试方法：**
1. 使用错误的 API Token
2. 提交查询请求
3. 验证错误响应

**测试用例：**
```bash
# 修改 INTERNAL_API_TOKEN 为无效值
export INTERNAL_API_TOKEN="invalid_token"

# 发起查询
curl -X POST http://localhost:8080/api/query \
  -H "Content-Type: application/json" \
  -d '{"question": "测试", "top_k": 6}'

# 验证点：
# - error_code: "UPSTREAM_AUTH"
# - degraded: true
# - answer 包含"认证失败"相关描述
```

---

### 5.2 上游429触发重试后失败返回UPSTREAM_RATE_LIMIT

**测试方法：**
1. 在短时间内发送大量请求（超过速率限制）
2. 验证触发限流后的错误响应

**测试用例：**
```bash
# 发送大量请求
for i in {1..200}; do
  curl -X POST http://localhost:8080/api/query \
    -H "Content-Type: application/json" \
    -d '{"question": "测试 '$i'", "top_k": 6}' &
done
wait

# 验证点：
# - 部分请求返回 error_code: "UPSTREAM_RATE_LIMIT"
# - degraded: true
# - answer 包含"限流"相关描述
```

---

### 5.3 上游超时/5xx返回UPSTREAM_TIMEOUT或UPSTREAM_UNAVAILABLE

**测试方法：**
1. 临时配置超时时间为极短值
2. 发起查询请求
3. 验证超时错误响应

**测试用例：**
```bash
# 修改超时配置
export LLM_TIMEOUT_MS=1

# 发起查询
curl -X POST http://localhost:8080/api/query \
  -H "Content-Type: application/json" \
  -d '{"question": "测试", "top_k": 6}'

# 验证点：
# - error_code: "UPSTREAM_TIMEOUT"
# - degraded: true
# - answer 包含"超时"相关描述
```

---

## 6. 安全验收

### 6.1 日志脱敏

**检查项目：**
- ❌ 日志中不包含完整的 API Token
- ❌ 日志中不包含完整的 prompt（仅记录长度或哈希）
- ❌ 日志中不包含用户敏感信息

**验收命令：**
```bash
# 检查日志
./scripts/security-check.sh
```

**手动检查：**
```bash
# 搜索日志中的 token
grep -r "sk-" logs/ backend/src/

# 搜索日志中的敏感信息
grep -r "password\|secret\|token" logs/ backend/src/ --exclude-dir=node_modules
```

---

### 6.2 Token 不落盘

**检查项目：**
- ❌ .env 文件不被 git 追踪
- ❌ 配置文件中不包含硬编码的 token
- ❌ 源代码中不包含 token

**验收命令：**
```bash
# 检查 .env 是否被追踪
git ls-files | grep "^\.env$"

# 检查硬编码的 token
grep -r "sk-" backend/src/
```

---

### 6.3 网络策略

**检查项目：**
- ✅ 后端仅允许出网到 INTERNAL_API_BASE_URL
- ✅ 后端仅允许出网到 Qdrant URL
- ✅ 前端仅允许访问后端 API

**验证方法：**
```bash
# 检查防火墙规则（如适用）
sudo iptables -L -n

# 检查网络连接
netstat -tlnp | grep 8080
```

---

## 7. 上线前检查清单

### 7.1 配置检查
- [ ] INTERNAL_API_BASE_URL 已配置且正确
- [ ] INTERNAL_API_TOKEN 已配置且有效
- [ ] QDRANT_URL 已配置
- [ ] KNOWLEDGE_DIR 已配置且包含文档
- [ ] .env 文件不被 git 追踪

### 7.2 服务检查
- [ ] Backend 服务运行正常
- [ ] Frontend 服务运行正常
- [ ] Qdrant 服务运行正常
- [ ] Nginx（如使用）配置正确

### 7.3 功能检查
- [ ] /health 返回 ok
- [ ] /api/status 返回正确信息
- [ ] /api/query 可以正常查询
- [ ] /api/feedback 可以提交反馈
- [ ] /api/reindex 可以触发索引

### 7.4 性能检查
- [ ] P95 延迟满足 1-3s 要求
- [ ] 50 并发下系统稳定
- [ ] degraded_ratio < 3%

### 7.5 安全检查
- [ ] 日志脱敏检查通过
- [ ] Token 不落盘检查通过
- [ ] 无硬编码敏感信息

### 7.6 文档检查
- [ ] 部署文档完整
- [ ] Runbook 文档完整
- [ ] 故障排查文档完整
- [ ] API 文档完整

---

## 8. 验收通过标准

**项目满足以下条件方可上线：**

1. ✅ 核心FAQ集合命中率 ≥ 85%
2. ✅ degraded_ratio < 3%
3. ✅ P95 延迟 1-3s
4. ✅ 所有功能测试用例通过
5. ✅ 所有异常测试用例通过
6. ✅ 所有安全检查通过
7. ✅ 上线前检查清单全部完成

---

## 9. 验收不通过的处理

**如果某个验收项不通过：**

1. **记录问题**：在 `ACCEPTANCE_ISSUES.md` 中记录
2. **分析原因**：定位问题的根本原因
3. **制定方案**：制定修复方案和时间表
4. **实施修复**：按计划修复问题
5. **重新验收**：修复后重新运行验收测试

**问题记录模板：**
```markdown
## 验收问题记录

| ID | 验收项 | 状态 | 问题描述 | 影响范围 | 修复方案 | 优先级 |
|----|--------|------|----------|----------|----------|--------|
| 1 | 准确性 | ❌ | FAQ命中率 70%，未达85% | 知识库质量 | 优化文档质量 | 高 |
| 2 | 性能 | ❌ | P95 延迟 4s，超3s目标 | 用户体验 | 优化检索策略 | 中 |
```
