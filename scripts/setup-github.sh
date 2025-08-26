#!/bin/bash

# RustDesk Enterprise Server GitHub Setup Script
# 此脚本帮助您将项目上传到GitHub并设置自动化

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 打印彩色消息
print_message() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

print_success() {
    print_message $GREEN "✅ $1"
}

print_warning() {
    print_message $YELLOW "⚠️  $1"
}

print_error() {
    print_message $RED "❌ $1"
}

print_info() {
    print_message $BLUE "ℹ️  $1"
}

# 检查必要工具
check_requirements() {
    print_info "检查必要工具..."
    
    if ! command -v git &> /dev/null; then
        print_error "Git 未安装，请先安装 Git"
        exit 1
    fi
    
    if ! command -v gh &> /dev/null; then
        print_warning "GitHub CLI 未安装，建议安装以便自动创建仓库"
        print_info "安装命令: https://cli.github.com/"
    fi
    
    print_success "工具检查完成"
}

# 获取用户输入
get_user_input() {
    print_info "请提供以下信息："
    
    # GitHub用户名
    read -p "GitHub 用户名: " GITHUB_USERNAME
    if [ -z "$GITHUB_USERNAME" ]; then
        print_error "GitHub用户名不能为空"
        exit 1
    fi
    
    # 仓库名称
    read -p "仓库名称 [rustdesk-enterprise-server]: " REPO_NAME
    REPO_NAME=${REPO_NAME:-rustdesk-enterprise-server}
    
    # 仓库描述
    read -p "仓库描述 [RustDesk Enterprise Server with advanced features]: " REPO_DESCRIPTION
    REPO_DESCRIPTION=${REPO_DESCRIPTION:-"RustDesk Enterprise Server with advanced features"}
    
    # 是否私有仓库
    read -p "创建私有仓库? (y/N): " IS_PRIVATE
    IS_PRIVATE=${IS_PRIVATE:-n}
    
    # 域名配置
    read -p "您的域名 (可选): " DOMAIN_NAME
    
    print_success "用户输入收集完成"
}

# 初始化Git仓库
init_git_repo() {
    print_info "初始化Git仓库..."
    
    if [ ! -d ".git" ]; then
        git init
        print_success "Git仓库初始化完成"
    else
        print_warning "Git仓库已存在"
    fi
    
    # 设置Git配置
    if [ -z "$(git config user.name)" ]; then
        read -p "Git 用户名: " GIT_USERNAME
        git config user.name "$GIT_USERNAME"
    fi
    
    if [ -z "$(git config user.email)" ]; then
        read -p "Git 邮箱: " GIT_EMAIL
        git config user.email "$GIT_EMAIL"
    fi
    
    print_success "Git配置完成"
}

# 创建.gitignore文件
create_gitignore() {
    print_info "创建.gitignore文件..."
    
    cat > .gitignore << 'EOF'
# Rust
/target/
**/*.rs.bk
Cargo.lock

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Logs
*.log
logs/

# Database
*.sqlite3
*.sqlite3-journal
*.db

# Environment variables
.env
.env.local
.env.production

# Data directories
data/
backups/
temp/

# SSL certificates
*.pem
*.key
*.crt
*.p12

# Docker
.dockerignore

# Build artifacts
dist/
build/

# Temporary files
tmp/
temp/
*.tmp

# Node modules (for web interface)
node_modules/
npm-debug.log*
yarn-debug.log*
yarn-error.log*

# Coverage reports
coverage/
*.lcov

# Dependency directories
vendor/

# Configuration files with secrets
config.toml
config.yaml
config.json

# Backup files
*.bak
*.backup

# Archive files
*.tar.gz
*.zip
*.rar
EOF

    print_success ".gitignore文件创建完成"
}

# 更新README中的用户名
update_readme() {
    print_info "更新README文件..."
    
    if [ -f "README.md" ]; then
        # 替换用户名占位符
        sed -i.bak "s/your-username/$GITHUB_USERNAME/g" README.md
        
        # 如果提供了域名，更新域名配置
        if [ -n "$DOMAIN_NAME" ]; then
            sed -i.bak "s/your-server\.com/$DOMAIN_NAME/g" README.md
            sed -i.bak "s/yourdomain\.com/$DOMAIN_NAME/g" README.md
        fi
        
        # 删除备份文件
        rm -f README.md.bak
        
        print_success "README文件更新完成"
    fi
}

# 更新GitHub Actions配置
update_github_actions() {
    print_info "更新GitHub Actions配置..."
    
    if [ -f ".github/workflows/build-enterprise.yml" ]; then
        # 替换用户名占位符
        sed -i.bak "s/your-username/$GITHUB_USERNAME/g" .github/workflows/build-enterprise.yml
        
        # 删除备份文件
        rm -f .github/workflows/build-enterprise.yml.bak
        
        print_success "GitHub Actions配置更新完成"
    fi
}

# 更新环境变量示例
update_env_example() {
    print_info "更新环境变量示例..."
    
    if [ -f ".env.example" ]; then
        # 如果提供了域名，更新域名配置
        if [ -n "$DOMAIN_NAME" ]; then
            sed -i.bak "s/yourdomain\.com/$DOMAIN_NAME/g" .env.example
            rm -f .env.example.bak
        fi
        
        print_success "环境变量示例更新完成"
    fi
}

# 创建GitHub仓库
create_github_repo() {
    print_info "创建GitHub仓库..."
    
    if command -v gh &> /dev/null; then
        # 使用GitHub CLI创建仓库
        local visibility_flag=""
        if [ "$IS_PRIVATE" = "y" ] || [ "$IS_PRIVATE" = "Y" ]; then
            visibility_flag="--private"
        else
            visibility_flag="--public"
        fi
        
        gh repo create "$REPO_NAME" \
            --description "$REPO_DESCRIPTION" \
            $visibility_flag \
            --source=. \
            --remote=origin \
            --push
        
        print_success "GitHub仓库创建完成"
    else
        # 手动设置远程仓库
        git remote add origin "https://github.com/$GITHUB_USERNAME/$REPO_NAME.git"
        
        print_warning "请手动在GitHub上创建仓库: https://github.com/new"
        print_info "仓库名称: $REPO_NAME"
        print_info "仓库描述: $REPO_DESCRIPTION"
        print_info "创建完成后按回车继续..."
        read
    fi
}

# 提交并推送代码
commit_and_push() {
    print_info "提交并推送代码..."
    
    # 添加所有文件
    git add .
    
    # 创建初始提交
    git commit -m "🎉 Initial commit: RustDesk Enterprise Server

✨ Features:
- Complete user authentication and authorization system
- Advanced file transfer with resume capability  
- Enterprise-grade security with 2FA and E2E encryption
- Web-based management interface
- Performance optimization and monitoring
- Multi-platform support with automated builds

📦 Includes:
- Docker deployment configuration
- GitHub Actions CI/CD pipeline
- Comprehensive documentation
- Multi-architecture builds
- Health checks and monitoring"

    # 设置主分支
    git branch -M main
    
    # 推送到GitHub
    if ! git push -u origin main; then
        print_error "推送失败，请检查仓库权限"
        print_info "手动推送命令:"
        print_info "git remote add origin https://github.com/$GITHUB_USERNAME/$REPO_NAME.git"
        print_info "git branch -M main"
        print_info "git push -u origin main"
        exit 1
    fi
    
    print_success "代码推送完成"
}

# 设置GitHub Secrets
setup_github_secrets() {
    print_info "设置GitHub Secrets..."
    
    if command -v gh &> /dev/null; then
        print_info "请设置以下GitHub Secrets用于自动化构建:"
        print_info "1. DOCKERHUB_USERNAME - Docker Hub用户名"
        print_info "2. DOCKERHUB_TOKEN - Docker Hub访问令牌"
        
        read -p "是否现在设置Docker Hub凭据? (y/N): " SETUP_DOCKER
        if [ "$SETUP_DOCKER" = "y" ] || [ "$SETUP_DOCKER" = "Y" ]; then
            read -p "Docker Hub用户名: " DOCKER_USERNAME
            read -s -p "Docker Hub令牌: " DOCKER_TOKEN
            echo
            
            gh secret set DOCKERHUB_USERNAME --body "$DOCKER_USERNAME"
            gh secret set DOCKERHUB_TOKEN --body "$DOCKER_TOKEN"
            
            print_success "Docker Hub凭据设置完成"
        fi
    else
        print_warning "请手动在GitHub仓库设置中添加以下Secrets:"
        print_info "Settings -> Secrets and variables -> Actions"
        print_info "1. DOCKERHUB_USERNAME - Docker Hub用户名"
        print_info "2. DOCKERHUB_TOKEN - Docker Hub访问令牌"
    fi
}

# 创建发布说明模板
create_release_template() {
    print_info "创建发布说明模板..."
    
    mkdir -p .github
    
    cat > .github/PULL_REQUEST_TEMPLATE.md << 'EOF'
## 变更说明

### 变更类型
- [ ] 🐛 Bug修复
- [ ] ✨ 新功能
- [ ] 💥 破坏性变更
- [ ] 📚 文档更新
- [ ] 🎨 代码格式化
- [ ] ♻️ 代码重构
- [ ] ⚡ 性能优化
- [ ] 🔒 安全修复

### 变更描述


### 测试
- [ ] 已添加测试用例
- [ ] 所有测试通过
- [ ] 手动测试完成

### 检查清单
- [ ] 代码遵循项目规范
- [ ] 已更新相关文档
- [ ] 已测试向后兼容性
- [ ] 已更新CHANGELOG
EOF

    cat > .github/ISSUE_TEMPLATE/bug_report.md << 'EOF'
---
name: Bug报告
about: 创建Bug报告帮助我们改进
title: '[BUG] '
labels: bug
assignees: ''
---

## Bug描述
简洁清晰地描述Bug。

## 复现步骤
1. 执行 '...'
2. 点击 '....'
3. 滚动到 '....'
4. 看到错误

## 期望行为
清晰简洁地描述您期望发生的事情。

## 实际行为
清晰简洁地描述实际发生的事情。

## 环境信息
- OS: [例如 Ubuntu 20.04]
- 版本: [例如 v1.0.0]
- 部署方式: [Docker/二进制]

## 日志信息
```
粘贴相关日志信息
```

## 附加信息
添加任何其他关于问题的上下文信息。
EOF

    cat > .github/ISSUE_TEMPLATE/feature_request.md << 'EOF'
---
name: 功能请求
about: 建议新功能
title: '[FEATURE] '
labels: enhancement
assignees: ''
---

## 功能描述
清晰简洁地描述您想要的功能。

## 问题描述
清晰简洁地描述问题。例如：我总是感到沮丧当[...]

## 解决方案
清晰简洁地描述您想要的解决方案。

## 替代方案
清晰简洁地描述您考虑过的任何替代解决方案或功能。

## 附加信息
添加任何其他关于功能请求的上下文或截图。
EOF

    print_success "GitHub模板创建完成"
}

# 显示完成信息
show_completion_info() {
    print_success "🎉 GitHub仓库设置完成！"
    echo
    print_info "仓库信息:"
    print_info "📍 仓库地址: https://github.com/$GITHUB_USERNAME/$REPO_NAME"
    print_info "🌐 Actions: https://github.com/$GITHUB_USERNAME/$REPO_NAME/actions"
    print_info "📋 Issues: https://github.com/$GITHUB_USERNAME/$REPO_NAME/issues"
    print_info "📦 Releases: https://github.com/$GITHUB_USERNAME/$REPO_NAME/releases"
    echo
    print_info "下一步操作:"
    print_info "1. 等待GitHub Actions完成首次构建"
    print_info "2. 检查构建状态和下载链接"
    print_info "3. 创建第一个Release标签触发发布"
    print_info "4. 配置域名和SSL证书"
    print_info "5. 部署到生产环境"
    echo
    print_info "快速部署命令:"
    echo "git clone https://github.com/$GITHUB_USERNAME/$REPO_NAME.git"
    echo "cd $REPO_NAME"
    echo "cp .env.example .env"
    echo "# 编辑 .env 文件设置您的配置"
    echo "docker-compose -f docker-compose-enterprise.yml up -d"
    echo
    print_success "感谢使用RustDesk Enterprise Server！"
}

# 主函数
main() {
    echo "🚀 RustDesk Enterprise Server GitHub Setup"
    echo "=========================================="
    echo
    
    check_requirements
    get_user_input
    init_git_repo
    create_gitignore
    update_readme
    update_github_actions
    update_env_example
    create_release_template
    create_github_repo
    commit_and_push
    setup_github_secrets
    show_completion_info
}

# 运行主函数
main "$@"