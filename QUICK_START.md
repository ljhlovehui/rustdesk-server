# 🚀 RustDesk企业版一键部署指南

## 📋 前提条件

- ✅ 已安装Git
- ✅ 有GitHub账户访问权限
- ✅ 本地有网络连接

## 🎯 一键升级命令

### 方法一：直接运行脚本（推荐）

```bash
# 下载并运行一键升级脚本
curl -fsSL https://raw.githubusercontent.com/ljhlovehui/rustdesk-server/main/scripts/one-click-enterprise.sh | bash
```

### 方法二：手动下载运行

```bash
# 1. 下载脚本
wget https://raw.githubusercontent.com/ljhlovehui/rustdesk-server/main/scripts/one-click-enterprise.sh

# 2. 给予执行权限
chmod +x one-click-enterprise.sh

# 3. 运行脚本
./one-click-enterprise.sh
```

## 🔄 脚本执行流程

脚本会自动完成以下操作：

1. ✅ **检查环境** - 验证Git等必要工具
2. ✅ **克隆仓库** - 从GitHub获取最新代码
3. ✅ **创建企业版分支** - 创建`enterprise-edition`分支
4. ✅ **添加企业版功能** - 添加所有企业级功能代码
5. ✅ **配置自动化** - 设置GitHub Actions自动编译
6. ✅ **创建Docker配置** - 生成Docker部署文件
7. ✅ **提交推送** - 自动提交并推送到GitHub

## 📦 完成后您将获得

### 🌟 企业版功能
- **用户认证系统** - 完整的登录注册、角色权限
- **双因素认证** - TOTP支持，备份码
- **设备分组管理** - 按部门/项目分组
- **高级文件传输** - 断点续传、文件夹同步
- **Web管理界面** - 现代化管理控制台
- **安全审计** - 完整操作日志和威胁检测
- **性能优化** - 硬件加速、低延迟模式

### 🔧 自动化构建
- **多平台编译** - Linux/Windows/macOS自动编译
- **Docker镜像** - 自动构建和发布
- **Release发布** - 自动创建下载链接

### 📱 支持平台
- Linux (x86_64, ARM64, ARMv7)
- Windows (x86_64, i686)
- macOS (Intel, Apple Silicon)

## 🚀 立即部署

脚本完成后，使用以下命令立即部署：

```bash
# 1. 进入项目目录
cd rustdesk-server

# 2. 复制环境配置
cp .env.example .env

# 3. 编辑配置（设置您的域名和密钥）
nano .env

# 4. 启动企业版服务
docker-compose -f docker-compose-enterprise.yml up -d
```

## 🌐 访问管理界面

- **URL**: `http://your-server:21119`
- **默认账户**: `admin` / `admin123`
- **⚠️ 重要**: 首次登录后立即修改密码！

## 📊 监控构建状态

- **GitHub Actions**: `https://github.com/ljhlovehui/rustdesk-server/actions`
- **企业版分支**: `https://github.com/ljhlovehui/rustdesk-server/tree/enterprise-edition`

## 🔧 端口配置

| 端口 | 用途 | 必需 |
|------|------|------|
| 21115 | 主服务端口 | ✅ |
| 21116 | 设备注册 | ✅ |
| 21117 | 中继服务 | ✅ |
| 21118 | WebSocket | 可选 |
| 21119 | Web管理界面 | 推荐 |

## 🆘 故障排除

### 脚本执行失败
```bash
# 检查网络连接
ping github.com

# 检查Git配置
git config --global user.name "Your Name"
git config --global user.email "your.email@example.com"

# 重新运行脚本
./one-click-enterprise.sh
```

### Docker启动失败
```bash
# 检查端口占用
sudo netstat -tlnp | grep :21115

# 查看日志
docker-compose logs -f

# 重新启动
docker-compose down && docker-compose up -d
```

## 📞 获取帮助

- **GitHub Issues**: https://github.com/ljhlovehui/rustdesk-server/issues
- **文档**: 查看项目中的详细文档
- **社区**: RustDesk官方社区

---

**🎉 一条命令，立即拥有企业级RustDesk服务器！**