// 性能优化模块 - 编解码器、低延迟模式、带宽优化
use hbb_common::{log, ResultType};
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, atomic::{AtomicU64, AtomicBool, Ordering}},
    time::{Duration, Instant, SystemTime},
};
use tokio::sync::{RwLock, Mutex};

// 编解码器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodecConfig {
    pub codec_type: CodecType,
    pub quality: u8,           // 1-100
    pub bitrate: u32,          // kbps
    pub framerate: u8,         // fps
    pub resolution: Resolution,
    pub hardware_acceleration: bool,
    pub low_latency_mode: bool,
    pub adaptive_quality: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodecType {
    H264,
    H265,
    VP8,
    VP9,
    AV1,
    MJPEG,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

// 性能监控
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub latency_ms: f64,
    pub throughput_mbps: f64,
    pub packet_loss_rate: f64,
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub gpu_usage: f64,
    pub frame_drops: u32,
    pub encoding_time_ms: f64,
    pub decoding_time_ms: f64,
    pub network_jitter_ms: f64,
}

// 自适应质量控制
#[derive(Debug, Clone)]
pub struct AdaptiveQualityController {
    target_latency_ms: f64,
    target_framerate: u8,
    min_quality: u8,
    max_quality: u8,
    current_quality: u8,
    quality_history: VecDeque<u8>,
    latency_history: VecDeque<f64>,
    adjustment_cooldown: Instant,
}

// 带宽管理
#[derive(Debug, Clone)]
pub struct BandwidthManager {
    available_bandwidth: Arc<AtomicU64>,
    used_bandwidth: Arc<AtomicU64>,
    bandwidth_history: Arc<Mutex<VecDeque<(Instant, u64)>>>,
    congestion_control: CongestionControl,
}

#[derive(Debug, Clone)]
pub enum CongestionControl {
    BBR,
    Cubic,
    Reno,
    Vegas,
}

pub struct PerformanceOptimizer {
    codec_configs: Arc<RwLock<HashMap<String, CodecConfig>>>,
    performance_metrics: Arc<RwLock<HashMap<String, PerformanceMetrics>>>,
    adaptive_controllers: Arc<RwLock<HashMap<String, AdaptiveQualityController>>>,
    bandwidth_manager: BandwidthManager,
    low_latency_enabled: Arc<AtomicBool>,
    hardware_acceleration_available: bool,
}

impl PerformanceOptimizer {
    pub fn new() -> Self {
        Self {
            codec_configs: Arc::new(RwLock::new(HashMap::new())),
            performance_metrics: Arc::new(RwLock::new(HashMap::new())),
            adaptive_controllers: Arc::new(RwLock::new(HashMap::new())),
            bandwidth_manager: BandwidthManager::new(),
            low_latency_enabled: Arc::new(AtomicBool::new(false)),
            hardware_acceleration_available: Self::detect_hardware_acceleration(),
        }
    }

    pub async fn initialize(&self) -> ResultType<()> {
        self.load_default_codec_configs().await;
        self.start_performance_monitoring().await;
        self.start_adaptive_quality_control().await;
        Ok(())
    }

    // 编解码器优化
    pub async fn get_optimal_codec_config(&self, session_id: &str, network_conditions: &NetworkConditions) -> CodecConfig {
        let base_config = self.get_base_codec_config(network_conditions).await;
        
        // 根据网络条件调整
        let mut optimized_config = base_config.clone();
        
        if network_conditions.latency_ms > 100.0 {
            // 高延迟网络，优先降低延迟
            optimized_config.low_latency_mode = true;
            optimized_config.framerate = (optimized_config.framerate * 2 / 3).max(15);
        }
        
        if network_conditions.bandwidth_kbps < 1000 {
            // 低带宽网络，降低质量和分辨率
            optimized_config.quality = (optimized_config.quality * 2 / 3).max(30);
            optimized_config.bitrate = network_conditions.bandwidth_kbps * 8 / 10; // 使用80%带宽
            
            if network_conditions.bandwidth_kbps < 500 {
                optimized_config.resolution.width = 1280;
                optimized_config.resolution.height = 720;
            }
        }
        
        if network_conditions.packet_loss_rate > 0.05 {
            // 高丢包率，使用更鲁棒的编码
            optimized_config.codec_type = CodecType::H264; // 更好的错误恢复
        }

        // 保存配置
        self.codec_configs.write().await.insert(session_id.to_string(), optimized_config.clone());
        
        optimized_config
    }

    async fn get_base_codec_config(&self, network_conditions: &NetworkConditions) -> CodecConfig {
        if self.hardware_acceleration_available && network_conditions.bandwidth_kbps > 2000 {
            // 高性能配置
            CodecConfig {
                codec_type: CodecType::H265,
                quality: 80,
                bitrate: 4000,
                framerate: 60,
                resolution: Resolution { width: 1920, height: 1080 },
                hardware_acceleration: true,
                low_latency_mode: false,
                adaptive_quality: true,
            }
        } else if network_conditions.bandwidth_kbps > 1000 {
            // 中等配置
            CodecConfig {
                codec_type: CodecType::H264,
                quality: 70,
                bitrate: 2000,
                framerate: 30,
                resolution: Resolution { width: 1920, height: 1080 },
                hardware_acceleration: self.hardware_acceleration_available,
                low_latency_mode: false,
                adaptive_quality: true,
            }
        } else {
            // 低带宽配置
            CodecConfig {
                codec_type: CodecType::H264,
                quality: 50,
                bitrate: 800,
                framerate: 24,
                resolution: Resolution { width: 1280, height: 720 },
                hardware_acceleration: false,
                low_latency_mode: true,
                adaptive_quality: true,
            }
        }
    }

    // 低延迟模式
    pub async fn enable_low_latency_mode(&self, session_id: &str) -> ResultType<()> {
        self.low_latency_enabled.store(true, Ordering::Relaxed);
        
        let mut configs = self.codec_configs.write().await;
        if let Some(config) = configs.get_mut(session_id) {
            config.low_latency_mode = true;
            config.framerate = config.framerate.min(30); // 限制帧率减少延迟
            config.quality = (config.quality * 9 / 10).max(40); // 略微降低质量
        }
        
        log::info!("Low latency mode enabled for session: {}", session_id);
        Ok(())
    }

    pub async fn disable_low_latency_mode(&self, session_id: &str) -> ResultType<()> {
        self.low_latency_enabled.store(false, Ordering::Relaxed);
        
        let mut configs = self.codec_configs.write().await;
        if let Some(config) = configs.get_mut(session_id) {
            config.low_latency_mode = false;
        }
        
        log::info!("Low latency mode disabled for session: {}", session_id);
        Ok(())
    }

    // 自适应质量控制
    pub async fn update_adaptive_quality(&self, session_id: &str, metrics: &PerformanceMetrics) -> ResultType<()> {
        let mut controllers = self.adaptive_controllers.write().await;
        let controller = controllers.entry(session_id.to_string())
            .or_insert_with(|| AdaptiveQualityController::new());

        // 更新历史数据
        controller.latency_history.push_back(metrics.latency_ms);
        if controller.latency_history.len() > 10 {
            controller.latency_history.pop_front();
        }

        // 检查是否需要调整质量
        if controller.adjustment_cooldown.elapsed() > Duration::from_secs(2) {
            let avg_latency = controller.latency_history.iter().sum::<f64>() / controller.latency_history.len() as f64;
            
            let mut configs = self.codec_configs.write().await;
            if let Some(config) = configs.get_mut(session_id) {
                if avg_latency > controller.target_latency_ms * 1.5 {
                    // 延迟过高，降低质量
                    if controller.current_quality > controller.min_quality {
                        controller.current_quality = (controller.current_quality - 10).max(controller.min_quality);
                        config.quality = controller.current_quality;
                        config.bitrate = (config.bitrate * 9 / 10).max(500);
                        controller.adjustment_cooldown = Instant::now();
                        log::info!("Reduced quality to {} for session {}", controller.current_quality, session_id);
                    }
                } else if avg_latency < controller.target_latency_ms * 0.7 && metrics.packet_loss_rate < 0.01 {
                    // 延迟较低且无丢包，可以提高质量
                    if controller.current_quality < controller.max_quality {
                        controller.current_quality = (controller.current_quality + 5).min(controller.max_quality);
                        config.quality = controller.current_quality;
                        config.bitrate = (config.bitrate * 11 / 10).min(8000);
                        controller.adjustment_cooldown = Instant::now();
                        log::info!("Increased quality to {} for session {}", controller.current_quality, session_id);
                    }
                }
            }
        }

        Ok(())
    }

    // 带宽管理
    pub async fn estimate_bandwidth(&self) -> u64 {
        self.bandwidth_manager.estimate_available_bandwidth().await
    }

    pub async fn allocate_bandwidth(&self, session_id: &str, requested_kbps: u32) -> u32 {
        self.bandwidth_manager.allocate_bandwidth(session_id, requested_kbps).await
    }

    // 性能监控
    pub async fn collect_performance_metrics(&self, session_id: &str) -> PerformanceMetrics {
        // 模拟性能指标收集
        let metrics = PerformanceMetrics {
            latency_ms: self.measure_latency().await,
            throughput_mbps: self.measure_throughput().await,
            packet_loss_rate: self.measure_packet_loss().await,
            cpu_usage: self.get_cpu_usage(),
            memory_usage: self.get_memory_usage(),
            gpu_usage: self.get_gpu_usage(),
            frame_drops: self.count_frame_drops(session_id).await,
            encoding_time_ms: self.measure_encoding_time(session_id).await,
            decoding_time_ms: self.measure_decoding_time(session_id).await,
            network_jitter_ms: self.measure_network_jitter().await,
        };

        // 保存指标
        self.performance_metrics.write().await.insert(session_id.to_string(), metrics.clone());

        metrics
    }

    async fn measure_latency(&self) -> f64 {
        // 实际实现中应该测量真实的网络延迟
        50.0 + (rand::random::<f64>() * 20.0)
    }

    async fn measure_throughput(&self) -> f64 {
        // 实际实现中应该测量真实的吞吐量
        10.0 + (rand::random::<f64>() * 5.0)
    }

    async fn measure_packet_loss(&self) -> f64 {
        // 实际实现中应该测量真实的丢包率
        rand::random::<f64>() * 0.02
    }

    fn get_cpu_usage(&self) -> f64 {
        // 实际实现中应该获取真实的CPU使用率
        30.0 + (rand::random::<f64>() * 40.0)
    }

    fn get_memory_usage(&self) -> u64 {
        // 实际实现中应该获取真实的内存使用量
        1024 * 1024 * 512 // 512MB
    }

    fn get_gpu_usage(&self) -> f64 {
        // 实际实现中应该获取真实的GPU使用率
        if self.hardware_acceleration_available {
            20.0 + (rand::random::<f64>() * 30.0)
        } else {
            0.0
        }
    }

    async fn count_frame_drops(&self, _session_id: &str) -> u32 {
        // 实际实现中应该统计真实的丢帧数
        (rand::random::<f64>() * 5.0) as u32
    }

    async fn measure_encoding_time(&self, _session_id: &str) -> f64 {
        // 实际实现中应该测量真实的编码时间
        5.0 + (rand::random::<f64>() * 10.0)
    }

    async fn measure_decoding_time(&self, _session_id: &str) -> f64 {
        // 实际实现中应该测量真实的解码时间
        3.0 + (rand::random::<f64>() * 7.0)
    }

    async fn measure_network_jitter(&self) -> f64 {
        // 实际实现中应该测量真实的网络抖动
        1.0 + (rand::random::<f64>() * 5.0)
    }

    // 硬件加速检测
    fn detect_hardware_acceleration() -> bool {
        // 实际实现中应该检测硬件编解码器支持
        // 这里简化为随机返回
        true
    }

    async fn load_default_codec_configs(&self) {
        // 加载默认编解码器配置
        let default_config = CodecConfig {
            codec_type: CodecType::H264,
            quality: 70,
            bitrate: 2000,
            framerate: 30,
            resolution: Resolution { width: 1920, height: 1080 },
            hardware_acceleration: self.hardware_acceleration_available,
            low_latency_mode: false,
            adaptive_quality: true,
        };

        self.codec_configs.write().await.insert("default".to_string(), default_config);
    }

    async fn start_performance_monitoring(&self) {
        let metrics = self.performance_metrics.clone();
        let controllers = self.adaptive_controllers.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                
                // 定期收集性能指标
                let sessions: Vec<String> = metrics.read().await.keys().cloned().collect();
                for session_id in sessions {
                    // 这里应该实际收集指标
                    // let metrics = collect_performance_metrics(&session_id).await;
                }
            }
        });
    }

    async fn start_adaptive_quality_control(&self) {
        let controllers = self.adaptive_controllers.clone();
        let configs = self.codec_configs.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(2));
            loop {
                interval.tick().await;
                
                // 定期调整质量
                // 实际实现中应该根据性能指标调整
            }
        });
    }
}

#[derive(Debug, Clone)]
pub struct NetworkConditions {
    pub latency_ms: f64,
    pub bandwidth_kbps: u32,
    pub packet_loss_rate: f64,
    pub jitter_ms: f64,
}

impl AdaptiveQualityController {
    fn new() -> Self {
        Self {
            target_latency_ms: 50.0,
            target_framerate: 30,
            min_quality: 30,
            max_quality: 90,
            current_quality: 70,
            quality_history: VecDeque::new(),
            latency_history: VecDeque::new(),
            adjustment_cooldown: Instant::now(),
        }
    }
}

impl BandwidthManager {
    fn new() -> Self {
        Self {
            available_bandwidth: Arc::new(AtomicU64::new(10000)), // 10Mbps default
            used_bandwidth: Arc::new(AtomicU64::new(0)),
            bandwidth_history: Arc::new(Mutex::new(VecDeque::new())),
            congestion_control: CongestionControl::BBR,
        }
    }

    async fn estimate_available_bandwidth(&self) -> u64 {
        // 实际实现中应该动态测量带宽
        self.available_bandwidth.load(Ordering::Relaxed)
    }

    async fn allocate_bandwidth(&self, _session_id: &str, requested_kbps: u32) -> u32 {
        let available = self.available_bandwidth.load(Ordering::Relaxed);
        let used = self.used_bandwidth.load(Ordering::Relaxed);
        let remaining = available.saturating_sub(used);
        
        let allocated = (requested_kbps as u64).min(remaining) as u32;
        self.used_bandwidth.fetch_add(allocated as u64, Ordering::Relaxed);
        
        allocated
    }
}