# EngineQA

广告引擎现网维优 QA 问答系统（Internal API 推理版）。

## 当前实现状态（2026-02-14）
- 默认运行基线：`backend-python/`（FastAPI + Qdrant embedded）。
- Rust 后端：`backend/` 已完成向量存储重构，检索与索引链路使用 LanceDB 本地存储。
- 统一入口脚本：`scripts/dev.sh`，通过 `BACKEND_RUNTIME` 切换运行时，或者使用`make dev`、`make dev-python`、`make dev-rust`来启动后端

## 目录结构
- `frontend/`: React + Vite + Tailwind 前端。
- `backend-python/`: Python FastAPI 后端（默认）。
- `backend/`: Rust Axum 后端（LanceDB 向量存储）。
- `docs/`: 启动、部署、验收、运维文档。
- `scripts/`: 启动、冒烟、验收、压测、安全检查脚本。
- `knowledge/`: Markdown 知识库目录。

## 端口约定
- Frontend: `5173`
- Backend: `8080`

## 快速启动（Host-Run，推荐）
1. 安装前端依赖：
```bash
npm install --prefix frontend
```

2. 初始化环境变量：
```bash
cp .env.example .env
```

3. 至少配置以下必填项：
- `INTERNAL_API_BASE_URL`
- `INTERNAL_API_TOKEN`

4. 按运行时准备依赖：

Python（默认）
```bash
python3 -m venv .venv-backend-python
.venv-backend-python/bin/pip install -r backend-python/requirements.txt
make dev
```

Rust（LanceDB）
```bash
cargo build --manifest-path backend/Cargo.toml
BACKEND_RUNTIME=rust make dev
```

5. 等价命令（统一入口）：
```bash
make dev                # 默认 python
make dev-python
make dev-rust
```

## 运行验证
健康检查：
```bash
curl -fsS http://127.0.0.1:8080/health
curl -fsS http://127.0.0.1:5173 >/dev/null
curl -fsS http://127.0.0.1:8080/api/status
```

Step-01 冒烟：
```bash
BACKEND_RUNTIME=python ./scripts/smoke-step-01.sh
BACKEND_RUNTIME=rust ./scripts/smoke-step-01.sh
```

## 前端页面说明（含图片预留）
### 1. 问答页面（Query）
- 用途：输入广告引擎维优问题，查看模型回答与参考片段。
- 建议操作：在该页面逐条执行“知识库测试问题（Top 10）”中的问题，观察回答质量与引用内容。

![问答页面截图（待补充）](docs/images/query-page.png)

### 2. 历史页面（History）
- 用途：查看历史问答记录，便于复盘问题、答案与效果反馈。
- 建议操作：按时间或关键词回看测试问题结果，定位不稳定问答。

![历史页面截图（待补充）](docs/images/history-page.png)

### 3. 状态页面（Status）
- 用途：查看服务状态、运行时信息与系统健康情况。
- 建议操作：在执行批量测试前后，先确认系统状态正常。

![状态页面截图（待补充）](docs/images/status-page.png)

### 4. 重新索引功能说明（位于 Status 页）
- 功能：触发知识库重新索引，使新增/更新的 `knowledge/` 文档生效。
- 适用场景：
  - 新增了场景化知识文档后。
  - 修改了文档内容但回答仍命中旧版本时。
  - 需要重建检索数据以验证回归结果时。
- 建议流程：
  1. 在 Status 页面触发重新索引。
  2. 等待状态变为完成（completed）。
  3. 返回 Query 页面重新提问验证命中效果。

![状态页-重新索引功能截图（待补充）](docs/images/status-reindex.png)

## 知识库测试问题（广告引擎维优 Top 10）
在前端界面中测试建议流程：
1. 打开 Query 页面（或提问输入框）。
2. 逐条输入下面问题并提交。
3. 检查回答是否引用了相关场景文档，且结论与“关键观测/根因/排查步骤”一致。
4. 对不准确回答记录问题与 trace_id，便于回放与优化。

建议回归问题集（10 条）：
- 填充率突然归零但请求量正常，一般先看哪三个指标？
- 素材替换后曝光不变点击下降，如何快速定位问题？
- CTR 稳定但 CVR 下滑，怎么判断是落地页还是回传链路问题？
- CPM 在短时段大幅上涨时，先排查哪些出价与竞争指标？
- 日预算花不出去时，如何区分频控过严与定向过窄？
- 如何识别 CTR 下滑是否由频次疲劳引起？
- 新计划冷启动无曝光，应该先调出价还是先放宽定向？
- 地域定向错配时，如何验证配置已正确下发生效？
- 时段投放异常集中在凌晨，如何排查时区或模板错误？
- 转化回传延迟为什么会导致智能出价误判？如何验证？

## 关键环境变量
通用：
- `INTERNAL_API_BASE_URL`
- `INTERNAL_API_TOKEN`
- `INTERNAL_API_CHAT_PATH`
- `INTERNAL_API_EMBED_PATH`
- `EMBEDDING_VECTOR_SIZE`

Rust + LanceDB：
- `VECTOR_STORE=lancedb`
- `LANCEDB_URI=./.lancedb`
- `LANCEDB_TABLE=knowledge_chunks`
- `VECTOR_SCORE_THRESHOLD=0.3`

Python + Qdrant：
- `QDRANT_LOCAL_PATH=./.qdrant-local`
- `QDRANT_COLLECTION=knowledge_chunks`

## API 清单
- `GET /health`
- `POST /api/query`
- `GET /api/status`
- `POST /api/feedback`
- `POST /api/reindex`
- `GET /api/reindex`

## 常用脚本
- `scripts/dev.sh`: 统一入口（根据 `BACKEND_RUNTIME` 分发）。
- `scripts/dev-python.sh`: 启动 Python 后端 + 前端。
- `scripts/dev-rust.sh`: 启动 Rust 后端 + 前端（LanceDB）。
- `scripts/package/build-python-backend.sh`: 打包 Python 运行时（PyInstaller）+ 前端静态资源。
- `scripts/package/build-rust-backend.sh`: 打包 Rust 运行时（Cargo release）+ 前端静态资源。
- `scripts/smoke-step-01.sh`: 基础冒烟（运行时感知）。
- `scripts/smoke-step-13.sh`: Step-13 冒烟（状态接口感知向量存储）。
- `scripts/acceptance-test.sh`: 验收测试。
- `scripts/security-check.sh`: 安全检查。

## 运行时打包（按后端运行时分离）
当前提供两条独立打包链路：
- Python 后端包：`engineqa-python-backend-<version>-<os>-<arch>`
- Rust 后端包：`enginqa-rust-backend-<version>-<os>-<arch>`

平台参数：
- `--os linux|windows`
- `--arch x86_64|arm64`

Rust 后端打包示例：
```bash
./scripts/package/build-rust-backend.sh --os linux --arch x86_64 --version v0.1.0
```

Python 后端打包示例：
```bash
./scripts/package/build-python-backend.sh --os linux --arch x86_64 --version v0.1.0
```

说明：
- Python 打包基于 PyInstaller，不支持可靠跨平台构建，必须在目标 `os/arch` 主机上构建。
- Python 打包需在所选解释器中先安装：`backend-python/requirements.txt` 与 `pyinstaller`。
- 两条打包链路都会先执行前端构建（`npm --prefix frontend run build`）；若未检测到 `frontend/node_modules`，脚本会自动安装前端依赖（优先 `npm ci`，无锁文件则 `npm install`）。
- Linux 产物为 `.tar.gz`，Windows 产物为 `.zip`。
- 包内 `start/stop` 脚本会统一启动/停止“后端 + Nginx（前端静态资源 + `/api` 反向代理）”。
- 目标机器默认需已安装并可直接调用 `nginx` 命令。
- 包内包含前端静态资源目录（`frontend/`）、`config/.env.example`、`knowledge/` 和运行目录（`logs/`、`data/`）。

### 发布包运行依赖（目标机器）
以下为解压后直接运行 `start.sh` / `start.ps1` 的最小依赖：
- 平台匹配：发布包与目标机器 `os/arch` 必须一致（例如 `linux/x86_64`）。
- 系统命令：必须可直接调用 `nginx`（在 `PATH` 中）。
- 外部网络：目标机器必须可访问 Internal API（`INTERNAL_API_BASE_URL`，或 split chat/embed 地址）。
- 环境变量：至少配置 `INTERNAL_API_BASE_URL`、`INTERNAL_API_TOKEN`（通过 `.env`）。
- 端口占用：默认使用 `5173`（前端/Nginx）和 `8080`（后端），需确保端口可用。
- 文件权限：运行用户需要可读写 `data/`、`logs/`、`run/`，并可读 `knowledge/`。
- 向量数据目录：
  - Rust 包默认使用 `data/.lancedb`。
  - Python 包默认使用 `data/.qdrant-local`。
- 资源规划：仓库当前未定义硬性 CPU/内存门槛，实际资源需求取决于并发请求量、知识库规模和索引增长速度。

补充：
- Python 发布包基于 PyInstaller，运行时通常不再依赖系统 Python 解释器。
