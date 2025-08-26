# RustDesk 企业版完整安装使用教程

## 📋 目录

1. [系统要求](#系统要求)
2. [安装方式选择](#安装方式选择)
3. [Docker 快速部署](#docker-快速部署)
4. [二进制文件部署](#二进制文件部署)
5. [源码编译安装](#源码编译安装)
6. [初始配置](#初始配置)
7. [客户端配置](#客户端配置)
8. [Web管理界面使用](#web管理界面使用)
9. [高级功能配置](#高级功能配置)
10. [故障排除](#故障排除)

## 🖥️ 系统要求

### 最低配置
- **CPU**: 2核心
- **内存**: 4GB RAM
- **存储**: 20GB 可用空间
- **网络**: 稳定的互联网连接

### 推荐配置
- **CPU**: 4核心或更多
- **内存**: 8GB RAM 或更多
- **存储**: 50GB SSD
- **网络**: 100Mbps 带宽

### 支持的操作系统
- ✅ Ubuntu 20.04/22.04 LTS
- ✅ CentOS 7/8, RHEL 7/8
- ✅ Debian 10/11
- ✅ Windows Server 2019/2022
- ✅ macOS 10.15+

## 🎯 安装方式选择

| 方式 | 难度 | 适用场景 | 推荐指数 |
|------|------|----------|----------|
| Docker | ⭐ | 快速测试、生产环境 | ⭐⭐⭐⭐⭐ |
| 二进制 | ⭐⭐ | 生产环境、定制需求 | ⭐⭐⭐⭐ |
| 源码编译 | ⭐⭐⭐ | 开发、深度定制 | ⭐⭐⭐ |

## 🐳 Docker 快速部署

### 方式一：一键部署脚本

```bash
# 下载并运行一键部署脚本
curl -fsSL https://raw.githubusercontent.com/ljhlovehui/rustdesk-server/master/scripts/one-click-enterprise.sh | bash
```

### 方式二：手动 Docker 部署

#### 1. 安装 Docker 和 Docker Compose

**Ubuntu/Debian:**
```bash
# 更新包列表
sudo apt update

# 安装 Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER

# 安装 Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# 重新登录以应用组权限
newgrp docker
```

**CentOS/RHEL:**
```bash
# 安装 Docker
sudo yum install -y yum-utils
sudo yum-config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo
sudo yum install -y docker-ce docker-ce-cli containerd.io
sudo systemctl start docker
sudo systemctl enable docker
sudo usermod -aG docker $USER

# 安装 Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose
```

#### 2. 下载配置文件

```bash
# 创建项目目录
mkdir -p ~/rustdesk-enterprise
cd ~/rustdesk-enterprise

# 下载配置文件
wget https://raw.githubusercontent.com/ljhlovehui/rustdesk-server/master/docker-compose-enterprise.yml
wget https://raw.githubusercontent.com/ljhlovehui/rustdesk-server/master/.env.example
mv .env.example .env
```

#### 3. 配置环境变量

```bash
# 编辑配置文件
nano .env
```

**重要配置项说明：**
```bash
# 服务器密钥（必须设置）
RUSTDESK_KEY=your-super-secret-key-here

# JWT 密钥（必须设置）
JWT_SECRET=your-jwt-secret-key-here

# 数据库配置
DATABASE_URL=sqlite:///app/data/enterprise.sqlite3

# 服务端口
HBBS_PORT=21115
HBBR_PORT=21117
WEB_PORT=21119

# 域名配置（如果有域名）
DOMAIN=your-domain.com

# 管理员邮箱
ADMIN_EMAIL=admin@your-domain.com

# SMTP 邮件配置（可选）
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USER=your-email@gmail.com
SMTP_PASS=your-app-password
```

#### 4. 启动服务

```bash
# 基础部署（SQLite数据库）
docker-compose -f docker-compose-enterprise.yml up -d

# 完整部署（PostgreSQL + 监控）
docker-compose -f docker-compose-enterprise.yml --profile postgres --profile monitoring up -d
```

#### 5. 验证部署

```bash
# 检查容器状态
docker ps

# 查看日志
docker logs rustdesk-hbbs-enterprise
docker logs rustdesk-hbbr-enterprise

# 测试连接
curl http://localhost:21119/api/health
```

## 📦 二进制文件部署

### 1. 下载二进制文件

```bash
# 创建安装目录
sudo mkdir -p /opt/rustdesk-enterprise
cd /opt/rustdesk-enterprise

# 下载最新版本（Linux x86_64）
sudo wget https://github.com/ljhlovehui/rustdesk-server/releases/latest/download/rustdesk-enterprise-server-linux-x86_64.tar.gz

# 解压
sudo tar -xzf rustdesk-enterprise-server-linux-x86_64.tar.gz

# 设置执行权限
sudo chmod +x hbbs-enterprise hbbr-enterprise rustdesk-utils-enterprise
```

### 2. 创建配置文件

```bash
# 创建配置目录
sudo mkdir -p /etc/rustdesk-enterprise

# 创建主配置文件
sudo tee /etc/rustdesk-enterprise/config.toml << EOF
[server]
key = "your-super-secret-key"
port = 21115
relay_port = 21117
web_port = 21119

[database]
url = "sqlite:///var/lib/rustdesk-enterprise/enterprise.db"

[security]
jwt_secret = "your-jwt-secret"
session_timeout = 3600

[logging]
level = "info"
file = "/var/log/rustdesk-enterprise/server.log"
EOF
```

### 3. 创建系统服务

**HBBS 服务：**
```bash
sudo tee /etc/systemd/system/rustdesk-hbbs-enterprise.service << EOF
[Unit]
Description=RustDesk Enterprise HBBS Server
After=network.target

[Service]
Type=simple
User=rustdesk
Group=rustdesk
WorkingDirectory=/opt/rustdesk-enterprise
ExecStart=/opt/rustdesk-enterprise/hbbs-enterprise --config /etc/rustdesk-enterprise/config.toml
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF
```

**HBBR 服务：**
```bash
sudo tee /etc/systemd/system/rustdesk-hbbr-enterprise.service << EOF
[Unit]
Description=RustDesk Enterprise HBBR Relay Server
After=network.target

[Service]
Type=simple
User=rustdesk
Group=rustdesk
WorkingDirectory=/opt/rustdesk-enterprise
ExecStart=/opt/rustdesk-enterprise/hbbr-enterprise --config /etc/rustdesk-enterprise/config.toml
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF
```

### 4. 创建用户和目录

```bash
# 创建系统用户
sudo useradd -r -s /bin/false rustdesk

# 创建数据目录
sudo mkdir -p /var/lib/rustdesk-enterprise
sudo mkdir -p /var/log/rustdesk-enterprise

# 设置权限
sudo chown -R rustdesk:rustdesk /var/lib/rustdesk-enterprise
sudo chown -R rustdesk:rustdesk /var/log/rustdesk-enterprise
sudo chown -R rustdesk:rustdesk /opt/rustdesk-enterprise
```

### 5. 启动服务

```bash
# 重新加载 systemd
sudo systemctl daemon-reload

# 启动并启用服务
sudo systemctl enable rustdesk-hbbs-enterprise
sudo systemctl enable rustdesk-hbbr-enterprise
sudo systemctl start rustdesk-hbbs-enterprise
sudo systemctl start rustdesk-hbbr-enterprise

# 检查状态
sudo systemctl status rustdesk-hbbs-enterprise
sudo systemctl status rustdesk-hbbr-enterprise
```

## 🔧 源码编译安装

### 1. 安装依赖

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev libsqlite3-dev git curl
```

**CentOS/RHEL:**
```bash
sudo yum groupinstall -y "Development Tools"
sudo yum install -y openssl-devel sqlite-devel git curl
```

### 2. 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup update
```

### 3. 克隆和编译

```bash
# 克隆仓库
git clone https://github.com/ljhlovehui/rustdesk-server.git
cd rustdesk-server

# 切换到企业版配置
cp Cargo_enterprise.toml Cargo.toml

# 编译企业版
cargo build --release --features enterprise

# 编译结果在 target/release/ 目录
ls -la target/release/
```

## ⚙️ 初始配置

### 1. 防火墙配置

**Ubuntu/Debian (UFW):**
```bash
sudo ufw allow 21115/tcp
sudo ufw allow 21116/udp
sudo ufw allow 21117/tcp
sudo ufw allow 21119/tcp
sudo ufw enable
```

**CentOS/RHEL (firewalld):**
```bash
sudo firewall-cmd --permanent --add-port=21115/tcp
sudo firewall-cmd --permanent --add-port=21116/udp
sudo firewall-cmd --permanent --add-port=21117/tcp
sudo firewall-cmd --permanent --add-port=21119/tcp
sudo firewall-cmd --reload
```

### 2. 生成服务器密钥

```bash
# 使用工具生成密钥
./rustdesk-utils-enterprise genkey

# 或者手动生成
openssl rand -base64 32
```

### 3. 首次访问设置

1. 打开浏览器访问：`http://your-server-ip:21119`
2. 使用默认账户登录：
   - 用户名：`admin`
   - 密码：`admin123`
3. **立即修改默认密码！**

## 📱 客户端配置

### 1. 下载 RustDesk 客户端

访问 [RustDesk 官网](https://rustdesk.com/) 下载适合你系统的客户端。

### 2. 配置服务器地址

#### 方法一：手动配置
1. 打开 RustDesk 客户端
2. 点击右上角的菜单按钮（三个点）
3. 选择"设置"
4. 在"网络"标签页中：
   - **ID服务器**: `your-server-ip:21116`
   - **中继服务器**: `your-server-ip:21117`
   - **密钥**: 输入你设置的服务器密钥

#### 方法二：配置文件
创建配置文件 `RustDesk.toml`：
```toml
[options]
custom-rendezvous-server = "your-server-ip:21116"
relay-server = "your-server-ip:21117"
key = "your-server-key"
```

#### 方法三：命令行参数
```bash
rustdesk --server your-server-ip:21116 --key your-server-key
```

### 3. 企业版客户端登录

1. 在客户端中点击"登录"
2. 输入在Web管理界面创建的用户账户
3. 如果启用了2FA，输入验证码

## 🌐 Web管理界面使用

### 1. 登录管理界面

访问：`http://your-server-ip:21119`

### 2. 用户管理

#### 创建用户
1. 进入"用户管理"页面
2. 点击"添加用户"
3. 填写用户信息：
   - 用户名
   - 邮箱
   - 密码
   - 角色（管理员/普通用户）
   - 所属组

#### 用户权限设置
- **管理员**: 完全访问权限
- **普通用户**: 基础远程访问权限
- **只读用户**: 仅查看权限

### 3. 设备管理

#### 设备分组
1. 创建设备组（如：开发部、财务部）
2. 设置组权限
3. 分配用户到组

#### 设备监控
- 实时在线状态
- 连接历史记录
- 性能指标

### 4. 安全设置

#### 启用双因素认证
1. 进入"安全设置"
2. 启用"双因素认证"
3. 用户扫描二维码绑定

#### IP访问控制
```bash
# 允许特定IP段
192.168.1.0/24
10.0.0.0/8

# 禁止特定IP
!192.168.1.100
```

## 🔧 高级功能配置

### 1. HTTPS 配置

#### 使用 Nginx 反向代理
```nginx
server {
    listen 443 ssl http2;
    server_name your-domain.com;

    ssl_certificate /path/to/your/cert.pem;
    ssl_certificate_key /path/to/your/key.pem;

    location / {
        proxy_pass http://127.0.0.1:21119;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### 2. 数据库配置

#### 使用 PostgreSQL
```bash
# 安装 PostgreSQL
sudo apt install postgresql postgresql-contrib

# 创建数据库
sudo -u postgres createdb rustdesk_enterprise
sudo -u postgres createuser rustdesk_user

# 设置密码
sudo -u postgres psql -c "ALTER USER rustdesk_user PASSWORD 'your_password';"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE rustdesk_enterprise TO rustdesk_user;"
```

更新配置：
```bash
DATABASE_URL=postgresql://rustdesk_user:your_password@localhost/rustdesk_enterprise
```

### 3. 邮件通知配置

```toml
[email]
smtp_host = "smtp.gmail.com"
smtp_port = 587
smtp_user = "your-email@gmail.com"
smtp_password = "your-app-password"
from_address = "noreply@your-domain.com"
```

### 4. 监控配置

#### Prometheus 指标
访问：`http://your-server:21119/metrics`

#### Grafana 仪表板
1. 导入提供的仪表板模板
2. 配置数据源指向 Prometheus
3. 设置告警规则

## 🔍 故障排除

### 常见问题

#### 1. 客户端无法连接
**检查项目：**
```bash
# 检查端口是否开放
netstat -tlnp | grep 21115
netstat -tlnp | grep 21117

# 检查防火墙
sudo ufw status
sudo firewall-cmd --list-ports

# 检查服务状态
sudo systemctl status rustdesk-hbbs-enterprise
sudo systemctl status rustdesk-hbbr-enterprise
```

#### 2. Web界面无法访问
```bash
# 检查Web服务
curl http://localhost:21119/api/health

# 检查日志
sudo journalctl -u rustdesk-hbbs-enterprise -f
```

#### 3. 数据库连接失败
```bash
# 检查数据库文件权限
ls -la /var/lib/rustdesk-enterprise/

# 检查数据库连接
sqlite3 /var/lib/rustdesk-enterprise/enterprise.db ".tables"
```

### 日志分析

#### 查看详细日志
```bash
# 系统日志
sudo journalctl -u rustdesk-hbbs-enterprise -f

# Docker 日志
docker logs rustdesk-hbbs-enterprise -f

# 应用日志
tail -f /var/log/rustdesk-enterprise/server.log
```

#### 启用调试模式
```bash
# 修改日志级别为 debug
RUST_LOG=debug ./hbbs-enterprise
```

### 性能优化

#### 1. 系统优化
```bash
# 增加文件描述符限制
echo "* soft nofile 65536" >> /etc/security/limits.conf
echo "* hard nofile 65536" >> /etc/security/limits.conf

# 优化网络参数
echo "net.core.rmem_max = 16777216" >> /etc/sysctl.conf
echo "net.core.wmem_max = 16777216" >> /etc/sysctl.conf
sysctl -p
```

#### 2. 数据库优化
```sql
-- SQLite 优化
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = 10000;
```

## 📞 获取帮助

### 技术支持
- 📖 [官方文档](https://github.com/ljhlovehui/rustdesk-server/wiki)
- 💬 [GitHub Issues](https://github.com/ljhlovehui/rustdesk-server/issues)
- 📧 技术支持: support@rustdesk.com

### 社区资源
- 🌐 [官方网站](https://rustdesk.com/)
- 💬 [Discord 社区](https://discord.gg/nDceKgxnkV)
- 📱 [Telegram 群组](https://t.me/rustdesk)

---

**🎉 恭喜！你已经成功部署了 RustDesk 企业版服务器！**

如果遇到任何问题，请参考故障排除部分或联系技术支持。