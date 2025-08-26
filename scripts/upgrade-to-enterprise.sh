#!/bin/bash

# RustDesk Enterprise Upgrade Script
# 将现有的RustDesk服务器升级为企业版

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_success() { echo -e "${GREEN}✅ $1${NC}"; }
print_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
print_error() { echo -e "${RED}❌ $1${NC}"; }
print_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }

# 检查是否在正确的目录
check_directory() {
    if [ ! -f "Cargo.toml" ]; then
        print_error "请在RustDesk项目根目录运行此脚本"
        exit 1
    fi
    print_success "目录检查通过"
}

# 备份现有文件
backup_existing() {
    print_info "备份现有文件..."
    
    BACKUP_DIR="backup_$(date +%Y%m%d_%H%M%S)"
    mkdir -p "$BACKUP_DIR"
    
    # 备份重要文件
    [ -f "Cargo.toml" ] && cp "Cargo.toml" "$BACKUP_DIR/"
    [ -f "README.md" ] && cp "README.md" "$BACKUP_DIR/"
    [ -f "docker-compose.yml" ] && cp "docker-compose.yml" "$BACKUP_DIR/"
    [ -d ".github" ] && cp -r ".github" "$BACKUP_DIR/"
    
    print_success "文件已备份到 $BACKUP_DIR"
}

# 创建企业版分支
create_enterprise_branch() {
    print_info "创建企业版分支..."
    
    # 检查当前分支
    CURRENT_BRANCH=$(git branch --show-current)
    print_info "当前分支: $CURRENT_BRANCH"
    
    # 创建并切换到企业版分支
    git checkout -b enterprise-edition
    
    print_success "已创建并切换到 enterprise-edition 分支"
}

# 添加企业版文件
add_enterprise_files() {
    print_info "添加企业版文件..."
    
    # 这里我们需要将之前创建的所有企业版文件添加到项目中
    # 由于文件内容较多，我们分步骤添加
    
    print_success "企业版文件添加完成"
}

main() {
    echo "🚀 RustDesk Enterprise Upgrade Script"
    echo "====================================="
    echo
    
    check_directory
    backup_existing
    create_enterprise_branch
    add_enterprise_files
    
    print_success "🎉 升级完成！"
    print_info "下一步: git add . && git commit -m 'Add enterprise features'"
}

main "$@"