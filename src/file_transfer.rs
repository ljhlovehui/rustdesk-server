// 高级文件传输模块 - 支持大文件、断点续传、文件夹同步
use hbb_common::{log, ResultType};
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

const CHUNK_SIZE: usize = 1024 * 1024; // 1MB chunks
const MAX_CONCURRENT_TRANSFERS: usize = 10;
const TRANSFER_TIMEOUT: u64 = 300; // 5 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferRequest {
    pub transfer_id: String,
    pub file_path: String,
    pub file_size: u64,
    pub file_hash: String, // SHA256
    pub chunk_size: usize,
    pub resume_from: u64, // 断点续传位置
    pub transfer_type: TransferType,
    pub compression: bool,
    pub encryption: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferType {
    Upload,
    Download,
    Sync,
    FolderSync,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunk {
    pub transfer_id: String,
    pub chunk_index: u64,
    pub chunk_size: usize,
    pub data: Vec<u8>,
    pub checksum: String, // CRC32
    pub is_last: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    pub transfer_id: String,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub speed_bps: u64, // bytes per second
    pub eta_seconds: u64, // estimated time to completion
    pub status: TransferStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferStatus {
    Pending,
    InProgress,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
struct ActiveTransfer {
    request: FileTransferRequest,
    file_handle: Option<File>,
    bytes_transferred: u64,
    start_time: SystemTime,
    last_activity: SystemTime,
    chunks_received: HashMap<u64, bool>,
    speed_samples: Vec<(SystemTime, u64)>, // (time, bytes)
}

pub struct FileTransferManager {
    active_transfers: Arc<RwLock<HashMap<String, ActiveTransfer>>>,
    transfer_permissions: Arc<RwLock<HashMap<String, TransferPermissions>>>,
    temp_dir: PathBuf,
    max_file_size: u64,
    allowed_extensions: Vec<String>,
}

#[derive(Debug, Clone)]
struct TransferPermissions {
    user_id: String,
    can_upload: bool,
    can_download: bool,
    can_sync: bool,
    max_file_size: u64,
    allowed_paths: Vec<PathBuf>,
    blocked_extensions: Vec<String>,
}

impl FileTransferManager {
    pub fn new(temp_dir: PathBuf, max_file_size: u64) -> Self {
        Self {
            active_transfers: Arc::new(RwLock::new(HashMap::new())),
            transfer_permissions: Arc::new(RwLock::new(HashMap::new())),
            temp_dir,
            max_file_size,
            allowed_extensions: vec![
                "txt".to_string(), "pdf".to_string(), "doc".to_string(), 
                "docx".to_string(), "xls".to_string(), "xlsx".to_string(),
                "ppt".to_string(), "pptx".to_string(), "zip".to_string(),
                "rar".to_string(), "7z".to_string(), "jpg".to_string(),
                "jpeg".to_string(), "png".to_string(), "gif".to_string(),
                "mp4".to_string(), "avi".to_string(), "mkv".to_string(),
            ],
        }
    }

    // 设置用户传输权限
    pub async fn set_user_permissions(&self, user_id: String, permissions: TransferPermissions) {
        self.transfer_permissions.write().await.insert(user_id, permissions);
    }

    // 检查用户权限
    async fn check_permissions(&self, user_id: &str, request: &FileTransferRequest) -> ResultType<()> {
        let permissions = self.transfer_permissions.read().await;
        let user_perms = permissions.get(user_id)
            .ok_or("User has no file transfer permissions")?;

        match request.transfer_type {
            TransferType::Upload => {
                if !user_perms.can_upload {
                    return Err("User not allowed to upload files".into());
                }
            }
            TransferType::Download => {
                if !user_perms.can_download {
                    return Err("User not allowed to download files".into());
                }
            }
            TransferType::Sync | TransferType::FolderSync => {
                if !user_perms.can_sync {
                    return Err("User not allowed to sync files".into());
                }
            }
        }

        // 检查文件大小
        if request.file_size > user_perms.max_file_size {
            return Err("File size exceeds user limit".into());
        }

        // 检查文件扩展名
        let path = Path::new(&request.file_path);
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if user_perms.blocked_extensions.contains(&ext_str) {
                return Err("File extension not allowed".into());
            }
        }

        // 检查路径权限
        if !user_perms.allowed_paths.is_empty() {
            let file_path = Path::new(&request.file_path);
            let allowed = user_perms.allowed_paths.iter().any(|allowed_path| {
                file_path.starts_with(allowed_path)
            });
            if !allowed {
                return Err("File path not allowed".into());
            }
        }

        Ok(())
    }

    // 开始文件传输
    pub async fn start_transfer(&self, user_id: &str, mut request: FileTransferRequest) -> ResultType<String> {
        // 检查权限
        self.check_permissions(user_id, &request).await?;

        // 检查并发传输限制
        let active_count = self.active_transfers.read().await.len();
        if active_count >= MAX_CONCURRENT_TRANSFERS {
            return Err("Too many concurrent transfers".into());
        }

        // 生成传输ID
        if request.transfer_id.is_empty() {
            request.transfer_id = Uuid::new_v4().to_string();
        }

        // 创建临时文件路径
        let temp_file_path = self.temp_dir.join(format!("{}.tmp", request.transfer_id));

        // 打开或创建文件
        let file_handle = match request.transfer_type {
            TransferType::Upload => {
                let mut file = if request.resume_from > 0 && temp_file_path.exists() {
                    // 断点续传
                    let mut file = OpenOptions::new()
                        .write(true)
                        .read(true)
                        .open(&temp_file_path)?;
                    file.seek(SeekFrom::Start(request.resume_from))?;
                    file
                } else {
                    // 新文件
                    File::create(&temp_file_path)?
                };
                Some(file)
            }
            TransferType::Download => {
                let file = File::open(&request.file_path)?;
                Some(file)
            }
            _ => None,
        };

        // 创建活跃传输记录
        let transfer = ActiveTransfer {
            request: request.clone(),
            file_handle,
            bytes_transferred: request.resume_from,
            start_time: SystemTime::now(),
            last_activity: SystemTime::now(),
            chunks_received: HashMap::new(),
            speed_samples: Vec::new(),
        };

        self.active_transfers.write().await.insert(request.transfer_id.clone(), transfer);

        log::info!("Started file transfer: {} for user: {}", request.transfer_id, user_id);
        Ok(request.transfer_id)
    }

    // 处理文件块
    pub async fn handle_chunk(&self, chunk: FileChunk) -> ResultType<()> {
        let mut transfers = self.active_transfers.write().await;
        let transfer = transfers.get_mut(&chunk.transfer_id)
            .ok_or("Transfer not found")?;

        // 验证块校验和
        let calculated_checksum = self.calculate_crc32(&chunk.data);
        if calculated_checksum != chunk.checksum {
            return Err("Chunk checksum mismatch".into());
        }

        // 写入数据
        if let Some(ref mut file) = transfer.file_handle {
            let seek_pos = chunk.chunk_index * CHUNK_SIZE as u64;
            file.seek(SeekFrom::Start(seek_pos))?;
            file.write_all(&chunk.data)?;
            file.flush()?;
        }

        // 更新进度
        transfer.chunks_received.insert(chunk.chunk_index, true);
        transfer.bytes_transferred += chunk.data.len() as u64;
        transfer.last_activity = SystemTime::now();

        // 更新速度统计
        let now = SystemTime::now();
        transfer.speed_samples.push((now, transfer.bytes_transferred));
        
        // 保持最近10个样本
        if transfer.speed_samples.len() > 10 {
            transfer.speed_samples.remove(0);
        }

        // 检查是否完成
        if chunk.is_last {
            self.complete_transfer(&chunk.transfer_id).await?;
        }

        Ok(())
    }

    // 完成传输
    async fn complete_transfer(&self, transfer_id: &str) -> ResultType<()> {
        let mut transfers = self.active_transfers.write().await;
        if let Some(mut transfer) = transfers.remove(transfer_id) {
            // 验证文件完整性
            if let Some(mut file) = transfer.file_handle.take() {
                file.flush()?;
                drop(file);

                // 验证文件哈希
                let temp_file_path = self.temp_dir.join(format!("{}.tmp", transfer_id));
                let file_hash = self.calculate_file_hash(&temp_file_path)?;
                
                if file_hash == transfer.request.file_hash {
                    // 移动到最终位置
                    let final_path = Path::new(&transfer.request.file_path);
                    if let Some(parent) = final_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::rename(&temp_file_path, final_path)?;
                    log::info!("Transfer completed successfully: {}", transfer_id);
                } else {
                    std::fs::remove_file(&temp_file_path)?;
                    return Err("File hash verification failed".into());
                }
            }
        }
        Ok(())
    }

    // 获取传输进度
    pub async fn get_progress(&self, transfer_id: &str) -> Option<TransferProgress> {
        let transfers = self.active_transfers.read().await;
        if let Some(transfer) = transfers.get(transfer_id) {
            let speed = self.calculate_speed(transfer);
            let eta = if speed > 0 {
                (transfer.request.file_size - transfer.bytes_transferred) / speed
            } else {
                0
            };

            Some(TransferProgress {
                transfer_id: transfer_id.to_string(),
                bytes_transferred: transfer.bytes_transferred,
                total_bytes: transfer.request.file_size,
                speed_bps: speed,
                eta_seconds: eta,
                status: TransferStatus::InProgress,
                error_message: None,
            })
        } else {
            None
        }
    }

    // 暂停传输
    pub async fn pause_transfer(&self, transfer_id: &str) -> ResultType<()> {
        let transfers = self.active_transfers.read().await;
        if transfers.contains_key(transfer_id) {
            log::info!("Transfer paused: {}", transfer_id);
            Ok(())
        } else {
            Err("Transfer not found".into())
        }
    }

    // 取消传输
    pub async fn cancel_transfer(&self, transfer_id: &str) -> ResultType<()> {
        let mut transfers = self.active_transfers.write().await;
        if let Some(transfer) = transfers.remove(transfer_id) {
            // 清理临时文件
            let temp_file_path = self.temp_dir.join(format!("{}.tmp", transfer_id));
            if temp_file_path.exists() {
                std::fs::remove_file(&temp_file_path)?;
            }
            log::info!("Transfer cancelled: {}", transfer_id);
            Ok(())
        } else {
            Err("Transfer not found".into())
        }
    }

    // 文件夹同步
    pub async fn sync_folder(&self, user_id: &str, source_path: &str, target_path: &str) -> ResultType<Vec<String>> {
        // 检查权限
        let permissions = self.transfer_permissions.read().await;
        let user_perms = permissions.get(user_id)
            .ok_or("User has no sync permissions")?;

        if !user_perms.can_sync {
            return Err("User not allowed to sync folders".into());
        }

        let mut transfer_ids = Vec::new();
        let source = Path::new(source_path);
        let target = Path::new(target_path);

        // 递归遍历源文件夹
        self.sync_directory_recursive(user_id, source, target, &mut transfer_ids).await?;

        Ok(transfer_ids)
    }

    async fn sync_directory_recursive(
        &self,
        user_id: &str,
        source: &Path,
        target: &Path,
        transfer_ids: &mut Vec<String>,
    ) -> ResultType<()> {
        if !source.exists() {
            return Err("Source directory does not exist".into());
        }

        // 创建目标目录
        if !target.exists() {
            std::fs::create_dir_all(target)?;
        }

        // 遍历源目录
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            let source_path = entry.path();
            let file_name = entry.file_name();
            let target_path = target.join(&file_name);

            if source_path.is_dir() {
                // 递归处理子目录
                self.sync_directory_recursive(user_id, &source_path, &target_path, transfer_ids).await?;
            } else {
                // 检查文件是否需要同步
                if self.should_sync_file(&source_path, &target_path)? {
                    let file_size = source_path.metadata()?.len();
                    let file_hash = self.calculate_file_hash(&source_path)?;

                    let request = FileTransferRequest {
                        transfer_id: Uuid::new_v4().to_string(),
                        file_path: target_path.to_string_lossy().to_string(),
                        file_size,
                        file_hash,
                        chunk_size: CHUNK_SIZE,
                        resume_from: 0,
                        transfer_type: TransferType::Sync,
                        compression: true,
                        encryption: false,
                    };

                    let transfer_id = self.start_transfer(user_id, request).await?;
                    transfer_ids.push(transfer_id);
                }
            }
        }

        Ok(())
    }

    fn should_sync_file(&self, source: &Path, target: &Path) -> ResultType<bool> {
        if !target.exists() {
            return Ok(true);
        }

        let source_meta = source.metadata()?;
        let target_meta = target.metadata()?;

        // 比较文件大小和修改时间
        Ok(source_meta.len() != target_meta.len() || 
           source_meta.modified()? > target_meta.modified()?)
    }

    // 计算传输速度
    fn calculate_speed(&self, transfer: &ActiveTransfer) -> u64 {
        if transfer.speed_samples.len() < 2 {
            return 0;
        }

        let first = &transfer.speed_samples[0];
        let last = &transfer.speed_samples[transfer.speed_samples.len() - 1];

        let time_diff = last.0.duration_since(first.0).unwrap_or_default().as_secs();
        let bytes_diff = last.1 - first.1;

        if time_diff > 0 {
            bytes_diff / time_diff
        } else {
            0
        }
    }

    // 计算CRC32校验和
    fn calculate_crc32(&self, data: &[u8]) -> String {
        use crc32fast::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(data);
        format!("{:08x}", hasher.finalize())
    }

    // 计算文件SHA256哈希
    fn calculate_file_hash(&self, path: &Path) -> ResultType<String> {
        use sha2::{Sha256, Digest};
        
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    // 清理过期传输
    pub async fn cleanup_expired_transfers(&self) {
        let mut transfers = self.active_transfers.write().await;
        let now = SystemTime::now();
        let mut to_remove = Vec::new();

        for (id, transfer) in transfers.iter() {
            if let Ok(duration) = now.duration_since(transfer.last_activity) {
                if duration.as_secs() > TRANSFER_TIMEOUT {
                    to_remove.push(id.clone());
                }
            }
        }

        for id in to_remove {
            if let Some(transfer) = transfers.remove(&id) {
                // 清理临时文件
                let temp_file_path = self.temp_dir.join(format!("{}.tmp", id));
                if temp_file_path.exists() {
                    let _ = std::fs::remove_file(&temp_file_path);
                }
                log::info!("Cleaned up expired transfer: {}", id);
            }
        }
    }
}

// 文件压缩支持
pub struct FileCompressor;

impl FileCompressor {
    pub fn compress_data(data: &[u8]) -> ResultType<Vec<u8>> {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        Ok(encoder.finish()?)
    }

    pub fn decompress_data(compressed: &[u8]) -> ResultType<Vec<u8>> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(compressed);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_transfer_manager() {
        let temp_dir = TempDir::new().unwrap();
        let manager = FileTransferManager::new(temp_dir.path().to_path_buf(), 1024 * 1024 * 100);

        // 设置用户权限
        let permissions = TransferPermissions {
            user_id: "test_user".to_string(),
            can_upload: true,
            can_download: true,
            can_sync: true,
            max_file_size: 1024 * 1024 * 10,
            allowed_paths: vec![temp_dir.path().to_path_buf()],
            blocked_extensions: vec!["exe".to_string()],
        };

        manager.set_user_permissions("test_user".to_string(), permissions).await;

        // 测试开始传输
        let request = FileTransferRequest {
            transfer_id: "".to_string(),
            file_path: temp_dir.path().join("test.txt").to_string_lossy().to_string(),
            file_size: 1024,
            file_hash: "test_hash".to_string(),
            chunk_size: CHUNK_SIZE,
            resume_from: 0,
            transfer_type: TransferType::Upload,
            compression: false,
            encryption: false,
        };

        let transfer_id = manager.start_transfer("test_user", request).await.unwrap();
        assert!(!transfer_id.is_empty());

        // 测试获取进度
        let progress = manager.get_progress(&transfer_id).await;
        assert!(progress.is_some());
    }
}