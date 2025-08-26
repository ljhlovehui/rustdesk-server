// 企业管理模块 - 用户组管理、权限控制、设备分组
use crate::auth::{User, UserRole};
use crate::enterprise_database::EnterpriseDatabase;
use hbb_common::{log, ResultType};
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserGroup {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub members: Vec<String>, // user IDs
    pub permissions: GroupPermissions,
    pub device_access: DeviceAccess,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupPermissions {
    // 系统权限
    pub can_manage_users: bool,
    pub can_manage_groups: bool,
    pub can_manage_devices: bool,
    pub can_view_audit_logs: bool,
    pub can_manage_settings: bool,
    
    // 连接权限
    pub can_control_devices: bool,
    pub can_view_screens: bool,
    pub can_transfer_files: bool,
    pub can_use_clipboard: bool,
    pub can_use_audio: bool,
    pub can_record_sessions: bool,
    
    // 文件传输权限
    pub max_file_size: u64,
    pub allowed_file_types: Vec<String>,
    pub blocked_file_types: Vec<String>,
    pub can_upload: bool,
    pub can_download: bool,
    pub can_sync_folders: bool,
    
    // 时间限制
    pub session_timeout: Option<Duration>,
    pub daily_time_limit: Option<Duration>,
    pub allowed_hours: Option<TimeRange>,
    pub allowed_days: Vec<u8>, // 0=Sunday, 1=Monday, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start_hour: u8, // 0-23
    pub start_minute: u8, // 0-59
    pub end_hour: u8,
    pub end_minute: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAccess {
    pub access_type: AccessType,
    pub device_groups: Vec<String>, // device group IDs
    pub specific_devices: Vec<String>, // specific device IDs
    pub excluded_devices: Vec<String>, // excluded device IDs
    pub ip_restrictions: Vec<IpRestriction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessType {
    All,           // 访问所有设备
    GroupOnly,     // 只能访问指定组的设备
    SpecificOnly,  // 只能访问指定的设备
    Restricted,    // 受限访问（排除某些设备）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpRestriction {
    pub ip_range: String, // CIDR格式，如 "192.168.1.0/24"
    pub allow: bool,      // true=允许，false=拒绝
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceGroup {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub devices: Vec<String>, // device IDs
    pub parent_group: Option<String>, // 支持层级结构
    pub child_groups: Vec<String>,
    pub tags: Vec<String>,
    pub auto_assignment_rules: Vec<AutoAssignmentRule>,
    pub monitoring_settings: MonitoringSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoAssignmentRule {
    pub rule_type: RuleType,
    pub condition: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleType {
    IpRange,      // IP地址范围
    Hostname,     // 主机名模式
    Os,           // 操作系统
    Tag,          // 设备标签
    Owner,        // 设备所有者
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringSettings {
    pub enable_monitoring: bool,
    pub alert_on_offline: bool,
    pub offline_threshold_minutes: u32,
    pub alert_on_unauthorized_access: bool,
    pub alert_recipients: Vec<String>, // email addresses
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: PermissionCategory,
    pub required_role: UserRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionCategory {
    System,
    Device,
    User,
    Audit,
    FileTransfer,
    Session,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRequest {
    pub id: String,
    pub user_id: String,
    pub device_id: String,
    pub requested_permissions: Vec<String>,
    pub reason: Option<String>,
    pub requested_at: SystemTime,
    pub expires_at: Option<SystemTime>,
    pub status: RequestStatus,
    pub approved_by: Option<String>,
    pub approved_at: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

pub struct EnterpriseManager {
    db: EnterpriseDatabase,
    user_groups: Arc<RwLock<HashMap<String, UserGroup>>>,
    device_groups: Arc<RwLock<HashMap<String, DeviceGroup>>>,
    permissions: Arc<RwLock<HashMap<String, Permission>>>,
    access_requests: Arc<RwLock<HashMap<String, AccessRequest>>>,
    active_sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
}

#[derive(Debug, Clone)]
struct SessionInfo {
    user_id: String,
    device_id: String,
    start_time: SystemTime,
    last_activity: SystemTime,
    permissions: Vec<String>,
    ip_address: String,
}

impl EnterpriseManager {
    pub fn new(db: EnterpriseDatabase) -> Self {
        Self {
            db,
            user_groups: Arc::new(RwLock::new(HashMap::new())),
            device_groups: Arc::new(RwLock::new(HashMap::new())),
            permissions: Arc::new(RwLock::new(HashMap::new())),
            access_requests: Arc::new(RwLock::new(HashMap::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn initialize(&self) -> ResultType<()> {
        self.load_default_permissions().await?;
        self.load_user_groups().await?;
        self.load_device_groups().await?;
        Ok(())
    }

    // 用户组管理
    pub async fn create_user_group(&self, group: UserGroup) -> ResultType<String> {
        // 验证组名唯一性
        let groups = self.user_groups.read().await;
        if groups.values().any(|g| g.name == group.name) {
            return Err("Group name already exists".into());
        }
        drop(groups);

        // 保存到数据库
        self.db.create_user_group(&group).await?;
        
        // 更新内存缓存
        self.user_groups.write().await.insert(group.id.clone(), group.clone());
        
        log::info!("Created user group: {} ({})", group.name, group.id);
        Ok(group.id)
    }

    pub async fn update_user_group(&self, group: UserGroup) -> ResultType<()> {
        // 更新数据库
        self.db.update_user_group(&group).await?;
        
        // 更新内存缓存
        self.user_groups.write().await.insert(group.id.clone(), group.clone());
        
        log::info!("Updated user group: {} ({})", group.name, group.id);
        Ok(())
    }

    pub async fn delete_user_group(&self, group_id: &str) -> ResultType<()> {
        // 检查是否有用户在此组中
        let groups = self.user_groups.read().await;
        if let Some(group) = groups.get(group_id) {
            if !group.members.is_empty() {
                return Err("Cannot delete group with members".into());
            }
        }
        drop(groups);

        // 从数据库删除
        self.db.delete_user_group(group_id).await?;
        
        // 从内存删除
        self.user_groups.write().await.remove(group_id);
        
        log::info!("Deleted user group: {}", group_id);
        Ok(())
    }

    pub async fn add_user_to_group(&self, user_id: &str, group_id: &str) -> ResultType<()> {
        let mut groups = self.user_groups.write().await;
        if let Some(group) = groups.get_mut(group_id) {
            if !group.members.contains(&user_id.to_string()) {
                group.members.push(user_id.to_string());
                group.updated_at = SystemTime::now();
                
                // 更新数据库
                self.db.update_user_group(group).await?;
                
                log::info!("Added user {} to group {}", user_id, group_id);
            }
            Ok(())
        } else {
            Err("Group not found".into())
        }
    }

    pub async fn remove_user_from_group(&self, user_id: &str, group_id: &str) -> ResultType<()> {
        let mut groups = self.user_groups.write().await;
        if let Some(group) = groups.get_mut(group_id) {
            group.members.retain(|id| id != user_id);
            group.updated_at = SystemTime::now();
            
            // 更新数据库
            self.db.update_user_group(group).await?;
            
            log::info!("Removed user {} from group {}", user_id, group_id);
            Ok(())
        } else {
            Err("Group not found".into())
        }
    }

    // 设备组管理
    pub async fn create_device_group(&self, group: DeviceGroup) -> ResultType<String> {
        // 验证组名唯一性
        let groups = self.device_groups.read().await;
        if groups.values().any(|g| g.name == group.name) {
            return Err("Device group name already exists".into());
        }
        drop(groups);

        // 保存到数据库
        self.db.create_device_group(&group).await?;
        
        // 更新内存缓存
        self.device_groups.write().await.insert(group.id.clone(), group.clone());
        
        log::info!("Created device group: {} ({})", group.name, group.id);
        Ok(group.id)
    }

    pub async fn auto_assign_device_to_groups(&self, device_id: &str, device_info: &crate::enterprise_database::DeviceInfo) -> ResultType<()> {
        let groups = self.device_groups.read().await;
        let mut assigned_groups = Vec::new();

        for (group_id, group) in groups.iter() {
            for rule in &group.auto_assignment_rules {
                if self.evaluate_assignment_rule(rule, device_info) {
                    assigned_groups.push(group_id.clone());
                    break;
                }
            }
        }
        drop(groups);

        // 将设备分配到匹配的组
        for group_id in assigned_groups {
            self.add_device_to_group(device_id, &group_id).await?;
        }

        Ok(())
    }

    fn evaluate_assignment_rule(&self, rule: &AutoAssignmentRule, device_info: &crate::enterprise_database::DeviceInfo) -> bool {
        match rule.rule_type {
            RuleType::IpRange => {
                // 检查IP是否在指定范围内
                self.ip_in_range(&device_info.ip_address, &rule.value)
            }
            RuleType::Hostname => {
                // 检查主机名是否匹配模式
                device_info.name.contains(&rule.value)
            }
            RuleType::Os => {
                // 检查操作系统
                device_info.os.to_lowercase().contains(&rule.value.to_lowercase())
            }
            RuleType::Tag => {
                // 检查设备标签
                device_info.tags.contains(&rule.value)
            }
            RuleType::Owner => {
                // 检查设备所有者
                device_info.owner_id == rule.value
            }
        }
    }

    fn ip_in_range(&self, ip: &str, cidr: &str) -> bool {
        use ipnetwork::IpNetwork;
        use std::net::IpAddr;

        if let (Ok(ip_addr), Ok(network)) = (ip.parse::<IpAddr>(), cidr.parse::<IpNetwork>()) {
            network.contains(ip_addr)
        } else {
            false
        }
    }

    pub async fn add_device_to_group(&self, device_id: &str, group_id: &str) -> ResultType<()> {
        let mut groups = self.device_groups.write().await;
        if let Some(group) = groups.get_mut(group_id) {
            if !group.devices.contains(&device_id.to_string()) {
                group.devices.push(device_id.to_string());
                group.updated_at = SystemTime::now();
                
                // 更新数据库
                self.db.update_device_group(group).await?;
                
                log::info!("Added device {} to group {}", device_id, group_id);
            }
            Ok(())
        } else {
            Err("Device group not found".into())
        }
    }

    // 权限检查
    pub async fn check_user_permission(&self, user_id: &str, permission: &str, device_id: Option<&str>) -> bool {
        // 获取用户信息
        let user = match self.db.get_user_by_id(user_id).await {
            Ok(Some(user)) => user,
            _ => return false,
        };

        // 超级管理员拥有所有权限
        if user.role == crate::auth::UserRole::SuperAdmin {
            return true;
        }

        // 检查用户组权限
        let user_groups = self.get_user_groups(user_id).await;
        for group in user_groups {
            if self.group_has_permission(&group, permission) {
                // 如果涉及设备访问，检查设备权限
                if let Some(device_id) = device_id {
                    if self.check_device_access(&group, device_id).await {
                        return true;
                    }
                } else {
                    return true;
                }
            }
        }

        false
    }

    async fn get_user_groups(&self, user_id: &str) -> Vec<UserGroup> {
        let groups = self.user_groups.read().await;
        groups.values()
            .filter(|group| group.members.contains(&user_id.to_string()))
            .cloned()
            .collect()
    }

    fn group_has_permission(&self, group: &UserGroup, permission: &str) -> bool {
        match permission {
            "manage_users" => group.permissions.can_manage_users,
            "manage_groups" => group.permissions.can_manage_groups,
            "manage_devices" => group.permissions.can_manage_devices,
            "view_audit_logs" => group.permissions.can_view_audit_logs,
            "manage_settings" => group.permissions.can_manage_settings,
            "control_devices" => group.permissions.can_control_devices,
            "view_screens" => group.permissions.can_view_screens,
            "transfer_files" => group.permissions.can_transfer_files,
            "use_clipboard" => group.permissions.can_use_clipboard,
            "use_audio" => group.permissions.can_use_audio,
            "record_sessions" => group.permissions.can_record_sessions,
            _ => false,
        }
    }

    async fn check_device_access(&self, group: &UserGroup, device_id: &str) -> bool {
        match group.device_access.access_type {
            AccessType::All => {
                // 检查是否在排除列表中
                !group.device_access.excluded_devices.contains(&device_id.to_string())
            }
            AccessType::SpecificOnly => {
                group.device_access.specific_devices.contains(&device_id.to_string())
            }
            AccessType::GroupOnly => {
                // 检查设备是否在允许的组中
                self.device_in_allowed_groups(device_id, &group.device_access.device_groups).await
            }
            AccessType::Restricted => {
                // 允许访问，但排除特定设备
                !group.device_access.excluded_devices.contains(&device_id.to_string())
            }
        }
    }

    async fn device_in_allowed_groups(&self, device_id: &str, allowed_groups: &[String]) -> bool {
        let device_groups = self.device_groups.read().await;
        for group_id in allowed_groups {
            if let Some(group) = device_groups.get(group_id) {
                if group.devices.contains(&device_id.to_string()) {
                    return true;
                }
            }
        }
        false
    }

    // 会话管理
    pub async fn start_session(&self, user_id: &str, device_id: &str, ip_address: &str, permissions: Vec<String>) -> ResultType<String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let session = SessionInfo {
            user_id: user_id.to_string(),
            device_id: device_id.to_string(),
            start_time: SystemTime::now(),
            last_activity: SystemTime::now(),
            permissions,
            ip_address: ip_address.to_string(),
        };

        self.active_sessions.write().await.insert(session_id.clone(), session);
        
        log::info!("Started session {} for user {} on device {}", session_id, user_id, device_id);
        Ok(session_id)
    }

    pub async fn end_session(&self, session_id: &str) -> ResultType<()> {
        if let Some(session) = self.active_sessions.write().await.remove(session_id) {
            let duration = SystemTime::now().duration_since(session.start_time).unwrap_or_default();
            log::info!("Ended session {} (duration: {:?})", session_id, duration);
            
            // 记录会话结束到审计日志
            // TODO: 实现审计日志记录
        }
        Ok(())
    }

    pub async fn update_session_activity(&self, session_id: &str) -> ResultType<()> {
        let mut sessions = self.active_sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = SystemTime::now();
        }
        Ok(())
    }

    // 时间限制检查
    pub async fn check_time_restrictions(&self, user_id: &str) -> bool {
        let user_groups = self.get_user_groups(user_id).await;
        
        for group in user_groups {
            if let Some(allowed_hours) = &group.permissions.allowed_hours {
                if !self.is_current_time_allowed(allowed_hours) {
                    return false;
                }
            }
            
            if !group.permissions.allowed_days.is_empty() {
                let current_day = chrono::Utc::now().weekday().num_days_from_sunday() as u8;
                if !group.permissions.allowed_days.contains(&current_day) {
                    return false;
                }
            }
        }
        
        true
    }

    fn is_current_time_allowed(&self, time_range: &TimeRange) -> bool {
        let now = chrono::Local::now();
        let current_hour = now.hour() as u8;
        let current_minute = now.minute() as u8;
        
        let current_minutes = current_hour as u16 * 60 + current_minute as u16;
        let start_minutes = time_range.start_hour as u16 * 60 + time_range.start_minute as u16;
        let end_minutes = time_range.end_hour as u16 * 60 + time_range.end_minute as u16;
        
        if start_minutes <= end_minutes {
            // 同一天内的时间范围
            current_minutes >= start_minutes && current_minutes <= end_minutes
        } else {
            // 跨天的时间范围
            current_minutes >= start_minutes || current_minutes <= end_minutes
        }
    }

    // 加载默认权限
    async fn load_default_permissions(&self) -> ResultType<()> {
        let default_permissions = vec![
            Permission {
                id: "manage_users".to_string(),
                name: "管理用户".to_string(),
                description: "创建、编辑、删除用户账户".to_string(),
                category: PermissionCategory::User,
                required_role: crate::auth::UserRole::Admin,
            },
            Permission {
                id: "manage_groups".to_string(),
                name: "管理用户组".to_string(),
                description: "创建、编辑、删除用户组".to_string(),
                category: PermissionCategory::User,
                required_role: crate::auth::UserRole::Admin,
            },
            Permission {
                id: "manage_devices".to_string(),
                name: "管理设备".to_string(),
                description: "管理设备和设备组".to_string(),
                category: PermissionCategory::Device,
                required_role: crate::auth::UserRole::Admin,
            },
            Permission {
                id: "control_devices".to_string(),
                name: "控制设备".to_string(),
                description: "远程控制设备".to_string(),
                category: PermissionCategory::Device,
                required_role: crate::auth::UserRole::User,
            },
            Permission {
                id: "transfer_files".to_string(),
                name: "文件传输".to_string(),
                description: "上传和下载文件".to_string(),
                category: PermissionCategory::FileTransfer,
                required_role: crate::auth::UserRole::User,
            },
        ];

        let mut permissions = self.permissions.write().await;
        for permission in default_permissions {
            permissions.insert(permission.id.clone(), permission);
        }

        Ok(())
    }

    async fn load_user_groups(&self) -> ResultType<()> {
        // TODO: 从数据库加载用户组
        Ok(())
    }

    async fn load_device_groups(&self) -> ResultType<()> {
        // TODO: 从数据库加载设备组
        Ok(())
    }

    // 获取用户的有效权限
    pub async fn get_user_effective_permissions(&self, user_id: &str) -> Vec<String> {
        let user_groups = self.get_user_groups(user_id).await;
        let mut permissions = HashSet::new();

        for group in user_groups {
            if group.permissions.can_manage_users { permissions.insert("manage_users".to_string()); }
            if group.permissions.can_manage_groups { permissions.insert("manage_groups".to_string()); }
            if group.permissions.can_manage_devices { permissions.insert("manage_devices".to_string()); }
            if group.permissions.can_view_audit_logs { permissions.insert("view_audit_logs".to_string()); }
            if group.permissions.can_manage_settings { permissions.insert("manage_settings".to_string()); }
            if group.permissions.can_control_devices { permissions.insert("control_devices".to_string()); }
            if group.permissions.can_view_screens { permissions.insert("view_screens".to_string()); }
            if group.permissions.can_transfer_files { permissions.insert("transfer_files".to_string()); }
            if group.permissions.can_use_clipboard { permissions.insert("use_clipboard".to_string()); }
            if group.permissions.can_use_audio { permissions.insert("use_audio".to_string()); }
            if group.permissions.can_record_sessions { permissions.insert("record_sessions".to_string()); }
        }

        permissions.into_iter().collect()
    }
}

// 扩展数据库接口
impl EnterpriseDatabase {
    pub async fn create_user_group(&self, group: &UserGroup) -> ResultType<()> {
        // TODO: 实现用户组创建
        Ok(())
    }

    pub async fn update_user_group(&self, group: &UserGroup) -> ResultType<()> {
        // TODO: 实现用户组更新
        Ok(())
    }

    pub async fn delete_user_group(&self, group_id: &str) -> ResultType<()> {
        // TODO: 实现用户组删除
        Ok(())
    }

    pub async fn create_device_group(&self, group: &DeviceGroup) -> ResultType<()> {
        // TODO: 实现设备组创建
        Ok(())
    }

    pub async fn update_device_group(&self, group: &DeviceGroup) -> ResultType<()> {
        // TODO: 实现设备组更新
        Ok(())
    }

    pub async fn get_user_by_id(&self, user_id: &str) -> ResultType<Option<User>> {
        // TODO: 实现根据ID获取用户
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_permission_check() {
        // TODO: 实现权限检查测试
    }

    #[tokio::test]
    async fn test_device_auto_assignment() {
        // TODO: 实现设备自动分配测试
    }
}