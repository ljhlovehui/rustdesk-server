// RustDesk 企业版 Web 管理界面
class RustDeskAdmin {
    constructor() {
        this.apiBase = '/api';
        this.token = localStorage.getItem('token');
        this.currentUser = null;
        this.init();
    }

    init() {
        if (this.token) {
            this.validateToken();
        } else {
            this.showLogin();
        }
        this.setupEventListeners();
    }

    setupEventListeners() {
        // 登录表单
        document.getElementById('loginForm').addEventListener('submit', (e) => {
            e.preventDefault();
            this.login();
        });

        // 导航菜单
        document.querySelectorAll('.nav-link[data-page]').forEach(link => {
            link.addEventListener('click', (e) => {
                e.preventDefault();
                const page = e.target.closest('[data-page]').dataset.page;
                this.showPage(page);
                this.updateActiveNav(e.target.closest('[data-page]'));
            });
        });

        // 创建用户表单
        document.getElementById('createUserForm').addEventListener('submit', (e) => {
            e.preventDefault();
            this.createUser();
        });
    }

    async validateToken() {
        try {
            const response = await this.apiCall('/auth/me', 'GET');
            if (response.success) {
                this.currentUser = response.data;
                this.showMainApp();
                this.loadDashboard();
            } else {
                this.showLogin();
            }
        } catch (error) {
            console.error('Token validation failed:', error);
            this.showLogin();
        }
    }

    async login() {
        const username = document.getElementById('username').value;
        const password = document.getElementById('password').value;
        const totpCode = document.getElementById('totpCode').value;

        try {
            const response = await this.apiCall('/auth/login', 'POST', {
                username,
                password,
                totp_code: totpCode || null
            });

            if (response.success) {
                this.token = response.token;
                this.currentUser = response.user;
                localStorage.setItem('token', this.token);
                this.showMainApp();
                this.loadDashboard();
                this.showAlert('登录成功', 'success');
            } else {
                this.showAlert(response.message, 'danger');
            }
        } catch (error) {
            console.error('Login failed:', error);
            this.showAlert('登录失败，请检查网络连接', 'danger');
        }
    }

    async logout() {
        try {
            await this.apiCall('/auth/logout', 'POST');
        } catch (error) {
            console.error('Logout error:', error);
        }
        
        this.token = null;
        this.currentUser = null;
        localStorage.removeItem('token');
        this.showLogin();
    }

    showLogin() {
        document.getElementById('loginPage').style.display = 'flex';
        document.getElementById('mainApp').style.display = 'none';
    }

    showMainApp() {
        document.getElementById('loginPage').style.display = 'none';
        document.getElementById('mainApp').style.display = 'block';
        
        if (this.currentUser) {
            document.getElementById('currentUser').textContent = this.currentUser.username;
        }
    }

    showPage(pageName) {
        // 隐藏所有页面
        document.querySelectorAll('.page-content').forEach(page => {
            page.style.display = 'none';
        });

        // 显示指定页面
        const targetPage = document.getElementById(pageName + 'Page');
        if (targetPage) {
            targetPage.style.display = 'block';
            
            // 加载页面数据
            switch (pageName) {
                case 'dashboard':
                    this.loadDashboard();
                    break;
                case 'devices':
                    this.loadDevices();
                    break;
                case 'users':
                    this.loadUsers();
                    break;
                case 'audit':
                    this.loadAuditLogs();
                    break;
            }
        }
    }

    updateActiveNav(activeElement) {
        document.querySelectorAll('.nav-link').forEach(link => {
            link.classList.remove('active');
        });
        activeElement.classList.add('active');
    }

    async loadDashboard() {
        try {
            const response = await this.apiCall('/stats/dashboard', 'GET');
            if (response.success) {
                const stats = response.data;
                document.getElementById('totalUsers').textContent = stats.total_users || 0;
                document.getElementById('onlineDevices').textContent = stats.online_devices || 0;
                document.getElementById('todayConnections').textContent = stats.total_connections_today || 0;
                document.getElementById('activeSessions').textContent = stats.active_sessions || 0;
            }
        } catch (error) {
            console.error('Failed to load dashboard:', error);
        }

        // 加载最近活动
        this.loadRecentActivity();
    }

    async loadRecentActivity() {
        try {
            const response = await this.apiCall('/audit-logs?limit=10', 'GET');
            if (response.success) {
                const tbody = document.getElementById('recentActivity');
                tbody.innerHTML = '';
                
                if (response.data.logs.length === 0) {
                    tbody.innerHTML = '<tr><td colspan="5" class="text-center">暂无活动记录</td></tr>';
                    return;
                }

                response.data.logs.forEach(log => {
                    const row = document.createElement('tr');
                    const time = new Date(log.timestamp * 1000).toLocaleString();
                    const statusBadge = log.success ? 
                        '<span class="badge bg-success">成功</span>' : 
                        '<span class="badge bg-danger">失败</span>';
                    
                    row.innerHTML = `
                        <td>${time}</td>
                        <td>${log.user_id}</td>
                        <td>${log.device_id}</td>
                        <td>${log.action}</td>
                        <td>${statusBadge}</td>
                    `;
                    tbody.appendChild(row);
                });
            }
        } catch (error) {
            console.error('Failed to load recent activity:', error);
        }
    }

    async loadDevices() {
        try {
            const response = await this.apiCall('/devices', 'GET');
            if (response.success) {
                const tbody = document.getElementById('devicesTable');
                tbody.innerHTML = '';
                
                if (response.data.devices.length === 0) {
                    tbody.innerHTML = '<tr><td colspan="7" class="text-center">暂无设备</td></tr>';
                    return;
                }

                response.data.devices.forEach(device => {
                    const row = document.createElement('tr');
                    const lastOnline = new Date(device.last_online * 1000).toLocaleString();
                    const isOnline = (Date.now() - device.last_online * 1000) < 300000; // 5分钟内算在线
                    const statusIcon = isOnline ? 
                        '<i class="bi bi-circle-fill device-status-online"></i> 在线' : 
                        '<i class="bi bi-circle-fill device-status-offline"></i> 离线';
                    
                    row.innerHTML = `
                        <td>${device.id}</td>
                        <td>${device.name}</td>
                        <td>${device.os}</td>
                        <td>${device.ip_address}</td>
                        <td>${lastOnline}</td>
                        <td>${statusIcon}</td>
                        <td>
                            <button class="btn btn-sm btn-primary" onclick="app.controlDevice('${device.id}')">
                                <i class="bi bi-play-circle"></i> 控制
                            </button>
                            <button class="btn btn-sm btn-secondary" onclick="app.viewDevice('${device.id}')">
                                <i class="bi bi-eye"></i> 查看
                            </button>
                        </td>
                    `;
                    tbody.appendChild(row);
                });
            }
        } catch (error) {
            console.error('Failed to load devices:', error);
        }
    }

    async loadUsers() {
        try {
            const response = await this.apiCall('/users', 'GET');
            if (response.success) {
                const tbody = document.getElementById('usersTable');
                tbody.innerHTML = '';
                
                // 模拟数据，因为后端可能还没实现完整的用户列表
                const mockUsers = [
                    {
                        id: '1',
                        username: 'admin',
                        email: 'admin@rustdesk.local',
                        role: 'SuperAdmin',
                        enabled: true,
                        last_login: Date.now() / 1000 - 3600
                    }
                ];

                mockUsers.forEach(user => {
                    const row = document.createElement('tr');
                    const lastLogin = user.last_login ? 
                        new Date(user.last_login * 1000).toLocaleString() : '从未登录';
                    const statusBadge = user.enabled ? 
                        '<span class="badge bg-success">启用</span>' : 
                        '<span class="badge bg-secondary">禁用</span>';
                    
                    row.innerHTML = `
                        <td>${user.username}</td>
                        <td>${user.email || '-'}</td>
                        <td>${user.role}</td>
                        <td>${statusBadge}</td>
                        <td>${lastLogin}</td>
                        <td>
                            <button class="btn btn-sm btn-warning" onclick="app.editUser('${user.id}')">
                                <i class="bi bi-pencil"></i> 编辑
                            </button>
                            <button class="btn btn-sm btn-danger" onclick="app.deleteUser('${user.id}')">
                                <i class="bi bi-trash"></i> 删除
                            </button>
                        </td>
                    `;
                    tbody.appendChild(row);
                });
            }
        } catch (error) {
            console.error('Failed to load users:', error);
        }
    }

    async loadAuditLogs() {
        try {
            const response = await this.apiCall('/audit-logs?limit=50', 'GET');
            if (response.success) {
                const tbody = document.getElementById('auditTable');
                tbody.innerHTML = '';
                
                if (response.data.logs.length === 0) {
                    tbody.innerHTML = '<tr><td colspan="7" class="text-center">暂无审计日志</td></tr>';
                    return;
                }

                response.data.logs.forEach(log => {
                    const row = document.createElement('tr');
                    const time = new Date(log.timestamp * 1000).toLocaleString();
                    const statusBadge = log.success ? 
                        '<span class="badge bg-success">成功</span>' : 
                        '<span class="badge bg-danger">失败</span>';
                    
                    row.innerHTML = `
                        <td>${time}</td>
                        <td>${log.user_id}</td>
                        <td>${log.device_id}</td>
                        <td>${log.action}</td>
                        <td>${log.details || '-'}</td>
                        <td>${log.ip_address}</td>
                        <td>${statusBadge}</td>
                    `;
                    tbody.appendChild(row);
                });
            }
        } catch (error) {
            console.error('Failed to load audit logs:', error);
        }
    }

    async createUser() {
        const username = document.getElementById('newUsername').value;
        const password = document.getElementById('newPassword').value;
        const email = document.getElementById('newEmail').value;
        const role = document.getElementById('newRole').value;

        try {
            const response = await this.apiCall('/users', 'POST', {
                username,
                password,
                email: email || null,
                role,
                groups: []
            });

            if (response.success) {
                this.showAlert('用户创建成功', 'success');
                bootstrap.Modal.getInstance(document.getElementById('createUserModal')).hide();
                document.getElementById('createUserForm').reset();
                this.loadUsers();
            } else {
                this.showAlert(response.message, 'danger');
            }
        } catch (error) {
            console.error('Failed to create user:', error);
            this.showAlert('创建用户失败', 'danger');
        }
    }

    async controlDevice(deviceId) {
        try {
            const response = await this.apiCall(`/devices/${deviceId}/control`, 'POST');
            if (response.success) {
                this.showAlert('设备控制会话已建立', 'success');
            } else {
                this.showAlert('无法控制设备', 'danger');
            }
        } catch (error) {
            console.error('Failed to control device:', error);
            this.showAlert('控制设备失败', 'danger');
        }
    }

    viewDevice(deviceId) {
        this.showAlert('设备详情功能开发中...', 'info');
    }

    editUser(userId) {
        this.showAlert('编辑用户功能开发中...', 'info');
    }

    deleteUser(userId) {
        if (confirm('确定要删除这个用户吗？')) {
            this.showAlert('删除用户功能开发中...', 'info');
        }
    }

    refreshDevices() {
        this.loadDevices();
        this.showAlert('设备列表已刷新', 'success');
    }

    async apiCall(endpoint, method = 'GET', data = null) {
        const url = this.apiBase + endpoint;
        const options = {
            method,
            headers: {
                'Content-Type': 'application/json',
            }
        };

        if (this.token) {
            options.headers['Authorization'] = `Bearer ${this.token}`;
        }

        if (data) {
            options.body = JSON.stringify(data);
        }

        const response = await fetch(url, options);
        
        if (response.status === 401) {
            this.logout();
            throw new Error('Unauthorized');
        }

        return await response.json();
    }

    showAlert(message, type = 'info') {
        // 创建警告框
        const alertDiv = document.createElement('div');
        alertDiv.className = `alert alert-${type} alert-dismissible fade show position-fixed`;
        alertDiv.style.cssText = 'top: 20px; right: 20px; z-index: 9999; min-width: 300px;';
        alertDiv.innerHTML = `
            ${message}
            <button type="button" class="btn-close" data-bs-dismiss="alert"></button>
        `;

        document.body.appendChild(alertDiv);

        // 3秒后自动消失
        setTimeout(() => {
            if (alertDiv.parentNode) {
                alertDiv.parentNode.removeChild(alertDiv);
            }
        }, 3000);
    }
}

// 全局实例
const app = new RustDeskAdmin();

// 全局函数
function logout() {
    app.logout();
}

function createUser() {
    app.createUser();
}

function refreshDevices() {
    app.refreshDevices();
}