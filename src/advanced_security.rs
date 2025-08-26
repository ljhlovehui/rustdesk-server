// 高级安全模块 - 双因素认证、端到端加密、安全审计
use crate::auth::{User, Claims};
use crate::enterprise_database::{EnterpriseDatabase, AuditLog};
use hbb_common::{log, ResultType};
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH, Duration},
};
use tokio::sync::{RwLock, Mutex};
use sodiumoxide::crypto::{box_, secretbox, sign, hash};
use base64::{encode, decode};

// 双因素认证
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorAuth {
    pub user_id: String,
    pub secret: String,
    pub backup_codes: Vec<String>,
    pub enabled: bool,
    pub created_at: SystemTime,
    pub last_used: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpConfig {
    pub issuer: String,
    pub account_name: String,
    pub secret: String,
    pub algorithm: String, // SHA1, SHA256, SHA512
    pub digits: u32,       // 6 or 8
    pub period: u32,       // 30 seconds
}

// 端到端加密
#[derive(Debug, Clone)]
pub struct E2EEncryption {
    pub session_id: String,
    pub local_keypair: (box_::PublicKey, box_::SecretKey),
    pub remote_public_key: Option<box_::PublicKey>,
    pub shared_secret: Option<secretbox::Key>,
    pub nonce_counter: Arc<Mutex<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedMessage {
    pub session_id: String,
    pub nonce: String,
    pub ciphertext: String,
    pub signature: String,
    pub timestamp: u64,
}

// 安全审计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub id: String,
    pub event_type: SecurityEventType,
    pub severity: SecuritySeverity,
    pub user_id: Option<String>,
    pub device_id: Option<String>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub details: HashMap<String, String>,
    pub timestamp: SystemTime,
    pub resolved: bool,
    pub resolution_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    LoginAttempt,
    LoginFailure,
    LoginSuccess,
    PasswordChange,
    TwoFactorEnabled,
    TwoFactorDisabled,
    UnauthorizedAccess,
    SuspiciousActivity,
    DataExfiltration,
    MalwareDetection,
    BruteForceAttack,
    PrivilegeEscalation,
    ConfigurationChange,
    SystemCompromise,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rules: Vec<SecurityRule>,
    pub enabled: bool,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRule {
    pub id: String,
    pub rule_type: SecurityRuleType,
    pub condition: String,
    pub action: SecurityAction,
    pub threshold: Option<u32>,
    pub time_window: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityRuleType {
    FailedLoginAttempts,
    UnusualLoginLocation,
    OffHoursAccess,
    MultipleDeviceAccess,
    DataTransferVolume,
    SessionDuration,
    PrivilegedOperations,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityAction {
    Alert,
    Block,
    Quarantine,
    RequireApproval,
    ForceLogout,
    DisableAccount,
}

pub struct AdvancedSecurityManager {
    db: EnterpriseDatabase,
    totp_configs: Arc<RwLock<HashMap<String, TotpConfig>>>,
    active_sessions: Arc<RwLock<HashMap<String, E2EEncryption>>>,
    security_events: Arc<RwLock<Vec<SecurityEvent>>>,
    security_policies: Arc<RwLock<HashMap<String, SecurityPolicy>>>,
    failed_attempts: Arc<RwLock<HashMap<String, Vec<SystemTime>>>>,
    suspicious_ips: Arc<RwLock<HashMap<String, SuspiciousActivity>>>,
}

#[derive(Debug, Clone)]
struct SuspiciousActivity {
    ip_address: String,
    attempts: u32,
    first_seen: SystemTime,
    last_seen: SystemTime,
    blocked: bool,
}

impl AdvancedSecurityManager {
    pub fn new(db: EnterpriseDatabase) -> Self {
        Self {
            db,
            totp_configs: Arc::new(RwLock::new(HashMap::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            security_events: Arc::new(RwLock::new(Vec::new())),
            security_policies: Arc::new(RwLock::new(HashMap::new())),
            failed_attempts: Arc::new(RwLock::new(HashMap::new())),
            suspicious_ips: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn initialize(&self) -> ResultType<()> {
        self.load_security_policies().await?;
        self.start_security_monitoring().await?;
        Ok(())
    }

    // 双因素认证实现
    pub async fn enable_2fa(&self, user_id: &str) -> ResultType<TotpConfig> {
        // 生成密钥
        let secret = self.generate_totp_secret();
        
        // 创建TOTP配置
        let config = TotpConfig {
            issuer: "RustDesk Enterprise".to_string(),
            account_name: user_id.to_string(),
            secret: secret.clone(),
            algorithm: "SHA1".to_string(),
            digits: 6,
            period: 30,
        };

        // 生成备份码
        let backup_codes = self.generate_backup_codes();

        // 保存到数据库
        let two_fa = TwoFactorAuth {
            user_id: user_id.to_string(),
            secret: secret.clone(),
            backup_codes,
            enabled: false, // 需要验证后才启用
            created_at: SystemTime::now(),
            last_used: None,
        };

        self.db.save_2fa_config(&two_fa).await?;
        
        // 缓存配置
        self.totp_configs.write().await.insert(user_id.to_string(), config.clone());

        // 记录安全事件
        self.log_security_event(SecurityEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: SecurityEventType::TwoFactorEnabled,
            severity: SecuritySeverity::Medium,
            user_id: Some(user_id.to_string()),
            device_id: None,
            ip_address: "127.0.0.1".to_string(),
            user_agent: None,
            details: HashMap::new(),
            timestamp: SystemTime::now(),
            resolved: true,
            resolution_notes: None,
        }).await;

        Ok(config)
    }

    pub async fn verify_2fa(&self, user_id: &str, code: &str) -> ResultType<bool> {
        let configs = self.totp_configs.read().await;
        if let Some(config) = configs.get(user_id) {
            let is_valid = self.verify_totp_code(&config.secret, code)?;
            
            if is_valid {
                // 更新最后使用时间
                self.db.update_2fa_last_used(user_id, SystemTime::now()).await?;
            }
            
            Ok(is_valid)
        } else {
            Ok(false)
        }
    }

    pub async fn disable_2fa(&self, user_id: &str) -> ResultType<()> {
        // 从数据库删除
        self.db.delete_2fa_config(user_id).await?;
        
        // 从缓存删除
        self.totp_configs.write().await.remove(user_id);

        // 记录安全事件
        self.log_security_event(SecurityEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: SecurityEventType::TwoFactorDisabled,
            severity: SecuritySeverity::Medium,
            user_id: Some(user_id.to_string()),
            device_id: None,
            ip_address: "127.0.0.1".to_string(),
            user_agent: None,
            details: HashMap::new(),
            timestamp: SystemTime::now(),
            resolved: true,
            resolution_notes: None,
        }).await;

        Ok(())
    }

    fn generate_totp_secret(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
        (0..32)
            .map(|_| chars[rng.gen_range(0..chars.len())] as char)
            .collect()
    }

    fn generate_backup_codes(&self) -> Vec<String> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..10)
            .map(|_| {
                format!("{:04}-{:04}", 
                    rng.gen_range(1000..9999),
                    rng.gen_range(1000..9999)
                )
            })
            .collect()
    }

    fn verify_totp_code(&self, secret: &str, code: &str) -> ResultType<bool> {
        use totp_rs::{Algorithm, TOTP};
        
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret.as_bytes().to_vec(),
        )?;

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        // 检查当前时间窗口和前后各一个窗口（允许时钟偏差）
        for offset in [-1, 0, 1] {
            let time = current_time as i64 + (offset * 30);
            if time >= 0 {
                let expected_code = totp.generate(time as u64);
                if expected_code == code {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    pub fn generate_qr_code(&self, config: &TotpConfig) -> ResultType<String> {
        let url = format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm={}&digits={}&period={}",
            config.issuer,
            config.account_name,
            config.secret,
            config.issuer,
            config.algorithm,
            config.digits,
            config.period
        );

        // 生成QR码
        use qrcode::QrCode;
        use image::Luma;

        let code = QrCode::new(&url)?;
        let image = code.render::<Luma<u8>>().build();
        
        // 转换为base64
        let mut buffer = Vec::new();
        image.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageOutputFormat::Png)?;
        Ok(encode(&buffer))
    }

    // 端到端加密实现
    pub async fn initiate_e2e_session(&self, session_id: &str) -> ResultType<String> {
        // 生成密钥对
        let (public_key, secret_key) = box_::gen_keypair();
        
        let encryption = E2EEncryption {
            session_id: session_id.to_string(),
            local_keypair: (public_key, secret_key),
            remote_public_key: None,
            shared_secret: None,
            nonce_counter: Arc::new(Mutex::new(0)),
        };

        // 保存会话
        self.active_sessions.write().await.insert(session_id.to_string(), encryption);

        // 返回公钥
        Ok(encode(&public_key.0))
    }

    pub async fn complete_e2e_handshake(&self, session_id: &str, remote_public_key: &str) -> ResultType<()> {
        let remote_key_bytes = decode(remote_public_key)?;
        let remote_public_key = box_::PublicKey::from_slice(&remote_key_bytes)
            .ok_or("Invalid public key")?;

        let mut sessions = self.active_sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.remote_public_key = Some(remote_public_key);
            
            // 生成共享密钥
            let shared_secret = box_::precompute(&remote_public_key, &session.local_keypair.1);
            session.shared_secret = Some(secretbox::Key::from_slice(&shared_secret.0[..secretbox::KEYBYTES]).unwrap());
            
            log::info!("E2E encryption established for session: {}", session_id);
        }

        Ok(())
    }

    pub async fn encrypt_message(&self, session_id: &str, plaintext: &[u8]) -> ResultType<EncryptedMessage> {
        let sessions = self.active_sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or("Session not found")?;

        let shared_secret = session.shared_secret
            .as_ref()
            .ok_or("E2E encryption not established")?;

        // 生成nonce
        let mut nonce_counter = session.nonce_counter.lock().await;
        *nonce_counter += 1;
        let nonce = secretbox::gen_nonce();

        // 加密数据
        let ciphertext = secretbox::seal(plaintext, &nonce, shared_secret);

        // 签名
        let signature = sign::sign(&ciphertext, &session.local_keypair.1);

        Ok(EncryptedMessage {
            session_id: session_id.to_string(),
            nonce: encode(&nonce.0),
            ciphertext: encode(&ciphertext),
            signature: encode(&signature),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        })
    }

    pub async fn decrypt_message(&self, message: &EncryptedMessage) -> ResultType<Vec<u8>> {
        let sessions = self.active_sessions.read().await;
        let session = sessions.get(&message.session_id)
            .ok_or("Session not found")?;

        let shared_secret = session.shared_secret
            .as_ref()
            .ok_or("E2E encryption not established")?;

        let remote_public_key = session.remote_public_key
            .as_ref()
            .ok_or("Remote public key not available")?;

        // 解码数据
        let nonce_bytes = decode(&message.nonce)?;
        let ciphertext = decode(&message.ciphertext)?;
        let signature = decode(&message.signature)?;

        // 验证签名
        let verified_data = sign::verify(&signature, remote_public_key)
            .map_err(|_| "Signature verification failed")?;

        if verified_data != ciphertext {
            return Err("Data integrity check failed".into());
        }

        // 解密
        let nonce = secretbox::Nonce::from_slice(&nonce_bytes)
            .ok_or("Invalid nonce")?;

        let plaintext = secretbox::open(&ciphertext, &nonce, shared_secret)
            .map_err(|_| "Decryption failed")?;

        Ok(plaintext)
    }

    // 安全审计实现
    pub async fn log_security_event(&self, event: SecurityEvent) {
        // 保存到数据库
        let _ = self.db.save_security_event(&event).await;

        // 添加到内存缓存
        self.security_events.write().await.push(event.clone());

        // 检查是否触发安全策略
        self.evaluate_security_policies(&event).await;

        // 发送告警（如果需要）
        if matches!(event.severity, SecuritySeverity::High | SecuritySeverity::Critical) {
            self.send_security_alert(&event).await;
        }
    }

    pub async fn log_login_attempt(&self, user_id: &str, ip_address: &str, success: bool, details: HashMap<String, String>) {
        let event_type = if success {
            SecurityEventType::LoginSuccess
        } else {
            SecurityEventType::LoginFailure
        };

        let event = SecurityEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            severity: if success { SecuritySeverity::Low } else { SecuritySeverity::Medium },
            user_id: Some(user_id.to_string()),
            device_id: None,
            ip_address: ip_address.to_string(),
            user_agent: details.get("user_agent").cloned(),
            details,
            timestamp: SystemTime::now(),
            resolved: success,
            resolution_notes: None,
        };

        self.log_security_event(event).await;

        // 跟踪失败尝试
        if !success {
            self.track_failed_attempt(user_id, ip_address).await;
        }
    }

    async fn track_failed_attempt(&self, user_id: &str, ip_address: &str) {
        let mut attempts = self.failed_attempts.write().await;
        let key = format!("{}:{}", user_id, ip_address);
        let now = SystemTime::now();

        let user_attempts = attempts.entry(key).or_insert_with(Vec::new);
        user_attempts.push(now);

        // 保留最近1小时的尝试记录
        user_attempts.retain(|&time| {
            now.duration_since(time).unwrap_or_default().as_secs() < 3600
        });

        // 检查是否达到暴力破解阈值
        if user_attempts.len() >= 5 {
            self.handle_brute_force_attack(user_id, ip_address).await;
        }
    }

    async fn handle_brute_force_attack(&self, user_id: &str, ip_address: &str) {
        // 记录暴力破解事件
        let event = SecurityEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: SecurityEventType::BruteForceAttack,
            severity: SecuritySeverity::High,
            user_id: Some(user_id.to_string()),
            device_id: None,
            ip_address: ip_address.to_string(),
            user_agent: None,
            details: HashMap::from([
                ("attack_type".to_string(), "brute_force".to_string()),
                ("target_user".to_string(), user_id.to_string()),
            ]),
            timestamp: SystemTime::now(),
            resolved: false,
            resolution_notes: None,
        };

        self.log_security_event(event).await;

        // 标记IP为可疑
        let mut suspicious = self.suspicious_ips.write().await;
        suspicious.insert(ip_address.to_string(), SuspiciousActivity {
            ip_address: ip_address.to_string(),
            attempts: 5,
            first_seen: SystemTime::now(),
            last_seen: SystemTime::now(),
            blocked: true,
        });

        log::warn!("Brute force attack detected from {} targeting user {}", ip_address, user_id);
    }

    pub async fn is_ip_blocked(&self, ip_address: &str) -> bool {
        let suspicious = self.suspicious_ips.read().await;
        suspicious.get(ip_address)
            .map(|activity| activity.blocked)
            .unwrap_or(false)
    }

    async fn evaluate_security_policies(&self, event: &SecurityEvent) {
        let policies = self.security_policies.read().await;
        
        for policy in policies.values() {
            if !policy.enabled {
                continue;
            }

            for rule in &policy.rules {
                if self.rule_matches(rule, event) {
                    self.execute_security_action(&rule.action, event).await;
                }
            }
        }
    }

    fn rule_matches(&self, rule: &SecurityRule, event: &SecurityEvent) -> bool {
        match rule.rule_type {
            SecurityRuleType::FailedLoginAttempts => {
                matches!(event.event_type, SecurityEventType::LoginFailure)
            }
            SecurityRuleType::UnusualLoginLocation => {
                // TODO: 实现地理位置检查
                false
            }
            SecurityRuleType::OffHoursAccess => {
                // TODO: 实现工作时间检查
                false
            }
            _ => false,
        }
    }

    async fn execute_security_action(&self, action: &SecurityAction, event: &SecurityEvent) {
        match action {
            SecurityAction::Alert => {
                self.send_security_alert(event).await;
            }
            SecurityAction::Block => {
                if let Some(user_id) = &event.user_id {
                    // TODO: 实现用户阻止
                    log::warn!("Blocking user: {}", user_id);
                }
            }
            SecurityAction::ForceLogout => {
                if let Some(user_id) = &event.user_id {
                    // TODO: 实现强制登出
                    log::warn!("Force logout user: {}", user_id);
                }
            }
            _ => {}
        }
    }

    async fn send_security_alert(&self, event: &SecurityEvent) {
        // TODO: 实现邮件/短信告警
        log::warn!("Security alert: {:?}", event);
    }

    async fn load_security_policies(&self) -> ResultType<()> {
        // 加载默认安全策略
        let default_policy = SecurityPolicy {
            id: "default_security_policy".to_string(),
            name: "默认安全策略".to_string(),
            description: "基础安全规则".to_string(),
            rules: vec![
                SecurityRule {
                    id: "failed_login_rule".to_string(),
                    rule_type: SecurityRuleType::FailedLoginAttempts,
                    condition: "count >= 5".to_string(),
                    action: SecurityAction::Block,
                    threshold: Some(5),
                    time_window: Some(Duration::from_secs(3600)),
                },
            ],
            enabled: true,
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        };

        self.security_policies.write().await.insert(
            default_policy.id.clone(),
            default_policy
        );

        Ok(())
    }

    async fn start_security_monitoring(&self) -> ResultType<()> {
        // 启动后台监控任务
        let failed_attempts = self.failed_attempts.clone();
        let suspicious_ips = self.suspicious_ips.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5分钟
            loop {
                interval.tick().await;
                
                // 清理过期的失败尝试记录
                let now = SystemTime::now();
                let mut attempts = failed_attempts.write().await;
                for (_, user_attempts) in attempts.iter_mut() {
                    user_attempts.retain(|&time| {
                        now.duration_since(time).unwrap_or_default().as_secs() < 3600
                    });
                }
                attempts.retain(|_, user_attempts| !user_attempts.is_empty());

                // 清理过期的可疑IP记录
                let mut suspicious = suspicious_ips.write().await;
                suspicious.retain(|_, activity| {
                    now.duration_since(activity.last_seen).unwrap_or_default().as_secs() < 86400 // 24小时
                });
            }
        });

        Ok(())
    }

    // 数据完整性验证
    pub fn calculate_data_hash(&self, data: &[u8]) -> String {
        let hash_result = hash::hash(data);
        encode(&hash_result.0)
    }

    pub fn verify_data_integrity(&self, data: &[u8], expected_hash: &str) -> bool {
        let calculated_hash = self.calculate_data_hash(data);
        calculated_hash == expected_hash
    }
}

// 扩展数据库接口
impl EnterpriseDatabase {
    pub async fn save_2fa_config(&self, config: &TwoFactorAuth) -> ResultType<()> {
        // TODO: 实现2FA配置保存
        Ok(())
    }

    pub async fn update_2fa_last_used(&self, user_id: &str, timestamp: SystemTime) -> ResultType<()> {
        // TODO: 实现2FA最后使用时间更新
        Ok(())
    }

    pub async fn delete_2fa_config(&self, user_id: &str) -> ResultType<()> {
        // TODO: 实现2FA配置删除
        Ok(())
    }

    pub async fn save_security_event(&self, event: &SecurityEvent) -> ResultType<()> {
        // TODO: 实现安全事件保存
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_totp_generation_and_verification() {
        let security_manager = AdvancedSecurityManager::new(
            EnterpriseDatabase::new("test.db").await.unwrap()
        );

        let secret = security_manager.generate_totp_secret();
        assert_eq!(secret.len(), 32);

        // TODO: 添加更多TOTP测试
    }

    #[tokio::test]
    async fn test_e2e_encryption() {
        let security_manager = AdvancedSecurityManager::new(
            EnterpriseDatabase::new("test.db").await.unwrap()
        );

        let session_id = "test_session";
        let public_key = security_manager.initiate_e2e_session(session_id).await.unwrap();
        assert!(!public_key.is_empty());

        // TODO: 添加完整的E2E加密测试
    }
}