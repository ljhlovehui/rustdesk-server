# RustDesk Enterprise Server

[![Build Status](https://github.com/ljhlovehui/rustdesk-enterprise-server/workflows/Build%20RustDesk%20Enterprise%20Server/badge.svg)](https://github.com/ljhlovehui/rustdesk-enterprise-server/actions)
[![Docker Pulls](https://img.shields.io/docker/pulls/rustdesk/rustdesk-enterprise-server)](https://hub.docker.com/r/rustdesk/rustdesk-enterprise-server)
[![License](https://img.shields.io/github/license/ljhlovehui/rustdesk-enterprise-server)](LICENSE)
[![Release](https://img.shields.io/github/v/release/ljhlovehui/rustdesk-enterprise-server)](https://github.com/ljhlovehui/rustdesk-enterprise-server/releases)

🚀 **企业级远程桌面服务器** - 基于开源RustDesk的商业级增强版本

## ✨ 核心特性

### 🔐 企业级认证与安全
- **完整用户系统**: 用户注册、登录、角色管理
- **双因素认证**: TOTP支持，备份码，QR码生成
- **端到端加密**: Sodium加密库，密钥交换
- **安全审计**: 完整操作日志，威胁检测
- **权限控制**: 基于角色的访问控制(RBAC)

### 📁 高级文件传输
- **断点续传**: 大文件传输中断后可继续
- **文件夹同步**: 双向同步，增量更新
- **传输加速**: 压缩传输，多线程并发
- **权限控制**: 文件类型限制，路径访问控制
- **完整性验证**: SHA256哈希校验

### 👥 企业管理功能
- **用户组管理**: 部门/项目分组，层级权限
- **设备分组**: 自动分配规则，监控设置
- **访问控制**: IP限制，时间窗口，会话管理
- **批量操作**: 批量设备管理和配置

### 🌐 Web管理界面
- **现代化界面**: Bootstrap 5，响应式设计
- **实时仪表板**: 系统状态，性能指标
- **完整管理**: 用户、设备、权限、审计
- **移动端支持**: 手机平板完美适配

### 🚀 性能优化
- **高级编解码器**: H.264/H.265/VP9/AV1
- **硬件加速**: GPU编解码加速
- **低延迟模式**: 专门的延迟优化
- **自适应质量**: 根据网络条件自动调整
- **带宽管理**: 智能分配，拥塞控制

## 🎯 适用场景

- ✅ **中小企业**: 50-500台设备的远程管理
- ✅ **教育机构**: 学校实验室和办公设备管理
- ✅ **IT服务商**: 为客户提供远程技术支持
- ✅ **政府机构**: 需要严格安全和审计的环境
- ✅ **大型企业**: 复杂组织架构的集中化管理

## 🚀 快速开始

### Docker 部署（推荐）

1. **下载配置文件**
```bash
wget https://raw.githubusercontent.com/ljhlovehui/rustdesk-enterprise-server/main/docker-compose-enterprise.yml
wget https://raw.githubusercontent.com/ljhlovehui/rustdesk-enterprise-server/main/.env.example
mv .env.example .env
```

2. **编辑配置**
```bash
nano .env
# 设置你的域名、密钥等配置
```

3. **启动服务**
```bash
# 基础部署
docker-compose -f docker-compose-enterprise.yml up -d

# 完整部署（包含监控）
docker-compose -f docker-compose-enterprise.yml --profile postgres --profile monitoring up -d
```

4. **访问管理界面**
- URL: `http://your-server:21119`
- 默认账户: `admin` / `admin123`
- ⚠️ **请立即修改默认密码！**

### 二进制部署

1. **下载最新版本**
```bash
# 选择适合你系统的版本
wget https://github.com/ljhlovehui/rustdesk-enterprise-server/releases/latest/download/rustdesk-enterprise-server-linux-x86_64.tar.gz
tar -xzf rustdesk-enterprise-server-linux-x86_64.tar.gz
```

2. **运行服务器**
```bash
# 启动HBBS服务器
./hbbs-enterprise --enterprise --port 21115 --key your-secret-key

# 启动HBBR中继服务器
./hbbr-enterprise --port 21117 --key your-secret-key
```

## 📦 支持的平台

| 平台 | 架构 | 状态 | 下载 |
|------|------|------|------|
| Linux | x86_64 | ✅ | [下载](https://github.com/ljhlovehui/rustdesk-enterprise-server/releases/latest/download/rustdesk-enterprise-server-linux-x86_64.tar.gz) |
| Linux | ARM64 | ✅ | [下载](https://github.com/ljhlovehui/rustdesk-enterprise-server/releases/latest/download/rustdesk-enterprise-server-linux-aarch64.tar.gz) |
| Linux | ARMv7 | ✅ | [下载](https://github.com/ljhlovehui/rustdesk-enterprise-server/releases/latest/download/rustdesk-enterprise-server-linux-armv7.tar.gz) |
| Windows | x86_64 | ✅ | [下载](https://github.com/ljhlovehui/rustdesk-enterprise-server/releases/latest/download/rustdesk-enterprise-server-windows-x86_64.zip) |
| Windows | i686 | ✅ | [下载](https://github.com/ljhlovehui/rustdesk-enterprise-server/releases/latest/download/rustdesk-enterprise-server-windows-i686.zip) |
| macOS | Intel | ✅ | [下载](https://github.com/ljhlovehui/rustdesk-enterprise-server/releases/latest/download/rustdesk-enterprise-server-macos-x86_64.tar.gz) |
| macOS | Apple Silicon | ✅ | [下载](https://github.com/ljhlovehui/rustdesk-enterprise-server/releases/latest/download/rustdesk-enterprise-server-macos-aarch64.tar.gz) |

### Docker 镜像

```bash
# Docker Hub
docker pull rustdesk/rustdesk-enterprise-server:latest

# GitHub Container Registry
docker pull ghcr.io/ljhlovehui/rustdesk-enterprise-server:latest
```

## 🔧 配置说明

### 环境变量

| 变量名 | 说明 | 默认值 | 必需 |
|--------|------|--------|------|
| `RUSTDESK_KEY` | 服务器密钥 | 自动生成 | 推荐设置 |
| `JWT_SECRET` | JWT签名密钥 | 自动生成 | 推荐设置 |
| `DATABASE_URL` | 数据库连接URL | `sqlite:///app/data/enterprise.sqlite3` | 否 |
| `HBBS_PORT` | HBBS服务端口 | `21115` | 否 |
| `HBBR_PORT` | HBBR中继端口 | `21117` | 否 |
| `WEB_PORT` | Web管理界面端口 | `21119` | 否 |

### 端口说明

| 端口 | 协议 | 用途 | 必需 |
|------|------|------|------|
| 21115 | TCP/UDP | 主服务端口 | ✅ |
| 21116 | TCP/UDP | 设备注册 | ✅ |
| 21117 | TCP | 中继服务 | ✅ |
| 21118 | TCP | WebSocket | 可选 |
| 21119 | TCP | Web管理界面 | 推荐 |

## 📚 文档

- 📖 [部署指南](DEPLOYMENT_GUIDE.md) - 详细的部署说明
- 👥 [用户指南](USER_GUIDE.md) - 完整的使用教程
- 🆚 [功能对比](FEATURE_COMPARISON.md) - 开源版vs企业版对比
- 🏗️ [企业版介绍](README_ENTERPRISE.md) - 企业版功能详解

## 🔄 从开源版迁移

如果您正在使用开源版RustDesk服务器，可以无缝迁移到企业版：

```bash
# 1. 备份现有数据
docker exec rustdesk-hbbs tar -czf /backup.tar.gz /root

# 2. 停止开源版服务
docker-compose down

# 3. 启动企业版
docker-compose -f docker-compose-enterprise.yml up -d

# 4. 导入数据（可选）
# 企业版会自动创建管理员账户
```

## 🔒 安全建议

### 生产环境部署

1. **更改默认密码**: 立即修改admin账户密码
2. **设置强密钥**: 使用复杂的服务器密钥和JWT密钥
3. **启用HTTPS**: 配置SSL证书保护Web界面
4. **配置防火墙**: 只开放必要端口
5. **定期备份**: 设置自动备份策略

### 网络安全

```bash
# 防火墙配置示例
sudo ufw allow 21115/tcp
sudo ufw allow 21116
sudo ufw allow 21117/tcp
sudo ufw allow 21119/tcp  # 限制为内网访问
sudo ufw enable
```

## 📊 监控和维护

### 健康检查

```bash
# Docker健康检查
docker ps  # 查看容器状态

# 手动健康检查
curl http://localhost:21119/api/health
```

### 日志查看

```bash
# Docker日志
docker logs rustdesk-hbbs-enterprise -f
docker logs rustdesk-hbbr-enterprise -f

# 系统日志
journalctl -u rustdesk-hbbs -f
```

### 性能监控

企业版包含完整的监控功能：
- Prometheus指标收集
- Grafana仪表板
- 实时性能监控
- 自动告警通知

## 🤝 贡献指南

我们欢迎社区贡献！

### 开发环境设置

```bash
# 克隆仓库
git clone https://github.com/ljhlovehui/rustdesk-enterprise-server.git
cd rustdesk-enterprise-server

# 安装依赖
sudo apt install build-essential pkg-config libssl-dev libsqlite3-dev

# 设置企业版构建
cp Cargo_enterprise.toml Cargo.toml

# 编译
cargo build --features enterprise

# 运行测试
cargo test --features enterprise
```

### 提交规范

- 🐛 Bug修复: `fix: 修复用户登录问题`
- ✨ 新功能: `feat: 添加设备分组功能`
- 📚 文档: `docs: 更新部署指南`
- 🎨 代码格式: `style: 格式化代码`

## 📞 支持与反馈

### 获取帮助

- 📖 [官方文档](https://github.com/ljhlovehui/rustdesk-enterprise-server/wiki)
- 💬 [GitHub Discussions](https://github.com/ljhlovehui/rustdesk-enterprise-server/discussions)
- 🐛 [问题报告](https://github.com/ljhlovehui/rustdesk-enterprise-server/issues)

### 商业支持

- 📧 企业支持: enterprise@rustdesk.com
- 💼 商务合作: business@rustdesk.com
- 🎓 培训服务: training@rustdesk.com

## 📄 许可证

本项目基于 [AGPL-3.0](LICENSE) 许可证开源。

企业版功能在相同许可证下提供，适用于：
- ✅ 内部使用
- ✅ 学习研究
- ✅ 非商业用途
- ❓ 商业用途请联系我们获取商业许可

## 🙏 致谢

感谢以下项目和贡献者：

- [RustDesk](https://github.com/rustdesk/rustdesk) - 原始开源项目
- [Rust](https://www.rust-lang.org/) - 系统编程语言
- [Tokio](https://tokio.rs/) - 异步运行时
- [Axum](https://github.com/tokio-rs/axum) - Web框架
- 所有贡献者和用户的支持

## ⭐ Star History

[![Star History Chart](https://api.star-history.com/svg?repos=ljhlovehui/rustdesk-enterprise-server&type=Date)](https://star-history.com/#ljhlovehui/rustdesk-enterprise-server&Date)

---

**🚀 立即开始使用RustDesk企业版，体验专业级远程桌面管理！**