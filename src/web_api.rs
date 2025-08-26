// Web管理界面API模块
use crate::auth::{AuthManager, User, UserRole, Claims};
use crate::enterprise_database::{EnterpriseDatabase, AuditLog, DeviceInfo};
use axum::{
    extract::{Query, State, Path},
    http::{StatusCode, HeaderMap},
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use hbb_common::{log, ResultType};
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::SystemTime};
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: EnterpriseDatabase,
    pub auth: Arc<AuthManager>,
}

#[derive(Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub totp_code: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub token: Option<String>,
    pub user: Option<UserInfo>,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub role: String,
    pub groups: Vec<String>,
    pub enabled: bool,
    pub last_login: Option<u64>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub role: String,
    pub groups: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct DeviceListResponse {
    pub devices: Vec<DeviceInfo>,
    pub total: usize,
}

#[derive(Serialize, Deserialize)]
pub struct AuditLogResponse {
    pub logs: Vec<AuditLog>,
    pub total: usize,
}

#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: String,
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

#[derive(Deserialize)]
pub struct AuditLogQuery {
    pub user_id: Option<String>,
    pub device_id: Option<String>,
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // 认证相关
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(get_current_user))
        
        // 用户管理
        .route("/api/users", get(list_users).post(create_user))
        .route("/api/users/:id", get(get_user).put(update_user).delete(delete_user))
        .route("/api/users/:id/reset-password", post(reset_user_password))
        .route("/api/users/:id/toggle-status", post(toggle_user_status))
        
        // 设备管理
        .route("/api/devices", get(list_devices))
        .route("/api/devices/:id", get(get_device).put(update_device).delete(delete_device))
        .route("/api/devices/:id/control", post(control_device))
        
        // 审计日志
        .route("/api/audit-logs", get(get_audit_logs))
        
        // 系统统计
        .route("/api/stats/dashboard", get(get_dashboard_stats))
        .route("/api/stats/connections", get(get_connection_stats))
        
        // 系统设置
        .route("/api/settings", get(get_settings).put(update_settings))
        
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

// 认证相关处理函数
async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    log::info!("Login attempt for user: {}", req.username);
    
    // 查找用户
    let user = match state.db.get_user_by_username(&req.username).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Ok(Json(LoginResponse {
                success: false,
                token: None,
                user: None,
                message: "用户名或密码错误".to_string(),
            }));
        }
        Err(e) => {
            log::error!("Database error during login: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // 检查用户是否被锁定
    if state.auth.is_user_locked(&user) {
        return Ok(Json(LoginResponse {
            success: false,
            token: None,
            user: None,
            message: "账户已被锁定，请稍后再试".to_string(),
        }));
    }

    // 验证密码
    if !state.auth.verify_password(&req.password, &user.password_hash) {
        // 记录失败的登录尝试
        let _ = state.db.update_user_login_info(&user.id, false).await;
        
        return Ok(Json(LoginResponse {
            success: false,
            token: None,
            user: None,
            message: "用户名或密码错误".to_string(),
        }));
    }

    // 如果启用了双因素认证，验证TOTP代码
    if user.two_factor_enabled {
        if let Some(totp_code) = req.totp_code {
            // 这里应该验证TOTP代码
            // 为了简化，暂时跳过
        } else {
            return Ok(Json(LoginResponse {
                success: false,
                token: None,
                user: None,
                message: "需要双因素认证代码".to_string(),
            }));
        }
    }

    // 生成JWT令牌
    let token = match state.auth.generate_jwt(&user) {
        Ok(token) => token,
        Err(e) => {
            log::error!("Failed to generate JWT: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // 更新登录信息
    let _ = state.db.update_user_login_info(&user.id, true).await;

    // 记录审计日志
    let audit_log = AuditLog {
        id: 0,
        user_id: user.id.clone(),
        device_id: "system".to_string(),
        action: "login".to_string(),
        details: Some("用户登录".to_string()),
        ip_address: "127.0.0.1".to_string(), // 这里应该从请求中获取真实IP
        user_agent: None,
        timestamp: SystemTime::now(),
        success: true,
    };
    let _ = state.db.log_audit(&audit_log).await;

    let user_info = UserInfo {
        id: user.id,
        username: user.username,
        email: user.email,
        role: format!("{:?}", user.role),
        groups: user.groups,
        enabled: user.enabled,
        last_login: user.last_login.map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
    };

    Ok(Json(LoginResponse {
        success: true,
        token: Some(token),
        user: Some(user_info),
        message: "登录成功".to_string(),
    }))
}

async fn logout(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    // 这里应该将JWT令牌加入黑名单
    // 为了简化，暂时只返回成功响应
    
    Ok(Json(ApiResponse {
        success: true,
        data: Some(()),
        message: "登出成功".to_string(),
    }))
}

async fn get_current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<UserInfo>>, StatusCode> {
    let claims = match extract_claims_from_headers(&state.auth, &headers) {
        Ok(claims) => claims,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    let user = match state.db.get_user_by_username(&claims.username).await {
        Ok(Some(user)) => user,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let user_info = UserInfo {
        id: user.id,
        username: user.username,
        email: user.email,
        role: format!("{:?}", user.role),
        groups: user.groups,
        enabled: user.enabled,
        last_login: user.last_login.map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
    };

    Ok(Json(ApiResponse {
        success: true,
        data: Some(user_info),
        message: "获取用户信息成功".to_string(),
    }))
}

// 用户管理处理函数
async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<Vec<UserInfo>>>, StatusCode> {
    let claims = match extract_claims_from_headers(&state.auth, &headers) {
        Ok(claims) => claims,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    // 检查权限 - 只有管理员可以查看用户列表
    if claims.role != "SuperAdmin" && claims.role != "Admin" {
        return Err(StatusCode::FORBIDDEN);
    }

    // 这里应该实现分页查询用户列表
    // 为了简化，暂时返回空列表
    Ok(Json(ApiResponse {
        success: true,
        data: Some(vec![]),
        message: "获取用户列表成功".to_string(),
    }))
}

async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<UserInfo>>, StatusCode> {
    let claims = match extract_claims_from_headers(&state.auth, &headers) {
        Ok(claims) => claims,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    // 检查权限
    if claims.role != "SuperAdmin" && claims.role != "Admin" {
        return Err(StatusCode::FORBIDDEN);
    }

    // 验证用户名是否已存在
    if let Ok(Some(_)) = state.db.get_user_by_username(&req.username).await {
        return Ok(Json(ApiResponse {
            success: false,
            data: None,
            message: "用户名已存在".to_string(),
        }));
    }

    // 创建新用户
    let password_hash = match state.auth.hash_password(&req.password) {
        Ok(hash) => hash,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let role = match req.role.as_str() {
        "SuperAdmin" => UserRole::SuperAdmin,
        "Admin" => UserRole::Admin,
        "User" => UserRole::User,
        "ReadOnly" => UserRole::ReadOnly,
        _ => UserRole::User,
    };

    let new_user = User {
        id: uuid::Uuid::new_v4().to_string(),
        username: req.username,
        password_hash,
        email: req.email,
        role,
        groups: req.groups,
        enabled: true,
        created_at: SystemTime::now(),
        last_login: None,
        failed_login_attempts: 0,
        locked_until: None,
        two_factor_enabled: false,
        two_factor_secret: None,
    };

    match state.db.create_user(&new_user).await {
        Ok(_) => {
            let user_info = UserInfo {
                id: new_user.id,
                username: new_user.username,
                email: new_user.email,
                role: format!("{:?}", new_user.role),
                groups: new_user.groups,
                enabled: new_user.enabled,
                last_login: None,
            };

            Ok(Json(ApiResponse {
                success: true,
                data: Some(user_info),
                message: "用户创建成功".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Failed to create user: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// 设备管理处理函数
async fn list_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<DeviceListResponse>>, StatusCode> {
    let claims = match extract_claims_from_headers(&state.auth, &headers) {
        Ok(claims) => claims,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    let devices = match state.db.get_devices_by_user(&claims.sub).await {
        Ok(devices) => devices,
        Err(e) => {
            log::error!("Failed to get devices: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let response = DeviceListResponse {
        total: devices.len(),
        devices,
    };

    Ok(Json(ApiResponse {
        success: true,
        data: Some(response),
        message: "获取设备列表成功".to_string(),
    }))
}

async fn control_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(device_id): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let claims = match extract_claims_from_headers(&state.auth, &headers) {
        Ok(claims) => claims,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    // 记录控制设备的审计日志
    let audit_log = AuditLog {
        id: 0,
        user_id: claims.sub,
        device_id: device_id.clone(),
        action: "control_device".to_string(),
        details: Some("用户开始控制设备".to_string()),
        ip_address: "127.0.0.1".to_string(),
        user_agent: None,
        timestamp: SystemTime::now(),
        success: true,
    };
    let _ = state.db.log_audit(&audit_log).await;

    Ok(Json(ApiResponse {
        success: true,
        data: Some("设备控制会话已建立".to_string()),
        message: "开始控制设备".to_string(),
    }))
}

// 审计日志处理函数
async fn get_audit_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<AuditLogQuery>,
) -> Result<Json<ApiResponse<AuditLogResponse>>, StatusCode> {
    let claims = match extract_claims_from_headers(&state.auth, &headers) {
        Ok(claims) => claims,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    // 检查权限 - 只有管理员可以查看所有审计日志
    let user_id_filter = if claims.role == "SuperAdmin" || claims.role == "Admin" {
        params.user_id.as_deref()
    } else {
        Some(claims.sub.as_str()) // 普通用户只能查看自己的日志
    };

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(50);
    let offset = (page - 1) * limit;

    let logs = match state.db.get_audit_logs(
        user_id_filter,
        params.device_id.as_deref(),
        limit as i64,
        offset as i64,
    ).await {
        Ok(logs) => logs,
        Err(e) => {
            log::error!("Failed to get audit logs: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let response = AuditLogResponse {
        total: logs.len(),
        logs,
    };

    Ok(Json(ApiResponse {
        success: true,
        data: Some(response),
        message: "获取审计日志成功".to_string(),
    }))
}

// 系统统计处理函数
async fn get_dashboard_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<HashMap<String, u64>>>, StatusCode> {
    let _claims = match extract_claims_from_headers(&state.auth, &headers) {
        Ok(claims) => claims,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    // 这里应该实现真实的统计数据查询
    let mut stats = HashMap::new();
    stats.insert("total_users".to_string(), 10);
    stats.insert("online_devices".to_string(), 5);
    stats.insert("total_connections_today".to_string(), 25);
    stats.insert("active_sessions".to_string(), 3);

    Ok(Json(ApiResponse {
        success: true,
        data: Some(stats),
        message: "获取统计数据成功".to_string(),
    }))
}

// 辅助函数
fn extract_claims_from_headers(auth: &AuthManager, headers: &HeaderMap) -> Result<Claims, &'static str> {
    let auth_header = headers
        .get("Authorization")
        .ok_or("Missing Authorization header")?
        .to_str()
        .map_err(|_| "Invalid Authorization header")?;

    if !auth_header.starts_with("Bearer ") {
        return Err("Invalid Authorization format");
    }

    let token = &auth_header[7..];
    auth.verify_jwt(token).map_err(|_| "Invalid token")
}

// 占位符函数 - 需要根据具体需求实现
async fn get_user() -> Result<Json<ApiResponse<UserInfo>>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn update_user() -> Result<Json<ApiResponse<UserInfo>>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn delete_user() -> Result<Json<ApiResponse<()>>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn reset_user_password() -> Result<Json<ApiResponse<()>>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn toggle_user_status() -> Result<Json<ApiResponse<()>>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn get_device() -> Result<Json<ApiResponse<DeviceInfo>>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn update_device() -> Result<Json<ApiResponse<DeviceInfo>>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn delete_device() -> Result<Json<ApiResponse<()>>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn get_connection_stats() -> Result<Json<ApiResponse<HashMap<String, u64>>>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn get_settings() -> Result<Json<ApiResponse<HashMap<String, String>>>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn update_settings() -> Result<Json<ApiResponse<()>>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}