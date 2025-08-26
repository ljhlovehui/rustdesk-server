// 企业级认证模块
use hbb_common::{log, ResultType};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,      // 用户ID
    pub username: String, // 用户名
    pub role: String,     // 角色
    pub groups: Vec<String>, // 用户组
    pub exp: usize,       // 过期时间
    pub iat: usize,       // 签发时间
    pub jti: String,      // JWT ID
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub email: Option<String>,
    pub role: UserRole,
    pub groups: Vec<String>,
    pub enabled: bool,
    pub created_at: SystemTime,
    pub last_login: Option<SystemTime>,
    pub failed_login_attempts: u32,
    pub locked_until: Option<SystemTime>,
    pub two_factor_enabled: bool,
    pub two_factor_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    SuperAdmin,
    Admin,
    User,
    ReadOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub token: String,
    pub created_at: SystemTime,
    pub expires_at: SystemTime,
    pub last_activity: SystemTime,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceGroup {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: SystemTime,
    pub devices: Vec<String>,
    pub permissions: GroupPermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupPermissions {
    pub can_control: bool,
    pub can_transfer_files: bool,
    pub can_view_screen: bool,
    pub can_use_audio: bool,
    pub can_use_clipboard: bool,
    pub session_timeout: Option<Duration>,
}

pub struct AuthManager {
    jwt_secret: String,
    session_timeout: Duration,
    max_failed_attempts: u32,
    lockout_duration: Duration,
}

impl AuthManager {
    pub fn new(jwt_secret: String) -> Self {
        Self {
            jwt_secret,
            session_timeout: Duration::from_hours(8),
            max_failed_attempts: 5,
            lockout_duration: Duration::from_minutes(30),
        }
    }

    pub fn hash_password(&self, password: &str) -> ResultType<String> {
        Ok(hash(password, DEFAULT_COST)?)
    }

    pub fn verify_password(&self, password: &str, hash: &str) -> bool {
        verify(password, hash).unwrap_or(false)
    }

    pub fn generate_jwt(&self, user: &User) -> ResultType<String> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as usize;
        let exp = now + self.session_timeout.as_secs() as usize;

        let claims = Claims {
            sub: user.id.clone(),
            username: user.username.clone(),
            role: format!("{:?}", user.role),
            groups: user.groups.clone(),
            exp,
            iat: now,
            jti: Uuid::new_v4().to_string(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )?;

        Ok(token)
    }

    pub fn verify_jwt(&self, token: &str) -> ResultType<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }

    pub fn is_user_locked(&self, user: &User) -> bool {
        if let Some(locked_until) = user.locked_until {
            SystemTime::now() < locked_until
        } else {
            false
        }
    }

    pub fn should_lock_user(&self, user: &User) -> bool {
        user.failed_login_attempts >= self.max_failed_attempts
    }

    pub fn generate_lockout_time(&self) -> SystemTime {
        SystemTime::now() + self.lockout_duration
    }

    pub fn check_permission(&self, user: &User, device_id: &str, action: &str) -> bool {
        // 超级管理员拥有所有权限
        if user.role == UserRole::SuperAdmin {
            return true;
        }

        // 根据用户角色和设备组权限检查
        match user.role {
            UserRole::Admin => true,
            UserRole::User => {
                // 检查用户是否有权限访问该设备
                // 这里需要查询设备组权限
                true // 简化实现
            }
            UserRole::ReadOnly => {
                matches!(action, "view" | "monitor")
            }
            _ => false,
        }
    }
}

// 双因素认证支持
pub struct TwoFactorAuth {
    secret: String,
}

impl TwoFactorAuth {
    pub fn new() -> Self {
        Self {
            secret: Self::generate_secret(),
        }
    }

    fn generate_secret() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..32)
            .map(|_| {
                let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
                chars[rng.gen_range(0..chars.len())] as char
            })
            .collect()
    }

    pub fn generate_qr_code_url(&self, username: &str, issuer: &str) -> String {
        format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}",
            issuer, username, self.secret, issuer
        )
    }

    pub fn verify_token(&self, token: &str) -> bool {
        // 这里应该实现TOTP验证逻辑
        // 为了简化，暂时返回true
        token.len() == 6 && token.chars().all(|c| c.is_ascii_digit())
    }

    pub fn get_secret(&self) -> &str {
        &self.secret
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let auth = AuthManager::new("test_secret".to_string());
        let password = "test_password";
        let hash = auth.hash_password(password).unwrap();
        assert!(auth.verify_password(password, &hash));
        assert!(!auth.verify_password("wrong_password", &hash));
    }

    #[test]
    fn test_jwt_generation() {
        let auth = AuthManager::new("test_secret".to_string());
        let user = User {
            id: "test_id".to_string(),
            username: "test_user".to_string(),
            password_hash: "hash".to_string(),
            email: None,
            role: UserRole::User,
            groups: vec!["group1".to_string()],
            enabled: true,
            created_at: SystemTime::now(),
            last_login: None,
            failed_login_attempts: 0,
            locked_until: None,
            two_factor_enabled: false,
            two_factor_secret: None,
        };

        let token = auth.generate_jwt(&user).unwrap();
        let claims = auth.verify_jwt(&token).unwrap();
        assert_eq!(claims.username, "test_user");
    }
}