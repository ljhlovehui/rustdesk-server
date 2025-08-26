# 🔄 RustDesk 企业版升级指南

## 📋 升级概述

本指南帮助你从开源版本或旧版本的 RustDesk 服务器升级到最新的企业版。

## 🔍 升级前检查

### 系统兼容性
- [ ] 操作系统版本支持
- [ ] 硬件资源充足
- [ ] 网络端口可用
- [ ] 数据库兼容性

### 数据备份
```bash
# 备份配置文件
cp -r /etc/rustdesk/ /backup/rustdesk-config-$(date +%Y%m%d)

# 备份数据库
cp /var/lib/rustdesk/db.sqlite3 /backup/rustdesk-db-$(date +%Y%m%d).sqlite3

# 备份日志
cp -r /var/log/rustdesk/ /backup/rustdesk-logs-$(date +%Y%m%d)
```

## 🚀 升级方式

### 方式一：Docker 升级（推荐）

#### 从开源版升级
```bash
# 1. 停止现有服务
docker-compose down

# 2. 备份数据
docker run --rm -v rustdesk_data:/data -v $(pwd):/backup alpine tar czf /backup/rustdesk-backup-$(date +%Y%m%d).tar.gz /data

# 3. 下载企业版配置
wget https://raw.githubusercontent.com/ljhlovehui/rustdesk-server/master/docker-compose-enterprise.yml

# 4. 迁移配置
cp .env .env.backup
wget https://raw.githubusercontent.com/ljhlovehui/rustdesk-server/master/.env.example
# 手动合并配置

# 5. 启动企业版
docker-compose -f docker-compose-enterprise.yml up -d
```

#### 企业版版本升级
```bash
# 1. 拉取最新镜像
docker-compose -f docker-compose-enterprise.yml pull

# 2. 重启服务
docker-compose -f docker-compose-enterprise.yml up -d
```

### 方式二：二进制文件升级

#### 停止现有服务
```bash
sudo systemctl stop rustdesk-hbbs
sudo systemctl stop rustdesk-hbbr
```

#### 备份和升级
```bash
# 1. 备份现有文件
sudo cp -r /opt/rustdesk/ /backup/rustdesk-$(date +%Y%m%d)

# 2. 下载新版本
cd /tmp
wget https://github.com/ljhlovehui/rustdesk-server/releases/latest/download/rustdesk-enterprise-server-linux-x86_64.tar.gz
tar -xzf rustdesk-enterprise-server-linux-x86_64.tar.gz

# 3. 替换文件
sudo cp hbbs-enterprise /opt/rustdesk/
sudo cp hbbr-enterprise /opt/rustdesk/
sudo chmod +x /opt/rustdesk/hbbs-enterprise /opt/rustdesk/hbbr-enterprise

# 4. 更新服务配置
sudo cp /backup/rustdesk-$(date +%Y%m%d)/systemd/*.service /etc/systemd/system/
sudo systemctl daemon-reload

# 5. 启动服务
sudo systemctl start rustdesk-hbbs-enterprise
sudo systemctl start rustdesk-hbbr-enterprise
```

## ⚙️ 配置迁移

### 环境变量映射

| 开源版 | 企业版 | 说明 |
|--------|--------|------|
| `RUSTDESK_KEY` | `RUSTDESK_KEY` | 保持不变 |
| `RUSTDESK_PORT` | `HBBS_PORT` | 端口配置 |
| `RUSTDESK_RELAY_PORT` | `HBBR_PORT` | 中继端口 |
| - | `WEB_PORT` | 新增Web端口 |
| - | `JWT_SECRET` | 新增JWT密钥 |

### 配置文件转换
```bash
# 自动转换脚本
./scripts/upgrade-to-enterprise.sh --config /etc/rustdesk/config.toml
```

## 🔧 数据库迁移

### SQLite 迁移
```bash
# 企业版会自动检测并升级数据库结构
# 首次启动时会执行迁移脚本
```

### PostgreSQL 迁移
```sql
-- 如果需要迁移到 PostgreSQL
-- 1. 导出 SQLite 数据
sqlite3 /var/lib/rustdesk/db.sqlite3 .dump > rustdesk_export.sql

-- 2. 转换为 PostgreSQL 格式
# 使用工具如 pgloader 或手动转换

-- 3. 导入到 PostgreSQL
psql -U rustdesk_user -d rustdesk_enterprise -f rustdesk_converted.sql
```

## 🔍 升级验证

### 服务状态检查
```bash
# Docker 部署
docker ps
docker logs rustdesk-hbbs-enterprise
docker logs rustdesk-hbbr-enterprise

# 二进制部署
sudo systemctl status rustdesk-hbbs-enterprise
sudo systemctl status rustdesk-hbbr-enterprise
```

### 功能测试
```bash
# 1. 健康检查
curl http://localhost:21119/api/health

# 2. Web界面访问
curl -I http://localhost:21119

# 3. 客户端连接测试
# 使用 RustDesk 客户端测试连接
```

### 性能验证
- [ ] 连接速度正常
- [ ] 文件传输功能
- [ ] Web管理界面响应
- [ ] 用户认证功能

## 🐛 常见升级问题

### 问题1：端口冲突
```bash
# 检查端口占用
netstat -tlnp | grep 21119

# 解决方案：修改端口配置
WEB_PORT=21120
```

### 问题2：权限问题
```bash
# 检查文件权限
ls -la /opt/rustdesk/
ls -la /var/lib/rustdesk/

# 修复权限
sudo chown -R rustdesk:rustdesk /opt/rustdesk/
sudo chown -R rustdesk:rustdesk /var/lib/rustdesk/
```

### 问题3：数据库迁移失败
```bash
# 检查数据库文件
sqlite3 /var/lib/rustdesk/db.sqlite3 ".tables"

# 手动执行迁移
./hbbs-enterprise --migrate-db
```

### 问题4：配置文件格式错误
```bash
# 验证配置文件
./hbbs-enterprise --check-config

# 重新生成配置
cp .env.example .env
# 手动编辑配置
```

## 🔄 回滚方案

### Docker 回滚
```bash
# 1. 停止企业版
docker-compose -f docker-compose-enterprise.yml down

# 2. 恢复数据
docker run --rm -v rustdesk_data:/data -v $(pwd):/backup alpine tar xzf /backup/rustdesk-backup-YYYYMMDD.tar.gz -C /

# 3. 启动开源版
docker-compose up -d
```

### 二进制回滚
```bash
# 1. 停止企业版服务
sudo systemctl stop rustdesk-hbbs-enterprise
sudo systemctl stop rustdesk-hbbr-enterprise

# 2. 恢复文件
sudo cp -r /backup/rustdesk-YYYYMMDD/* /opt/rustdesk/

# 3. 恢复服务
sudo systemctl start rustdesk-hbbs
sudo systemctl start rustdesk-hbbr
```

## 📞 升级支持

### 获取帮助
- 📖 [升级FAQ](https://github.com/ljhlovehui/rustdesk-server/wiki/Upgrade-FAQ)
- 💬 [技术支持](mailto:support@rustdesk.com)
- 🐛 [问题报告](https://github.com/ljhlovehui/rustdesk-server/issues)

### 专业服务
- 🎯 **升级咨询**: 免费升级指导
- 🛠️ **迁移服务**: 专业迁移支持
- 📞 **紧急支持**: 24/7 技术支持

---

**升级前请务必备份数据！** ⚠️