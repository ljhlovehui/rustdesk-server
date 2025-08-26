# RustDesk 企业版部署指南

## 📋 系统要求

### 最低配置
- **CPU**: 2核心 2.0GHz
- **内存**: 4GB RAM
- **存储**: 20GB 可用空间
- **网络**: 100Mbps 带宽
- **操作系统**: 
  - Ubuntu 20.04+ / Debian 11+
  - CentOS 8+ / RHEL 8+
  - Windows Server 2019+

### 推荐配置
- **CPU**: 4核心 3.0GHz+
- **内存**: 8GB+ RAM
- **存储**: 100GB+ SSD
- **网络**: 1Gbps+ 带宽
- **操作系统**: Ubuntu 22.04 LTS

### 大规模部署配置
- **CPU**: 8核心+ 3.5GHz+
- **内存**: 16GB+ RAM
- **存储**: 500GB+ NVMe SSD
- **网络**: 10Gbps+ 带宽
- **数据库**: PostgreSQL 集群

## 🚀 快速部署

### 方式一：Docker Compose（推荐新手）

1. **准备环境**
```bash
# 安装 Docker 和 Docker Compose
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh
sudo usermod -aG docker $USER

# 安装 Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/download/v2.20.0/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose
```

2. **下载项目**
```bash
git clone https://github.com/your-repo/rustdesk-enterprise-server.git
cd rustdesk-enterprise-server
```

3. **配置环境变量**
```bash
cp .env.example .env
nano .env
```

编辑 `.env` 文件：
```env
# 基础配置
RUSTDESK_KEY=your-super-secret-server-key-change-this
JWT_SECRET=your-jwt-secret-key-at-least-32-characters-long
ENTERPRISE_DB_URL=sqlite:///data/enterprise.sqlite3

# 网络配置
DOMAIN=your-domain.com
HTTP_PORT=80
HTTPS_PORT=443
HBBS_PORT=21115
HBBR_PORT=21117
WEB_PORT=21119

# 数据库配置（可选，用于大规模部署）
POSTGRES_PASSWORD=your-postgres-password
POSTGRES_USER=rustdesk
POSTGRES_DB=rustdesk_enterprise

# 邮件配置（可选）
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USER=your-email@gmail.com
SMTP_PASSWORD=your-app-password

# 监控配置（可选）
GRAFANA_PASSWORD=admin123
```

4. **启动服务**
```bash
# 基础部署
docker-compose -f docker-compose-enterprise.yml up -d

# 包含 PostgreSQL
docker-compose -f docker-compose-enterprise.yml --profile postgres up -d

# 完整部署（包含监控和反向代理）
docker-compose -f docker-compose-enterprise.yml --profile postgres --profile monitoring --profile nginx up -d
```

5. **验证部署**
```bash
# 检查服务状态
docker-compose ps

# 查看日志
docker-compose logs -f hbbs-enterprise
docker-compose logs -f hbbr-enterprise

# 测试连接
curl http://localhost:21119/api/health
```

### 方式二：手动编译部署

1. **安装依赖**
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev libsqlite3-dev \
    cmake git curl wget unzip

# CentOS/RHEL
sudo yum groupinstall -y "Development Tools"
sudo yum install -y openssl-devel sqlite-devel cmake git curl wget unzip

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

2. **编译项目**
```bash
git clone https://github.com/your-repo/rustdesk-enterprise-server.git
cd rustdesk-enterprise-server

# 使用企业版配置
cp Cargo_enterprise.toml Cargo.toml

# 编译
cargo build --release --features enterprise

# 编译结果在 target/release/ 目录
ls -la target/release/
```

3. **创建系统用户**
```bash
sudo useradd -r -s /bin/false rustdesk
sudo mkdir -p /opt/rustdesk/{bin,data,logs,web}
sudo chown -R rustdesk:rustdesk /opt/rustdesk
```

4. **安装文件**
```bash
# 复制二进制文件
sudo cp target/release/hbbs-enterprise /opt/rustdesk/bin/
sudo cp target/release/hbbr-enterprise /opt/rustdesk/bin/
sudo cp -r web/* /opt/rustdesk/web/

# 设置权限
sudo chmod +x /opt/rustdesk/bin/*
sudo chown -R rustdesk:rustdesk /opt/rustdesk
```

5. **创建配置文件**
```bash
sudo mkdir -p /etc/rustdesk
sudo tee /etc/rustdesk/config.toml << EOF
[server]
port = 21115
key = "your-super-secret-server-key"
relay_port = 21117
web_port = 21119

[database]
url = "sqlite:///opt/rustdesk/data/enterprise.sqlite3"
max_connections = 10

[security]
jwt_secret = "your-jwt-secret-key-at-least-32-characters-long"
enable_2fa = true
session_timeout = 28800  # 8 hours

[performance]
enable_hardware_acceleration = true
low_latency_mode = false
max_concurrent_sessions = 100
EOF

sudo chown rustdesk:rustdesk /etc/rustdesk/config.toml
sudo chmod 600 /etc/rustdesk/config.toml
```

6. **创建 systemd 服务**
```bash
# HBBS 服务
sudo tee /etc/systemd/system/rustdesk-hbbs.service << EOF
[Unit]
Description=RustDesk Enterprise HBBS Server
After=network.target
Wants=network.target

[Service]
Type=simple
User=rustdesk
Group=rustdesk
ExecStart=/opt/rustdesk/bin/hbbs-enterprise --enterprise --config /etc/rustdesk/config.toml
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=rustdesk-hbbs
KillMode=mixed
TimeoutStopSec=5

# 安全设置
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/opt/rustdesk/data /opt/rustdesk/logs

[Install]
WantedBy=multi-user.target
EOF

# HBBR 服务
sudo tee /etc/systemd/system/rustdesk-hbbr.service << EOF
[Unit]
Description=RustDesk Enterprise HBBR Relay Server
After=network.target rustdesk-hbbs.service
Wants=network.target
Requires=rustdesk-hbbs.service

[Service]
Type=simple
User=rustdesk
Group=rustdesk
ExecStart=/opt/rustdesk/bin/hbbr-enterprise --config /etc/rustdesk/config.toml
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=rustdesk-hbbr
KillMode=mixed
TimeoutStopSec=5

# 安全设置
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/opt/rustdesk/data /opt/rustdesk/logs

[Install]
WantedBy=multi-user.target
EOF
```

7. **启动服务**
```bash
# 重新加载 systemd
sudo systemctl daemon-reload

# 启动并启用服务
sudo systemctl enable rustdesk-hbbs rustdesk-hbbr
sudo systemctl start rustdesk-hbbs rustdesk-hbbr

# 检查状态
sudo systemctl status rustdesk-hbbs
sudo systemctl status rustdesk-hbbr

# 查看日志
sudo journalctl -u rustdesk-hbbs -f
sudo journalctl -u rustdesk-hbbr -f
```

## 🔧 网络配置

### 防火墙设置

```bash
# Ubuntu/Debian (ufw)
sudo ufw allow 21115/tcp    # HBBS 主端口
sudo ufw allow 21116/tcp    # HBBS 注册端口
sudo ufw allow 21116/udp    # HBBS UDP
sudo ufw allow 21117/tcp    # HBBR 中继端口
sudo ufw allow 21118/tcp    # WebSocket
sudo ufw allow 21119/tcp    # Web 管理界面
sudo ufw allow 80/tcp       # HTTP (可选)
sudo ufw allow 443/tcp      # HTTPS (可选)
sudo ufw enable

# CentOS/RHEL (firewalld)
sudo firewall-cmd --permanent --add-port=21115/tcp
sudo firewall-cmd --permanent --add-port=21116/tcp
sudo firewall-cmd --permanent --add-port=21116/udp
sudo firewall-cmd --permanent --add-port=21117/tcp
sudo firewall-cmd --permanent --add-port=21118/tcp
sudo firewall-cmd --permanent --add-port=21119/tcp
sudo firewall-cmd --permanent --add-port=80/tcp
sudo firewall-cmd --permanent --add-port=443/tcp
sudo firewall-cmd --reload
```

### 端口说明

| 端口 | 协议 | 用途 | 是否必需 |
|------|------|------|----------|
| 21115 | TCP/UDP | 主服务端口 | 必需 |
| 21116 | TCP/UDP | 设备注册 | 必需 |
| 21117 | TCP | 中继服务 | 必需 |
| 21118 | TCP | WebSocket | 可选 |
| 21119 | TCP | Web管理界面 | 推荐 |
| 80 | TCP | HTTP | 可选 |
| 443 | TCP | HTTPS | 推荐 |

### 域名配置

如果使用域名访问，需要配置 DNS 记录：

```dns
# A 记录
rustdesk.yourdomain.com.    IN  A   your.server.ip.address

# 或者 CNAME 记录
rustdesk.yourdomain.com.    IN  CNAME   your-server-hostname.com.
```

## 🔒 SSL/TLS 配置

### 使用 Let's Encrypt（推荐）

1. **安装 Certbot**
```bash
# Ubuntu/Debian
sudo apt install certbot

# CentOS/RHEL
sudo yum install certbot
```

2. **获取证书**
```bash
sudo certbot certonly --standalone -d rustdesk.yourdomain.com
```

3. **配置 Nginx 反向代理**
```bash
sudo tee /etc/nginx/sites-available/rustdesk << EOF
server {
    listen 80;
    server_name rustdesk.yourdomain.com;
    return 301 https://\$server_name\$request_uri;
}

server {
    listen 443 ssl http2;
    server_name rustdesk.yourdomain.com;

    ssl_certificate /etc/letsencrypt/live/rustdesk.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/rustdesk.yourdomain.com/privkey.pem;
    
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512:ECDHE-RSA-AES256-GCM-SHA384:DHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 10m;

    location / {
        proxy_pass http://localhost:21119;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }
}
EOF

sudo ln -s /etc/nginx/sites-available/rustdesk /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

4. **自动续期**
```bash
sudo crontab -e
# 添加以下行
0 12 * * * /usr/bin/certbot renew --quiet
```

## 📊 监控配置

### Prometheus + Grafana

1. **启用监控**
```bash
docker-compose -f docker-compose-enterprise.yml --profile monitoring up -d
```

2. **访问 Grafana**
- URL: `http://your-server:3000`
- 用户名: `admin`
- 密码: 在 `.env` 文件中设置的 `GRAFANA_PASSWORD`

3. **导入仪表板**
- 导入预配置的 RustDesk 仪表板
- 配置数据源指向 Prometheus

### 日志监控

```bash
# 查看实时日志
sudo journalctl -u rustdesk-hbbs -f
sudo journalctl -u rustdesk-hbbr -f

# 查看错误日志
sudo journalctl -u rustdesk-hbbs --since "1 hour ago" -p err

# 日志轮转配置
sudo tee /etc/logrotate.d/rustdesk << EOF
/opt/rustdesk/logs/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 644 rustdesk rustdesk
    postrotate
        systemctl reload rustdesk-hbbs rustdesk-hbbr
    endscript
}
EOF
```

## 🔧 性能调优

### 系统优化

```bash
# 增加文件描述符限制
echo "* soft nofile 65536" | sudo tee -a /etc/security/limits.conf
echo "* hard nofile 65536" | sudo tee -a /etc/security/limits.conf

# 网络优化
sudo tee -a /etc/sysctl.conf << EOF
# 网络性能优化
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
net.core.netdev_max_backlog = 5000
net.ipv4.tcp_congestion_control = bbr
net.ipv4.tcp_rmem = 4096 87380 134217728
net.ipv4.tcp_wmem = 4096 65536 134217728
net.ipv4.tcp_mtu_probing = 1

# 连接优化
net.ipv4.ip_local_port_range = 1024 65535
net.ipv4.tcp_fin_timeout = 30
net.ipv4.tcp_keepalive_time = 1200
net.ipv4.tcp_max_syn_backlog = 8192
EOF

sudo sysctl -p
```

### 数据库优化

对于 SQLite：
```bash
# 在配置文件中添加
[database]
pragma_journal_mode = "WAL"
pragma_synchronous = "NORMAL"
pragma_cache_size = 10000
pragma_temp_store = "MEMORY"
```

对于 PostgreSQL：
```sql
-- 优化配置
ALTER SYSTEM SET shared_buffers = '256MB';
ALTER SYSTEM SET effective_cache_size = '1GB';
ALTER SYSTEM SET maintenance_work_mem = '64MB';
ALTER SYSTEM SET checkpoint_completion_target = 0.9;
ALTER SYSTEM SET wal_buffers = '16MB';
ALTER SYSTEM SET default_statistics_target = 100;
SELECT pg_reload_conf();
```

## 🔄 备份和恢复

### 自动备份脚本

```bash
sudo tee /opt/rustdesk/backup.sh << 'EOF'
#!/bin/bash

BACKUP_DIR="/opt/rustdesk/backups"
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="rustdesk_backup_${DATE}.tar.gz"

# 创建备份目录
mkdir -p "$BACKUP_DIR"

# 停止服务
systemctl stop rustdesk-hbbs rustdesk-hbbr

# 创建备份
tar -czf "$BACKUP_DIR/$BACKUP_FILE" \
    /opt/rustdesk/data \
    /etc/rustdesk \
    /opt/rustdesk/web

# 启动服务
systemctl start rustdesk-hbbs rustdesk-hbbr

# 清理旧备份（保留30天）
find "$BACKUP_DIR" -name "rustdesk_backup_*.tar.gz" -mtime +30 -delete

echo "Backup completed: $BACKUP_DIR/$BACKUP_FILE"
EOF

sudo chmod +x /opt/rustdesk/backup.sh

# 添加到 crontab（每天凌晨2点备份）
echo "0 2 * * * /opt/rustdesk/backup.sh" | sudo crontab -
```

### 恢复数据

```bash
# 停止服务
sudo systemctl stop rustdesk-hbbs rustdesk-hbbr

# 恢复备份
sudo tar -xzf /path/to/backup/rustdesk_backup_YYYYMMDD_HHMMSS.tar.gz -C /

# 设置权限
sudo chown -R rustdesk:rustdesk /opt/rustdesk/data
sudo chown -R rustdesk:rustdesk /etc/rustdesk

# 启动服务
sudo systemctl start rustdesk-hbbs rustdesk-hbbr
```

## 🔍 故障排除

### 常见问题

1. **服务无法启动**
```bash
# 检查配置文件
sudo -u rustdesk /opt/rustdesk/bin/hbbs-enterprise --config /etc/rustdesk/config.toml --check-config

# 检查端口占用
sudo netstat -tlnp | grep :21115

# 检查权限
ls -la /opt/rustdesk/data
```

2. **客户端无法连接**
```bash
# 测试端口连通性
telnet your-server-ip 21115
telnet your-server-ip 21117

# 检查防火墙
sudo ufw status
sudo iptables -L

# 查看服务器日志
sudo journalctl -u rustdesk-hbbs -n 100
```

3. **Web界面无法访问**
```bash
# 检查Web服务
curl http://localhost:21119/api/health

# 检查Nginx配置
sudo nginx -t
sudo systemctl status nginx
```

### 日志分析

```bash
# 查看错误日志
sudo journalctl -u rustdesk-hbbs --since "1 hour ago" -p err

# 查看连接日志
sudo journalctl -u rustdesk-hbbs | grep "connection"

# 实时监控
sudo tail -f /opt/rustdesk/logs/hbbs.log
```

## 📈 扩展部署

### 负载均衡配置

```nginx
upstream rustdesk_backend {
    server 192.168.1.10:21115;
    server 192.168.1.11:21115;
    server 192.168.1.12:21115;
}

server {
    listen 21115;
    proxy_pass rustdesk_backend;
}
```

### 集群部署

1. **数据库集群**
2. **Redis 会话共享**
3. **文件存储共享**
4. **监控集群**

详细的集群部署指南请参考高级部署文档。