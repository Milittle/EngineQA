# EngineQA 部署指南

## 部署架构

```
┌─────────────────────────────────────────────────────────────┐
│                         Nginx (可选)                        │
│                    (反向代理 + SSL)                         │
└──────────────────────┬──────────────────────────────────────┘
                       │
        ┌──────────────┼──────────────┐
        │              │              │
┌───────▼──────┐ ┌───▼────┐ ┌────▼────────┐
│   Frontend    │ │Backend │ │   Qdrant    │
│  (React/Vite) │ │(Rust)  │ │(Vector DB)  │
│   Port: 5173  │ │Port:8080│ │Port: 6333   │
└──────────────┘ └─────────┘ └─────────────┘
                      │
                      ▼
              ┌───────────────┐
              │ Internal API  │
              │(公司内部服务)  │
              └───────────────┘
```

## 单机/VM 部署

### 前置要求

- Linux (Ubuntu 22.04 LTS 或类似)
- Rust 1.70+
- Node.js 18+
- Docker & Docker Compose (用于 Qdrant)
- 2GB+ RAM
- 20GB+ 磁盘空间

### 步骤 1: 安装依赖

```bash
# 更新系统
sudo apt update && sudo apt upgrade -y

# 安装 Node.js
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt install -y nodejs

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 安装 Docker
sudo apt install -y docker.io docker-compose
sudo usermod -aG docker $USER
newgrp docker
```

### 步骤 2: 克隆代码

```bash
cd /opt
sudo git clone <your-repo-url> engineqa
cd engineqa
```

### 步骤 3: 配置环境变量

```bash
# 复制配置文件
cp .env.example .env

# 编辑配置
nano .env
```

**必需配置项：**

```bash
# Backend
INTERNAL_API_BASE_URL=https://your-internal-api.com
INTERNAL_API_TOKEN=your-service-token
QDRANT_URL=http://localhost:6333

# Frontend
VITE_API_BASE_URL=http://localhost:8080
```

### 步骤 4: 启动 Qdrant

```bash
# 使用 Docker Compose
docker-compose -f deploy/qdrant-compose.yaml up -d

# 验证 Qdrant 运行
curl http://localhost:6333/health
```

### 步骤 5: 构建并启动 Backend

```bash
cd backend

# 安装依赖
cargo build --release

# 创建知识库目录
mkdir -p /data/knowledge

# 启动 Backend
./target/release/engineqa-backend
```

**后台运行：**

```bash
# 使用 systemd（推荐）
sudo cat > /etc/systemd/system/engineqa-backend.service <<EOF
[Unit]
Description=EngineQA Backend
After=network.target qdrant.service

[Service]
Type=simple
User=root
WorkingDirectory=/opt/engineqa/backend
Environment="PATH=/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin"
ExecStart=/opt/engineqa/backend/target/release/engineqa-backend
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl enable engineqa-backend
sudo systemctl start engineqa-backend
sudo systemctl status engineqa-backend
```

### 步骤 6: 构建并启动 Frontend

```bash
cd frontend

# 安装依赖
npm install

# 构建生产版本
npm run build

# 部署到 Nginx 或使用 serve
npx serve dist -l 5173 -n
```

**使用 systemd：**

```bash
sudo cat > /etc/systemd/system/engineqa-frontend.service <<EOF
[Unit]
Description=EngineQA Frontend
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/engineqa/frontend
ExecStart=/root/.nvm/versions/node/v18.*/bin/serve dist -l 5173 -n
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl enable engineqa-frontend
sudo systemctl start engineqa-frontend
```

### 步骤 7: 配置 Nginx（可选）

```bash
sudo apt install -y nginx

sudo cat > /etc/nginx/sites-available/engineqa <<EOF
server {
    listen 80;
    server_name engineqa.your-domain.com;

    # Frontend
    location / {
        proxy_pass http://localhost:5173;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host \$host;
        proxy_cache_bypass \$http_upgrade;
    }

    # Backend API
    location /api/ {
        proxy_pass http://localhost:8080;
        proxy_http_version 1.1;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }

    # Health check
    location /health {
        proxy_pass http://localhost:8080;
        proxy_http_version 1.1;
    }
}
EOF

# 启用站点
sudo ln -s /etc/nginx/sites-available/engineqa /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl restart nginx
```

### 步骤 8: 配置 SSL（推荐）

```bash
# 使用 Let's Encrypt
sudo apt install -y certbot python3-certbot-nginx

sudo certbot --nginx -d engineqa.your-domain.com

# 自动续期
sudo certbot renew --dry-run
```

## 验证部署

```bash
# 健康检查
curl http://localhost:8080/health

# 状态检查
curl http://localhost:8080/api/status

# 查询测试
curl -X POST http://localhost:8080/api/query \
  -H "Content-Type: application/json" \
  -d '{"question": "测试问题", "top_k": 6}'
```

## 首次索引

```bash
# 触发首次索引
curl -X POST http://localhost:8080/api/reindex \
  -H "Content-Type: application/json" \
  -d '{}'

# 查看索引任务状态
curl http://localhost:8080/api/reindex
```

## 监控与日志

```bash
# 查看服务状态
sudo systemctl status engineqa-backend
sudo systemctl status engineqa-frontend

# 查看日志
sudo journalctl -u engineqa-backend -f
sudo journalctl -u engineqa-frontend -f

# Qdrant 日志
sudo docker logs -f qdrant
```

## 性能优化

### Backend
- 调整 `OUTBOUND_MAX_CONCURRENCY` 参数
- 启用 Redis 缓存（未来实现）
- 配置适当的 `LLM_TIMEOUT_MS` 和 `EMBED_TIMEOUT_MS`

### Frontend
- 启用 CDN
- 配置浏览器缓存
- 使用 Gzip 压缩

### Qdrant
- 配置适当的向量维度
- 调整索引参数
- 使用 SSD 存储

## 故障排查

### Backend 无法启动
```bash
# 检查配置
cat .env

# 检查端口占用
sudo netstat -tlnp | grep 8080

# 查看详细日志
RUST_LOG=debug ./target/release/engineqa-backend
```

### Qdrant 连接失败
```bash
# 检查 Qdrant 运行状态
sudo docker ps | grep qdrant

# 检查 Qdrant 日志
sudo docker logs qdrant

# 测试连接
curl http://localhost:6333/health
```

### 查询超时
```bash
# 检查上游 API 延迟
curl -X POST $INTERNAL_API_BASE_URL/v1/embeddings \
  -H "Authorization: Bearer $INTERNAL_API_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"model": "ad-embed-v1", "input": "test"}'

# 检查 Qdrant 延迟
curl http://localhost:6333/collections/knowledge_chunks
```
