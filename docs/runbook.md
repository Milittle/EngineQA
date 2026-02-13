# EngineQA Runbook

## 概述

本 Runbook 提供了 EngineQA 系统的运维手册，包括常见问题的排查步骤、监控指标、应急预案等。

---

## 1. 系统架构

```
┌─────────────────────────────────────────────────────┐
│                    用户访问                          │
│                  (Browser/API)                    │
└──────────────────┬────────────────────────────────┘
                   │
        ┌──────────┼──────────┐
        │                     │
┌───────▼────────┐  ┌──────▼────────┐
│   Frontend     │  │   Backend     │
│   (React)      │  │   (Rust)      │
│   Port: 5173   │  │   Port: 8080   │
└────────────────┘  └───────┬────────┘
                           │
               ┌───────────┼───────────┐
               │           │           │
        ┌──────▼─────┐ ┌──▼────┐ ┌──▼────────┐
        │   Qdrant   │ │ Internal│ │  Logs    │
        │ (Vector DB)│ │   API  │ │ (File/DB) │
        │ Port: 6333  │ │        │ │           │
        └────────────┘ └────────┘ └───────────┘
```

---

## 2. 常见问题排查

### 2.1 查询响应慢

**症状：**
- P95 延迟 > 3s
- 用户反馈查询慢

**排查步骤：**

1. **检查上游 API 延迟**
   ```bash
   # 查看 Backend 日志中的上游延迟
   grep "upstream_latency" logs/backend.log

   # 使用 curl 测试上游 API
   time curl -X POST $INTERNAL_API_BASE_URL/v1/embeddings \
     -H "Authorization: Bearer $INTERNAL_API_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"model": "ad-embed-v1", "input": "test"}'
   ```

2. **检查 Qdrant 检索延迟**
   ```bash
   # 查看 Backend 日志中的检索延迟
   grep "retrieval_latency" logs/backend.log

   # 检查 Qdrant 性能
   curl http://localhost:6333/metrics
   ```

3. **检查系统资源**
   ```bash
   # CPU 使用率
   top

   # 内存使用
   free -h

   # 磁盘 I/O
   iotop
   ```

**可能的解决方案：**
- 上游 API 慢：联系上游服务团队，或增加缓存
- Qdrant 慢：增加 Qdrant 资源，优化索引参数
- 系统资源不足：扩容服务器资源

---

### 2.2 查询返回降级模式

**症状：**
- degraded: true
- error_code: UPSTREAM_*

**排查步骤：**

1. **查看错误码**
   ```bash
   # 查看日志中的错误码
   grep "error_code" logs/backend.log
   ```

2. **根据错误码排查**

   **UPSTREAM_TIMEOUT**
   ```bash
   # 检查上游服务可用性
   curl -X POST $INTERNAL_API_BASE_URL/v1/chat/completions \
     -H "Authorization: Bearer $INTERNAL_API_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"model": "ad-qa-chat-v1", "messages": [{"role": "user", "content": "test"}]}'
   ```

   **UPSTREAM_RATE_LIMIT**
   ```bash
   # 检查当前 RPM
   curl http://localhost:8080/api/status | jq '.rate_limit_state'

   # 如果接近限制，考虑增加配额或添加缓存
   ```

   **UPSTREAM_AUTH**
   ```bash
   # 检查 Token 配置
   echo $INTERNAL_API_TOKEN

   # 验证 Token 有效性
   curl -H "Authorization: Bearer $INTERNAL_API_TOKEN" \
     $INTERNAL_API_BASE_URL/v1/models
   ```

   **UPSTREAM_UNAVAILABLE**
   ```bash
   # 检查上游服务状态
   curl $INTERNAL_API_BASE_URL/health

   # 检查网络连接
   ping $(echo $INTERNAL_API_BASE_URL | cut -d'/' -f3)
   ```

---

### 2.3 Qdrant 连接失败

**症状：**
- Qdrant 连接错误
- 检索失败

**排查步骤：**

1. **检查 Qdrant 服务状态**
   ```bash
   # 检查 Docker 容器状态
   docker ps | grep qdrant

   # 查看 Qdrant 日志
   docker logs qdrant

   # 检查 Qdrant 健康状态
   curl http://localhost:6333/health
   ```

2. **检查网络连接**
   ```bash
   # 测试端口连通性
   telnet localhost 6333

   # 检查防火墙规则
   sudo iptables -L -n | grep 6333
   ```

3. **检查 Qdrant 数据**
   ```bash
   # 查看 collection 状态
   curl http://localhost:6333/collections/knowledge_chunks

   # 查看 collection 大小
   curl http://localhost:6333/collections/knowledge_chunks | jq '.result.points_count'
   ```

**解决方案：**
- 服务未运行：`docker start qdrant`
- 数据损坏：重建 collection
- 网络问题：检查网络配置和防火墙

---

### 2.4 索引任务失败

**症状：**
- 索引任务状态为 Failed
- 知识库未更新

**排查步骤：**

1. **查看索引任务状态**
   ```bash
   curl http://localhost:8080/api/reindex
   ```

2. **查看 Backend 日志**
   ```bash
   # 查看索引相关日志
   grep "indexer\|reindex" logs/backend.log | tail -100
   ```

3. **检查知识库文件**
   ```bash
   # 检查知识库目录
   ls -la $KNOWLEDGE_DIR

   # 检查 Markdown 文件格式
   file $KNOWLEDGE_DIR/*.md
   ```

4. **检查上游 API**
   ```bash
   # 测试 embedding API
   curl -X POST $INTERNAL_API_BASE_URL/v1/embeddings \
     -H "Authorization: Bearer $INTERNAL_API_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"model": "ad-embed-v1", "input": "test"}'
   ```

**常见错误：**
- **文件格式错误**：修复 Markdown 文件
- **API 限流**：分批处理文件，增加重试间隔
- **权限问题**：检查文件读写权限

---

### 2.5 Frontend 无法访问

**症状：**
- 前端页面无法打开
- 502/503 错误

**排查步骤：**

1. **检查 Frontend 服务状态**
   ```bash
   # 检查 systemd 服务
   sudo systemctl status engineqa-frontend

   # 检查端口监听
   sudo netstat -tlnp | grep 5173
   ```

2. **检查 Nginx 配置（如使用）**
   ```bash
   # 测试 Nginx 配置
   sudo nginx -t

   # 查看 Nginx 日志
   sudo tail -f /var/log/nginx/error.log
   ```

3. **检查 Frontend 日志**
   ```bash
   # 查看 systemd 日志
   sudo journalctl -u engineqa-frontend -f
   ```

**解决方案：**
- 服务未运行：`sudo systemctl start engineqa-frontend`
- Nginx 配置错误：修正配置后重启
- 端口冲突：修改端口配置

---

## 3. 监控指标

### 3.1 关键指标

| 指标 | 目标值 | 告警阈值 | 说明 |
|------|--------|----------|------|
| qa_qps | ≥ 50 | < 10 | 每秒查询数 |
| qa_latency_p95_ms | 1000-3000 | > 5000 | P95 延迟（毫秒） |
| upstream_latency_ms | < 500 | > 2000 | 上游延迟（毫秒） |
| retrieval_latency_ms | < 100 | > 500 | 检索延迟（毫秒） |
| upstream_4xx_total | < 10/min | > 50/min | 上游 4xx 错误数 |
| upstream_5xx_total | < 5/min | > 20/min | 上游 5xx 错误数 |
| degraded_ratio | < 3% | > 5% | 降级比例 |
| retrieval_hit_ratio | ≥ 80% | < 60% | 检索命中率 |

### 3.2 监控工具

**系统监控：**
- Prometheus + Grafana
- Datadog
- New Relic

**日志监控：**
- ELK Stack (Elasticsearch, Logstash, Kibana)
- Splunk
- CloudWatch Logs

**告警方式：**
- 钉钉机器人
- 邮件
- 短信

---

## 4. 应急预案

### 4.1 上游服务故障

**症状：**
- 大量 UPSTREAM_UNAVAILABLE 错误
- 查询全部降级

**应急预案：**

1. **立即行动**
   ```bash
   # 1. 确认故障范围
   curl $INTERNAL_API_BASE_URL/health

   # 2. 检查上游服务状态
   # 联系上游服务团队

   # 3. 启用降级模式（系统已自动）
   ```

2. **缓解措施**
   - 通知用户当前服务状态
   - 引导用户使用历史记录
   - 准备人工客服支持

3. **恢复步骤**
   - 上游服务恢复后，监控系统自动恢复
   - 验证查询功能正常
   - 通知用户服务已恢复

---

### 4.2 Qdrant 故障

**症状：**
- 检索全部失败
- Qdrant 连接错误

**应急预案：**

1. **立即行动**
   ```bash
   # 1. 尝试重启 Qdrant
   sudo docker restart qdrant

   # 2. 检查 Qdrant 数据
   docker exec qdrant qdrant collections list

   # 3. 查看 Qdrant 日志
   sudo docker logs qdrant
   ```

2. **缓解措施**
   - 如果 Qdrant 无法恢复，启用纯关键词搜索（需开发）
   - 通知用户当前功能受限

3. **恢复步骤**
   - Qdrant 恢复后，验证 collection 存在
   - 必要时重建索引
   - 验证检索功能正常

---

### 4.3 数据损坏

**症状：**
- 索引数据异常
- 查询结果不准确

**应急预案：**

1. **立即行动**
   ```bash
   # 1. 备份当前数据
   docker exec qdrant qdrant snapshots create

   # 2. 检查 collection 状态
   curl http://localhost:6333/collections/knowledge_chunks

   # 3. 重建索引
   curl -X POST http://localhost:8080/api/reindex \
     -H "Content-Type: application/json" \
     -d '{}'
   ```

2. **恢复步骤**
   - 等待索引任务完成
   - 验证查询结果准确
   - 必要时从快照恢复

---

## 5. 定期维护

### 5.1 每日
- [ ] 检查系统健康状态
- [ ] 检查关键指标
- [ ] 检查日志中的错误和警告
- [ ] 检查磁盘空间

### 5.2 每周
- [ ] 审查告警情况
- [ ] 分析性能趋势
- [ ] 检查索引任务执行情况
- [ ] 审查反馈数据

### 5.3 每月
- [ ] 检查知识库更新情况
- [ ] 审查准确性指标
- [ ] 优化系统配置
- [ ] 进行安全扫描
- [ ] 备份重要数据

---

## 6. 灰度策略

### 6.1 灰度步骤

**阶段1：内部测试（1-2天）**
- 范围：开发团队
- 目标：验证基本功能
- 监控：所有关键指标

**阶段2：小范围灰度（3-5天）**
- 范围：10% 用户
- 目标：验证性能和稳定性
- 监控：性能、错误率、用户反馈

**阶段3：中等灰度（3-5天）**
- 范围：50% 用户
- 目标：验证大规模下的表现
- 监控：所有关键指标

**阶段4：全量上线**
- 范围：100% 用户
- 目标：全量上线
- 监控：所有关键指标，24小时

### 6.2 灰度回滚策略

**回滚条件：**
- P95 延迟 > 5s
- Error rate > 5%
- Degraded ratio > 10%
- 用户投诉激增

**回滚步骤：**
1. 立即停止灰度流量
2. 切换回旧版本
3. 分析问题原因
4. 修复问题后重新灰度

---

## 7. 联系信息

**技术团队：**
- 后端负责人：[姓名/联系方式]
- 前端负责人：[姓名/联系方式]
- 运维负责人：[姓名/联系方式]

**上游服务：**
- API 提供方：[联系方式]
- 技术支持：[联系方式]

**紧急联系：**
- 值班电话：[电话号码]
- 值班邮箱：[邮箱地址]

---

## 8. 版本信息

- **文档版本**：v1.0
- **最后更新**：2026-02-13
- **维护人**：[姓名]
