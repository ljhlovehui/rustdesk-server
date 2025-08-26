// 企业级数据库模块 - 支持用户管理、设备分组、审计日志等
use crate::auth::{User, UserRole, Session, DeviceGroup, GroupPermissions};
use async_trait::async_trait;
use hbb_common::{log, ResultType};
use serde_derive::{Deserialize, Serialize};
use sqlx::{
    sqlite::SqliteConnectOptions, ConnectOptions, Connection, Error as SqlxError, SqliteConnection, Row,
};
use std::{ops::DerefMut, str::FromStr, time::SystemTime, collections::HashMap};

type Pool = deadpool::managed::Pool<DbPool>;

pub struct DbPool {
    url: String,
}

#[async_trait]
impl deadpool::managed::Manager for DbPool {
    type Type = SqliteConnection;
    type Error = SqlxError;
    
    async fn create(&self) -> Result<SqliteConnection, SqlxError> {
        let mut opt = SqliteConnectOptions::from_str(&self.url).unwrap()
            .create_if_missing(true)
            .pragma("foreign_keys", "ON");
        
        // 生产环境不记录SQL语句
        if cfg!(debug_assertions) {
            opt = opt.log_statements(log::LevelFilter::Debug);
        }
        
        SqliteConnection::connect_with(&opt).await
    }
    
    async fn recycle(
        &self,
        obj: &mut SqliteConnection,
    ) -> deadpool::managed::RecycleResult<SqlxError> {
        Ok(obj.ping().await?)
    }
}

#[derive(Clone)]
pub struct EnterpriseDatabase {
    pool: Pool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: i64,
    pub user_id: String,
    pub device_id: String,
    pub action: String,
    pub details: Option<String>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub timestamp: SystemTime,
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub os: String,
    pub version: String,
    pub ip_address: String,
    pub mac_address: Option<String>,
    pub last_online: SystemTime,
    pub owner_id: String,
    pub group_ids: Vec<String>,
    pub enabled: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionSession {
    pub id: String,
    pub controller_id: String,
    pub controlled_device_id: String,
    pub start_time: SystemTime,
    pub end_time: Option<SystemTime>,
    pub duration_seconds: Option<i64>,
    pub bytes_transferred: i64,
    pub connection_type: String, // "direct", "relay"
    pub quality_score: Option<f32>,
}

impl EnterpriseDatabase {
    pub async fn new(url: &str) -> ResultType<Self> {
        let n: usize = std::env::var("MAX_DATABASE_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_owned())
            .parse()
            .unwrap_or(10);
        
        log::info!("Enterprise Database - MAX_CONNECTIONS={}", n);
        
        let pool = Pool::new(
            DbPool {
                url: url.to_owned(),
            },
            n,
        );
        
        let _ = pool.get().await?; // 测试连接
        let db = Self { pool };
        db.create_tables().await?;
        db.create_default_admin().await?;
        
        Ok(db)
    }

    async fn create_tables(&self) -> ResultType<()> {
        let mut conn = self.pool.get().await?;
        
        // 用户表
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY NOT NULL,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                email TEXT,
                role TEXT NOT NULL DEFAULT 'User',
                groups TEXT NOT NULL DEFAULT '[]',
                enabled BOOLEAN NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                last_login INTEGER,
                failed_login_attempts INTEGER NOT NULL DEFAULT 0,
                locked_until INTEGER,
                two_factor_enabled BOOLEAN NOT NULL DEFAULT 0,
                two_factor_secret TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
            CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
            "#
        )
        .execute(conn.deref_mut())
        .await?;

        // 会话表
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL,
                token TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                last_activity INTEGER NOT NULL,
                ip_address TEXT NOT NULL,
                user_agent TEXT,
                active BOOLEAN NOT NULL DEFAULT 1,
                FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token);
            "#
        )
        .execute(conn.deref_mut())
        .await?;

        // 设备组表
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS device_groups (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                created_by TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                devices TEXT NOT NULL DEFAULT '[]',
                permissions TEXT NOT NULL,
                FOREIGN KEY (created_by) REFERENCES users (id)
            );
            CREATE INDEX IF NOT EXISTS idx_device_groups_name ON device_groups(name);
            "#
        )
        .execute(conn.deref_mut())
        .await?;

        // 设备信息表
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                os TEXT NOT NULL,
                version TEXT NOT NULL,
                ip_address TEXT NOT NULL,
                mac_address TEXT,
                last_online INTEGER NOT NULL,
                owner_id TEXT NOT NULL,
                group_ids TEXT NOT NULL DEFAULT '[]',
                enabled BOOLEAN NOT NULL DEFAULT 1,
                tags TEXT NOT NULL DEFAULT '[]',
                FOREIGN KEY (owner_id) REFERENCES users (id)
            );
            CREATE INDEX IF NOT EXISTS idx_devices_owner ON devices(owner_id);
            CREATE INDEX IF NOT EXISTS idx_devices_ip ON devices(ip_address);
            "#
        )
        .execute(conn.deref_mut())
        .await?;

        // 审计日志表
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                device_id TEXT NOT NULL,
                action TEXT NOT NULL,
                details TEXT,
                ip_address TEXT NOT NULL,
                user_agent TEXT,
                timestamp INTEGER NOT NULL,
                success BOOLEAN NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users (id)
            );
            CREATE INDEX IF NOT EXISTS idx_audit_logs_user ON audit_logs(user_id);
            CREATE INDEX IF NOT EXISTS idx_audit_logs_device ON audit_logs(device_id);
            CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp);
            "#
        )
        .execute(conn.deref_mut())
        .await?;

        // 连接会话表
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS connection_sessions (
                id TEXT PRIMARY KEY NOT NULL,
                controller_id TEXT NOT NULL,
                controlled_device_id TEXT NOT NULL,
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                duration_seconds INTEGER,
                bytes_transferred INTEGER NOT NULL DEFAULT 0,
                connection_type TEXT NOT NULL,
                quality_score REAL,
                FOREIGN KEY (controller_id) REFERENCES users (id)
            );
            CREATE INDEX IF NOT EXISTS idx_conn_sessions_controller ON connection_sessions(controller_id);
            CREATE INDEX IF NOT EXISTS idx_conn_sessions_device ON connection_sessions(controlled_device_id);
            "#
        )
        .execute(conn.deref_mut())
        .await?;

        // 原有的peer表保持兼容性
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS peer (
                guid BLOB PRIMARY KEY NOT NULL,
                id VARCHAR(100) NOT NULL,
                uuid BLOB NOT NULL,
                pk BLOB NOT NULL,
                created_at DATETIME NOT NULL DEFAULT(current_timestamp),
                user BLOB,
                status TINYINT,
                note VARCHAR(300),
                info TEXT NOT NULL
            ) WITHOUT ROWID;
            CREATE UNIQUE INDEX IF NOT EXISTS index_peer_id ON peer (id);
            "#
        )
        .execute(conn.deref_mut())
        .await?;

        Ok(())
    }

    async fn create_default_admin(&self) -> ResultType<()> {
        // 检查是否已存在管理员用户
        let existing_admin = self.get_user_by_username("admin").await?;
        if existing_admin.is_some() {
            return Ok(());
        }

        // 创建默认管理员账户
        let admin_user = User {
            id: uuid::Uuid::new_v4().to_string(),
            username: "admin".to_string(),
            password_hash: bcrypt::hash("admin123", bcrypt::DEFAULT_COST)?,
            email: Some("admin@rustdesk.local".to_string()),
            role: UserRole::SuperAdmin,
            groups: vec!["administrators".to_string()],
            enabled: true,
            created_at: SystemTime::now(),
            last_login: None,
            failed_login_attempts: 0,
            locked_until: None,
            two_factor_enabled: false,
            two_factor_secret: None,
        };

        self.create_user(&admin_user).await?;
        log::info!("Created default admin user - username: admin, password: admin123");
        log::warn!("Please change the default admin password immediately!");

        Ok(())
    }

    // 用户管理方法
    pub async fn create_user(&self, user: &User) -> ResultType<()> {
        let mut conn = self.pool.get().await?;
        let created_at = user.created_at.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64;
        let groups_json = serde_json::to_string(&user.groups)?;
        let role_str = format!("{:?}", user.role);

        sqlx::query!(
            r#"
            INSERT INTO users (
                id, username, password_hash, email, role, groups, enabled,
                created_at, failed_login_attempts, two_factor_enabled, two_factor_secret
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            user.id,
            user.username,
            user.password_hash,
            user.email,
            role_str,
            groups_json,
            user.enabled,
            created_at,
            user.failed_login_attempts,
            user.two_factor_enabled,
            user.two_factor_secret
        )
        .execute(conn.deref_mut())
        .await?;

        Ok(())
    }

    pub async fn get_user_by_username(&self, username: &str) -> ResultType<Option<User>> {
        let mut conn = self.pool.get().await?;
        
        let row = sqlx::query!(
            "SELECT * FROM users WHERE username = ?",
            username
        )
        .fetch_optional(conn.deref_mut())
        .await?;

        if let Some(row) = row {
            let role = match row.role.as_str() {
                "SuperAdmin" => UserRole::SuperAdmin,
                "Admin" => UserRole::Admin,
                "User" => UserRole::User,
                "ReadOnly" => UserRole::ReadOnly,
                _ => UserRole::User,
            };

            let groups: Vec<String> = serde_json::from_str(&row.groups).unwrap_or_default();
            let created_at = std::time::UNIX_EPOCH + std::time::Duration::from_secs(row.created_at as u64);
            let last_login = row.last_login.map(|ts| std::time::UNIX_EPOCH + std::time::Duration::from_secs(ts as u64));
            let locked_until = row.locked_until.map(|ts| std::time::UNIX_EPOCH + std::time::Duration::from_secs(ts as u64));

            Ok(Some(User {
                id: row.id,
                username: row.username,
                password_hash: row.password_hash,
                email: row.email,
                role,
                groups,
                enabled: row.enabled,
                created_at,
                last_login,
                failed_login_attempts: row.failed_login_attempts as u32,
                locked_until,
                two_factor_enabled: row.two_factor_enabled,
                two_factor_secret: row.two_factor_secret,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_user_login_info(&self, user_id: &str, success: bool) -> ResultType<()> {
        let mut conn = self.pool.get().await?;
        let now = SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64;

        if success {
            sqlx::query!(
                "UPDATE users SET last_login = ?, failed_login_attempts = 0, locked_until = NULL WHERE id = ?",
                now,
                user_id
            )
            .execute(conn.deref_mut())
            .await?;
        } else {
            sqlx::query!(
                "UPDATE users SET failed_login_attempts = failed_login_attempts + 1 WHERE id = ?",
                user_id
            )
            .execute(conn.deref_mut())
            .await?;
        }

        Ok(())
    }

    // 审计日志方法
    pub async fn log_audit(&self, log: &AuditLog) -> ResultType<()> {
        let mut conn = self.pool.get().await?;
        let timestamp = log.timestamp.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64;

        sqlx::query!(
            r#"
            INSERT INTO audit_logs (
                user_id, device_id, action, details, ip_address, 
                user_agent, timestamp, success
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            log.user_id,
            log.device_id,
            log.action,
            log.details,
            log.ip_address,
            log.user_agent,
            timestamp,
            log.success
        )
        .execute(conn.deref_mut())
        .await?;

        Ok(())
    }

    pub async fn get_audit_logs(
        &self,
        user_id: Option<&str>,
        device_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> ResultType<Vec<AuditLog>> {
        let mut conn = self.pool.get().await?;
        
        let rows = match (user_id, device_id) {
            (Some(uid), Some(did)) => {
                sqlx::query!(
                    "SELECT * FROM audit_logs WHERE user_id = ? AND device_id = ? ORDER BY timestamp DESC LIMIT ? OFFSET ?",
                    uid, did, limit, offset
                )
                .fetch_all(conn.deref_mut())
                .await?
            }
            (Some(uid), None) => {
                sqlx::query!(
                    "SELECT * FROM audit_logs WHERE user_id = ? ORDER BY timestamp DESC LIMIT ? OFFSET ?",
                    uid, limit, offset
                )
                .fetch_all(conn.deref_mut())
                .await?
            }
            (None, Some(did)) => {
                sqlx::query!(
                    "SELECT * FROM audit_logs WHERE device_id = ? ORDER BY timestamp DESC LIMIT ? OFFSET ?",
                    did, limit, offset
                )
                .fetch_all(conn.deref_mut())
                .await?
            }
            (None, None) => {
                sqlx::query!(
                    "SELECT * FROM audit_logs ORDER BY timestamp DESC LIMIT ? OFFSET ?",
                    limit, offset
                )
                .fetch_all(conn.deref_mut())
                .await?
            }
        };

        let mut logs = Vec::new();
        for row in rows {
            let timestamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(row.timestamp as u64);
            logs.push(AuditLog {
                id: row.id,
                user_id: row.user_id,
                device_id: row.device_id,
                action: row.action,
                details: row.details,
                ip_address: row.ip_address,
                user_agent: row.user_agent,
                timestamp,
                success: row.success,
            });
        }

        Ok(logs)
    }

    // 设备管理方法
    pub async fn register_device(&self, device: &DeviceInfo) -> ResultType<()> {
        let mut conn = self.pool.get().await?;
        let last_online = device.last_online.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64;
        let group_ids_json = serde_json::to_string(&device.group_ids)?;
        let tags_json = serde_json::to_string(&device.tags)?;

        sqlx::query!(
            r#"
            INSERT OR REPLACE INTO devices (
                id, name, os, version, ip_address, mac_address,
                last_online, owner_id, group_ids, enabled, tags
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            device.id,
            device.name,
            device.os,
            device.version,
            device.ip_address,
            device.mac_address,
            last_online,
            device.owner_id,
            group_ids_json,
            device.enabled,
            tags_json
        )
        .execute(conn.deref_mut())
        .await?;

        Ok(())
    }

    pub async fn get_devices_by_user(&self, user_id: &str) -> ResultType<Vec<DeviceInfo>> {
        let mut conn = self.pool.get().await?;
        
        let rows = sqlx::query!(
            "SELECT * FROM devices WHERE owner_id = ? AND enabled = 1",
            user_id
        )
        .fetch_all(conn.deref_mut())
        .await?;

        let mut devices = Vec::new();
        for row in rows {
            let last_online = std::time::UNIX_EPOCH + std::time::Duration::from_secs(row.last_online as u64);
            let group_ids: Vec<String> = serde_json::from_str(&row.group_ids).unwrap_or_default();
            let tags: Vec<String> = serde_json::from_str(&row.tags).unwrap_or_default();

            devices.push(DeviceInfo {
                id: row.id,
                name: row.name,
                os: row.os,
                version: row.version,
                ip_address: row.ip_address,
                mac_address: row.mac_address,
                last_online,
                owner_id: row.owner_id,
                group_ids,
                enabled: row.enabled,
                tags,
            });
        }

        Ok(devices)
    }
}