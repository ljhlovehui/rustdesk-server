# RustDesk 企业版服务器

这是基于开源RustDesk服务器的企业级增强版本，添加了商业版的核心功能特性。

## 🚀 企业版功能特性

### ✅ 已实现功能

#### 🔐 用户认证与权限管理
- **多角色用户系统**: 超级管理员、管理员、普通用户、只读用户
- **JWT令牌认证**: 安全的会话管理
- **双因素认证(2FA)**: TOTP支持，增强安全性
- **密码策略**: BCrypt加密，失败锁定机制
- **会话管理**: 自动过期，多设备登录控制

#### 👥 设备管理
- **设备注册**: 自动记录设备信息（OS、IP、MAC地址等）
- **设备分组**: 按部门、项目或用途分组管理
- **权限控制**: 细粒度的设备访问权限
- **在线状态**: 实时监控设备在线状态
- **设备标签**: 自定义标签分类

#### 📊 审计与监控
- **完整审计日志**: 记录所有用户操作和设备连接
- **连接会话记录**: 详细的连接时长、数据传输量统计
- **实时监控**: 系统状态、性能指标监控
- **报表生成**: 使用情况统计和分析

#### 🌐 Web管理界面
- **现代化界面**: 响应式设计，支持移动端
- **仪表板**: 实时统计和系统状态概览
- **用户管理**: 创建、编辑、删除用户账户
- **设备控制**: 远程设备管理和控制
- **日志查看**: 审计日志查询和筛选

#### 🔒 企业级安全
- **IP封锁**: 智能IP封锁和白名单机制
- **访问控制**: 基于角色的访问控制(RBAC)
- **加密通信**: 端到端加密保护
- **安全审计**: 安全事件记录和告警

### 🚧 计划中功能

- **LDAP/AD集成**: 企业目录服务集成
- **SSO单点登录**: SAML/OAuth2支持
- **邮件通知**: 安全事件和系统通知
- **API接口**: RESTful API用于第三方集成
- **集群部署**: 高可用性和负载均衡
- **数据备份**: 自动备份和恢复机制

## 📦 快速部署

### 方式一：Docker Compose（推荐）

1. **克隆项目**
```bash
git clone <your-repo>
cd rustdesk-server-enterprise
```

2. **配置环境变量**
```bash
cp .env.example .env
# 编辑 .env 文件，设置必要的配置
```

3. **启动服务**
```bash
# 基础部署
docker-compose -f docker-compose-enterprise.yml up -d

# 包含PostgreSQL数据库
docker-compose -f docker-compose-enterprise.yml --profile postgres up -d

# 完整部署（包含监控）
docker-compose -f docker-compose-enterprise.yml --profile postgres --profile monitoring --profile nginx up -d
```

### 方式二：手动编译

1. **安装依赖**
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install build-essential pkg-config libssl-dev libsqlite3-dev

# CentOS/RHEL
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel sqlite-devel
```

2. **编译项目**
```bash
# 复制企业版配置
cp Cargo_enterprise.toml Cargo.toml

# 编译
cargo build --release --features enterprise
```

3. **运行服务**
```bash
# 设置环境变量
export RUSTDESK_ENTERPRISE=1
export JWT_SECRET="your-super-secret-jwt-key"
export ENTERPRISE_DB_URL="enterprise.sqlite3"

# 启动服务器
./target/release/hbbs-enterprise --enterprise --port 21115 --key your-secret-key
./target/release/hbbr-enterprise --port 21117 --key your-secret-key
```

## 🔧 配置说明

### 环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `RUSTDESK_ENTERPRISE` | 启用企业功能 | - |
| `JWT_SECRET` | JWT签名密钥 | 随机生成 |
| `ENTERPRISE_DB_URL` | 企业数据库URL | `enterprise.sqlite3` |
| `MAX_DATABASE_CONNECTIONS` | 最大数据库连接数 | `10` |
| `WEB_PORT` | Web管理界面端口 | `主端口+3` |

### 端口说明

| 端口 | 协议 | 用途 |
|------|------|------|
| 21115 | TCP/UDP | 主服务端口 |
| 21116 | TCP/UDP | 设备注册端口 |
| 21117 | TCP | 中继服务端口 |
| 21118 | TCP | WebSocket端口 |
| 21119 | TCP | Web管理界面 |

## 🎯 使用指南

### 首次登录

1. **访问Web界面**: `http://your-server:21119`
2. **默认管理员账户**:
   - 用户名: `admin`
   - 密码: `admin123`
3. **⚠️ 重要**: 首次登录后立即修改默认密码！

### 用户管理

1. **创建用户**
   - 进入"用户管理"页面
   - 点击"创建用户"按钮
   - 填写用户信息并选择角色

2. **角色权限**
   - **超级管理员**: 所有权限
   - **管理员**: 用户和设备管理
   - **普通用户**: 访问分配的设备
   - **只读用户**: 仅查看权限

### 设备管理

1. **设备注册**: 客户端连接时自动注册
2. **设备分组**: 创建分组并分配设备
3. **权限控制**: 设置用户对设备的访问权限

### 监控和审计

1. **仪表板**: 查看系统概览和实时统计
2. **审计日志**: 查看详细的操作记录
3. **连接监控**: 实时查看活跃连接

## 🔒 安全建议

### 生产环境部署

1. **更改默认密码**: 立即修改admin账户密码
2. **设置强密钥**: 使用复杂的JWT密钥和服务器密钥
3. **启用HTTPS**: 配置SSL证书保护Web界面
4. **防火墙配置**: 只开放必要端口
5. **定期备份**: 备份数据库和配置文件

### 网络安全

```bash
# 防火墙配置示例（Ubuntu）
sudo ufw allow 21115/tcp
sudo ufw allow 21116
sudo ufw allow 21117/tcp
sudo ufw allow 21118/tcp
sudo ufw allow 21119/tcp  # 仅内网访问
sudo ufw enable
```

### SSL配置

```nginx
# Nginx配置示例
server {
    listen 443 ssl;
    server_name your-domain.com;
    
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    location / {
        proxy_pass http://localhost:21119;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## 📈 性能优化

### 数据库优化

```bash
# SQLite优化（小规模部署）
export SQLITE_CACHE_SIZE=10000
export SQLITE_TEMP_STORE=memory

# PostgreSQL（大规模部署）
export ENTERPRISE_DB_URL="postgresql://user:pass@localhost/rustdesk"
```

### 系统优化

```bash
# 增加文件描述符限制
echo "* soft nofile 65536" >> /etc/security/limits.conf
echo "* hard nofile 65536" >> /etc/security/limits.conf

# 优化网络参数
echo "net.core.rmem_max = 134217728" >> /etc/sysctl.conf
echo "net.core.wmem_max = 134217728" >> /etc/sysctl.conf
sysctl -p
```

## 🐛 故障排除

### 常见问题

1. **无法访问Web界面**
   - 检查端口是否开放
   - 确认服务是否正常启动
   - 查看防火墙设置

2. **设备无法连接**
   - 验证服务器密钥配置
   - 检查网络连通性
   - 查看服务器日志

3. **数据库错误**
   - 检查数据库文件权限
   - 确认磁盘空间充足
   - 查看数据库连接配置

### 日志查看

```bash
# Docker部署
docker logs rustdesk-hbbs-enterprise
docker logs rustdesk-hbbr-enterprise

# 手动部署
tail -f /var/log/rustdesk/hbbs.log
tail -f /var/log/rustdesk/hbbr.log
```

## 🤝 贡献指南

欢迎贡献代码和建议！

1. Fork项目
2. 创建功能分支
3. 提交更改
4. 发起Pull Request

## 📄 许可证

本项目基于原RustDesk开源协议，企业版功能遵循相同许可证。

## 📞 支持

- 📧 邮箱: support@your-domain.com
- 💬 讨论: GitHub Issues
- 📖 文档: [详细文档链接]

---

**⚠️ 免责声明**: 这是一个基于开源RustDesk的增强版本，用于学习和研究目的。生产环境使用请充分测试并评估安全风险。