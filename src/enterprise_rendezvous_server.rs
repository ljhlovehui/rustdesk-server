// 企业级会合服务器 - 集成用户认证和权限控制
use crate::auth::{AuthManager, Claims};
use crate::enterprise_database::{EnterpriseDatabase, AuditLog, DeviceInfo};
use crate::peer::*;
use crate::web_api::{create_router, AppState};
use hbb_common::{
    allow_err, bail,
    bytes::{Bytes, BytesMut},
    bytes_codec::BytesCodec,
    config,
    futures::future::join_all,
    futures_util::{
        sink::SinkExt,
        stream::{SplitSink, StreamExt},
    },
    log,
    protobuf::{Message as _, MessageField},
    rendezvous_proto::{
        register_pk_response::Result::{TOO_FREQUENT, UUID_MISMATCH},
        *,
    },
    tcp::{listen_any, FramedStream},
    timeout,
    tokio::{
        self,
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        sync::{mpsc, Mutex},
        time::{interval, Duration},
    },
    tokio_util::codec::Framed,
    try_into_v4,
    udp::FramedSocket,
    AddrMangle, ResultType,
};
use ipnetwork::Ipv4Network;
use sodiumoxide::crypto::sign;
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    sync::Arc,
    time::{Instant, SystemTime},
};

#[derive(Clone, Debug)]
enum Data {
    Msg(Box<RendezvousMessage>, SocketAddr),
    RelayServers0(String),
    RelayServers(RelayServers),
}

const REG_TIMEOUT: i32 = 30_000;
type TcpStreamSink = SplitSink<Framed<TcpStream, BytesCodec>, Bytes>;
type WsSink = SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, tungstenite::Message>;

enum Sink {
    TcpStream(TcpStreamSink),
    Ws(WsSink),
}

type Sender = mpsc::UnboundedSender<Data>;
type Receiver = mpsc::UnboundedReceiver<Data>;
static ROTATION_RELAY_SERVER: AtomicUsize = AtomicUsize::new(0);
type RelayServers = Vec<String>;
const CHECK_RELAY_TIMEOUT: u64 = 3_000;
static ALWAYS_USE_RELAY: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
struct Inner {
    serial: i32,
    version: String,
    software_url: String,
    mask: Option<Ipv4Network>,
    local_ip: String,
    sk: Option<sign::SecretKey>,
}

#[derive(Clone)]
pub struct EnterpriseRendezvousServer {
    tcp_punch: Arc<Mutex<HashMap<SocketAddr, Sink>>>,
    pm: PeerMap,
    tx: Sender,
    relay_servers: Arc<RelayServers>,
    relay_servers0: Arc<RelayServers>,
    rendezvous_servers: Arc<Vec<String>>,
    inner: Arc<Inner>,
    // 企业级功能
    enterprise_db: EnterpriseDatabase,
    auth_manager: Arc<AuthManager>,
    device_sessions: Arc<Mutex<HashMap<String, DeviceSession>>>,
}

#[derive(Clone, Debug)]
struct DeviceSession {
    device_id: String,
    user_id: Option<String>,
    authenticated: bool,
    last_activity: Instant,
    permissions: DevicePermissions,
    connection_count: u32,
}

#[derive(Clone, Debug)]
struct DevicePermissions {
    can_control: bool,
    can_transfer_files: bool,
    can_view_screen: bool,
    can_use_audio: bool,
    can_use_clipboard: bool,
    session_timeout: Option<Duration>,
}

impl Default for DevicePermissions {
    fn default() -> Self {
        Self {
            can_control: false,
            can_transfer_files: false,
            can_view_screen: false,
            can_use_audio: false,
            can_use_clipboard: false,
            session_timeout: Some(Duration::from_hours(1)),
        }
    }
}

enum LoopFailure {
    UdpSocket,
    Listener3,
    Listener2,
    Listener,
}

impl EnterpriseRendezvousServer {
    #[tokio::main(flavor = "multi_thread")]
    pub async fn start(port: i32, serial: i32, key: &str, rmem: usize) -> ResultType<()> {
        let (key, sk) = Self::get_server_sk(key);
        let nat_port = port - 1;
        let ws_port = port + 2;
        let web_port = port + 3; // Web管理界面端口
        
        // 初始化企业级数据库
        let db_url = std::env::var("ENTERPRISE_DB_URL").unwrap_or_else(|_| "enterprise.sqlite3".to_string());
        let enterprise_db = EnterpriseDatabase::new(&db_url).await?;
        
        // 初始化认证管理器
        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-super-secret-jwt-key".to_string());
        let auth_manager = Arc::new(AuthManager::new(jwt_secret));
        
        let pm = PeerMap::new().await?;
        log::info!("Enterprise Rendezvous Server starting...");
        log::info!("Serial: {}", serial);
        
        let rendezvous_servers = get_servers(&get_arg("rendezvous-servers"), "rendezvous-servers");
        log::info!("Listening on tcp/udp :{}", port);
        log::info!("Listening on tcp :{}, extra port for NAT test", nat_port);
        log::info!("Listening on websocket :{}", ws_port);
        log::info!("Web management interface on :{}", web_port);
        
        let mut socket = create_udp_listener(port, rmem).await?;
        let (tx, mut rx) = mpsc::unbounded_channel::<Data>();
        
        let software_url = get_arg("software-url");
        let version = hbb_common::get_version_from_url(&software_url);
        if !version.is_empty() {
            log::info!("software_url: {}, version: {}", software_url, version);
        }
        
        let mask = get_arg("mask").parse().ok();
        let local_ip = if mask.is_none() {
            "".to_owned()
        } else {
            get_arg_or(
                "local-ip",
                local_ip_address::local_ip()
                    .map(|x| x.to_string())
                    .unwrap_or_default(),
            )
        };
        
        let mut rs = Self {
            tcp_punch: Arc::new(Mutex::new(HashMap::new())),
            pm,
            tx: tx.clone(),
            relay_servers: Default::default(),
            relay_servers0: Default::default(),
            rendezvous_servers: Arc::new(rendezvous_servers),
            inner: Arc::new(Inner {
                serial,
                version,
                software_url,
                sk,
                mask,
                local_ip,
            }),
            enterprise_db: enterprise_db.clone(),
            auth_manager: auth_manager.clone(),
            device_sessions: Arc::new(Mutex::new(HashMap::new())),
        };
        
        log::info!("mask: {:?}", rs.inner.mask);
        log::info!("local-ip: {:?}", rs.inner.local_ip);
        
        std::env::set_var("PORT_FOR_API", port.to_string());
        rs.parse_relay_servers(&get_arg("relay-servers"));
        
        let mut listener = create_tcp_listener(port).await?;
        let mut listener2 = create_tcp_listener(nat_port).await?;
        let mut listener3 = create_tcp_listener(ws_port).await?;
        
        // 启动Web管理界面
        let web_state = AppState {
            db: enterprise_db,
            auth: auth_manager,
        };
        let web_app = create_router(web_state);
        
        tokio::spawn(async move {
            let web_listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", web_port))
                .await
                .expect("Failed to bind web server");
            log::info!("Web management interface started on port {}", web_port);
            axum::serve(web_listener, web_app)
                .await
                .expect("Web server failed");
        });
        
        // 启动设备会话清理任务
        let device_sessions_clone = rs.device_sessions.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                Self::cleanup_expired_sessions(device_sessions_clone.clone()).await;
            }
        });
        
        if std::env::var("ALWAYS_USE_RELAY")
            .unwrap_or_default()
            .to_uppercase()
            == "Y"
        {
            ALWAYS_USE_RELAY.store(true, Ordering::SeqCst);
        }
        
        log::info!(
            "ALWAYS_USE_RELAY={}",
            if ALWAYS_USE_RELAY.load(Ordering::SeqCst) {
                "Y"
            } else {
                "N"
            }
        );
        
        let main_task = async move {
            loop {
                log::info!("Enterprise Server Start");
                match rs
                    .io_loop(
                        &mut rx,
                        &mut listener,
                        &mut listener2,
                        &mut listener3,
                        &mut socket,
                        &key,
                    )
                    .await
                {
                    LoopFailure::UdpSocket => {
                        drop(socket);
                        socket = create_udp_listener(port, rmem).await?;
                    }
                    LoopFailure::Listener => {
                        drop(listener);
                        listener = create_tcp_listener(port).await?;
                    }
                    LoopFailure::Listener2 => {
                        drop(listener2);
                        listener2 = create_tcp_listener(nat_port).await?;
                    }
                    LoopFailure::Listener3 => {
                        drop(listener3);
                        listener3 = create_tcp_listener(ws_port).await?;
                    }
                }
            }
        };
        
        let listen_signal = crate::common::listen_signal();
        tokio::select!(
            res = main_task => res,
            res = listen_signal => res,
        )
    }

    async fn io_loop(
        &mut self,
        rx: &mut Receiver,
        listener: &mut TcpListener,
        listener2: &mut TcpListener,
        listener3: &mut TcpListener,
        socket: &mut FramedSocket,
        key: &str,
    ) -> LoopFailure {
        let mut timer_check_relay = interval(Duration::from_millis(CHECK_RELAY_TIMEOUT));
        loop {
            tokio::select! {
                _ = timer_check_relay.tick() => {
                    if self.relay_servers0.len() > 1 {
                        let rs = self.relay_servers0.clone();
                        let tx = self.tx.clone();
                        tokio::spawn(async move {
                            check_relay_servers(rs, tx).await;
                        });
                    }
                }
                Some(data) = rx.recv() => {
                    match data {
                        Data::Msg(msg, addr) => { allow_err!(socket.send(msg.as_ref(), addr).await); }
                        Data::RelayServers0(rs) => { self.parse_relay_servers(&rs); }
                        Data::RelayServers(rs) => { self.relay_servers = Arc::new(rs); }
                    }
                }
                res = socket.next() => {
                    match res {
                        Some(Ok((bytes, addr))) => {
                            if let Err(err) = self.handle_udp(&bytes, addr.into(), socket, key).await {
                                log::error!("udp failure: {}", err);
                                return LoopFailure::UdpSocket;
                            }
                        }
                        Some(Err(err)) => {
                            log::error!("udp failure: {}", err);
                            return LoopFailure::UdpSocket;
                        }
                        None => {}
                    }
                }
                res = listener2.accept() => {
                    match res {
                        Ok((stream, addr))  => {
                            stream.set_nodelay(true).ok();
                            self.handle_listener2(stream, addr).await;
                        }
                        Err(err) => {
                           log::error!("listener2.accept failed: {}", err);
                           return LoopFailure::Listener2;
                        }
                    }
                }
                res = listener3.accept() => {
                    match res {
                        Ok((stream, addr))  => {
                            stream.set_nodelay(true).ok();
                            self.handle_listener(stream, addr, key, true).await;
                        }
                        Err(err) => {
                           log::error!("listener3.accept failed: {}", err);
                           return LoopFailure::Listener3;
                        }
                    }
                }
                res = listener.accept() => {
                    match res {
                        Ok((stream, addr)) => {
                            stream.set_nodelay(true).ok();
                            self.handle_listener(stream, addr, key, false).await;
                        }
                       Err(err) => {
                           log::error!("listener.accept failed: {}", err);
                           return LoopFailure::Listener;
                       }
                    }
                }
            }
        }
    }

    // 企业级设备认证
    async fn authenticate_device(&self, device_id: &str, token: Option<&str>) -> ResultType<Option<String>> {
        if let Some(token) = token {
            match self.auth_manager.verify_jwt(token) {
                Ok(claims) => {
                    // 检查用户是否有权限访问该设备
                    if self.auth_manager.check_permission(
                        &self.enterprise_db.get_user_by_username(&claims.username).await?.unwrap(),
                        device_id,
                        "access"
                    ) {
                        return Ok(Some(claims.sub));
                    }
                }
                Err(e) => {
                    log::warn!("Invalid JWT token for device {}: {}", device_id, e);
                }
            }
        }
        Ok(None)
    }

    // 记录设备连接会话
    async fn create_device_session(&self, device_id: String, user_id: Option<String>) -> ResultType<()> {
        let permissions = if user_id.is_some() {
            DevicePermissions {
                can_control: true,
                can_transfer_files: true,
                can_view_screen: true,
                can_use_audio: true,
                can_use_clipboard: true,
                session_timeout: Some(Duration::from_hours(8)),
            }
        } else {
            DevicePermissions::default()
        };

        let session = DeviceSession {
            device_id: device_id.clone(),
            user_id: user_id.clone(),
            authenticated: user_id.is_some(),
            last_activity: Instant::now(),
            permissions,
            connection_count: 1,
        };

        self.device_sessions.lock().await.insert(device_id.clone(), session);

        // 记录审计日志
        if let Some(user_id) = user_id {
            let audit_log = AuditLog {
                id: 0,
                user_id,
                device_id,
                action: "device_connect".to_string(),
                details: Some("设备连接".to_string()),
                ip_address: "0.0.0.0".to_string(), // 这里应该获取真实IP
                user_agent: None,
                timestamp: SystemTime::now(),
                success: true,
            };
            let _ = self.enterprise_db.log_audit(&audit_log).await;
        }

        Ok(())
    }

    // 清理过期会话
    async fn cleanup_expired_sessions(device_sessions: Arc<Mutex<HashMap<String, DeviceSession>>>) {
        let mut sessions = device_sessions.lock().await;
        let now = Instant::now();
        
        sessions.retain(|device_id, session| {
            if let Some(timeout) = session.permissions.session_timeout {
                if now.duration_since(session.last_activity) > timeout {
                    log::info!("Session expired for device: {}", device_id);
                    return false;
                }
            }
            true
        });
    }

    // 增强的UDP处理，包含企业级认证
    async fn handle_udp(
        &mut self,
        bytes: &BytesMut,
        addr: SocketAddr,
        socket: &mut FramedSocket,
        key: &str,
    ) -> ResultType<()> {
        if let Ok(msg_in) = RendezvousMessage::parse_from_bytes(bytes) {
            match msg_in.union {
                Some(rendezvous_message::Union::RegisterPeer(rp)) => {
                    if !rp.id.is_empty() {
                        log::trace!("New peer registered: {:?} {:?}", &rp.id, &addr);
                        
                        // 企业级功能：设备注册时记录设备信息
                        let device_info = DeviceInfo {
                            id: rp.id.clone(),
                            name: rp.id.clone(), // 可以从客户端获取更详细的名称
                            os: "Unknown".to_string(), // 可以从客户端获取
                            version: "Unknown".to_string(),
                            ip_address: addr.ip().to_string(),
                            mac_address: None,
                            last_online: SystemTime::now(),
                            owner_id: "system".to_string(), // 默认系统拥有，可以后续分配
                            group_ids: vec![],
                            enabled: true,
                            tags: vec![],
                        };
                        
                        let _ = self.enterprise_db.register_device(&device_info).await;
                        
                        self.update_addr(rp.id, addr, socket).await?;
                        if self.inner.serial > rp.serial {
                            let mut msg_out = RendezvousMessage::new();
                            msg_out.set_configure_update(ConfigUpdate {
                                serial: self.inner.serial,
                                rendezvous_servers: (*self.rendezvous_servers).clone(),
                                ..Default::default()
                            });
                            socket.send(&msg_out, addr).await?;
                        }
                    }
                }
                Some(rendezvous_message::Union::RegisterPk(rk)) => {
                    if rk.uuid.is_empty() || rk.pk.is_empty() {
                        return Ok(());
                    }
                    let id = rk.id;
                    let ip = addr.ip().to_string();
                    
                    // 企业级IP封锁检查
                    if id.len() < 6 {
                        return send_rk_res(socket, addr, UUID_MISMATCH).await;
                    } else if !self.check_ip_blocker(&ip, &id).await {
                        return send_rk_res(socket, addr, TOO_FREQUENT).await;
                    }
                    
                    // 其余逻辑与原版相同...
                    let peer = self.pm.get_or(&id).await;
                    let (changed, ip_changed) = {
                        let peer = peer.read().await;
                        if peer.uuid.is_empty() {
                            (true, false)
                        } else {
                            if peer.uuid == rk.uuid {
                                if peer.info.ip != ip && peer.pk != rk.pk {
                                    log::warn!(
                                        "Peer {} ip/pk mismatch: {}/{:?} vs {}/{:?}",
                                        id,
                                        ip,
                                        rk.pk,
                                        peer.info.ip,
                                        peer.pk,
                                    );
                                    drop(peer);
                                    return send_rk_res(socket, addr, UUID_MISMATCH).await;
                                }
                            } else {
                                log::warn!(
                                    "Peer {} uuid mismatch: {:?} vs {:?}",
                                    id,
                                    rk.uuid,
                                    peer.uuid
                                );
                                drop(peer);
                                return send_rk_res(socket, addr, UUID_MISMATCH).await;
                            }
                            let ip_changed = peer.info.ip != ip;
                            (
                                peer.uuid != rk.uuid || peer.pk != rk.pk || ip_changed,
                                ip_changed,
                            )
                        }
                    };
                    
                    if changed {
                        self.pm.update_pk(id, peer, addr, rk.uuid, rk.pk, ip).await;
                    }
                    
                    let mut msg_out = RendezvousMessage::new();
                    msg_out.set_register_pk_response(RegisterPkResponse {
                        result: register_pk_response::Result::OK.into(),
                        ..Default::default()
                    });
                    socket.send(&msg_out, addr).await?
                }
                Some(rendezvous_message::Union::PunchHoleRequest(ph)) => {
                    // 企业级权限检查
                    if !key.is_empty() && ph.licence_key != key {
                        let mut msg_out = RendezvousMessage::new();
                        msg_out.set_punch_hole_response(PunchHoleResponse {
                            failure: punch_hole_response::Failure::LICENSE_MISMATCH.into(),
                            ..Default::default()
                        });
                        socket.send(&msg_out, addr).await?;
                        return Ok(());
                    }
                    
                    if self.pm.is_in_memory(&ph.id).await {
                        self.handle_udp_punch_hole_request(addr, ph, key).await?;
                    } else {
                        let mut me = self.clone();
                        let key = key.to_owned();
                        tokio::spawn(async move {
                            allow_err!(me.handle_udp_punch_hole_request(addr, ph, &key).await);
                        });
                    }
                }
                _ => {
                    // 其他消息类型的处理保持与原版相同
                }
            }
        }
        Ok(())
    }

    // 其他方法保持与原版相似，但添加企业级功能...
    // 为了节省空间，这里只展示关键的企业级增强部分

    fn get_server_sk(key: &str) -> (String, Option<sign::SecretKey>) {
        // 与原版相同的实现
        let mut out_sk = None;
        let mut key = key.to_owned();
        if let Ok(sk) = base64::decode(&key) {
            if sk.len() == sign::SECRETKEYBYTES {
                log::info!("The key is a crypto private key");
                key = base64::encode(&sk[(sign::SECRETKEYBYTES / 2)..]);
                let mut tmp = [0u8; sign::SECRETKEYBYTES];
                tmp[..].copy_from_slice(&sk);
                out_sk = Some(sign::SecretKey(tmp));
            }
        }

        if key.is_empty() || key == "-" || key == "_" {
            let (pk, sk) = crate::common::gen_sk(0);
            out_sk = sk;
            if !key.is_empty() {
                key = pk;
            }
        }

        if !key.is_empty() {
            log::info!("Key: {}", key);
        }
        (key, out_sk)
    }

    // 简化的方法实现 - 实际应用中需要完整实现
    async fn update_addr(&mut self, id: String, addr: SocketAddr, socket: &mut FramedSocket) -> ResultType<()> {
        // 简化实现
        Ok(())
    }

    async fn check_ip_blocker(&self, ip: &str, id: &str) -> bool {
        // 简化实现 - 实际应该使用企业级IP封锁逻辑
        true
    }

    async fn handle_udp_punch_hole_request(&mut self, addr: SocketAddr, ph: PunchHoleRequest, key: &str) -> ResultType<()> {
        // 简化实现
        Ok(())
    }

    fn parse_relay_servers(&mut self, relay_servers: &str) {
        // 与原版相同的实现
    }

    async fn handle_listener2(&self, stream: TcpStream, addr: SocketAddr) {
        // 与原版相同的实现
    }

    async fn handle_listener(&self, stream: TcpStream, addr: SocketAddr, key: &str, ws: bool) {
        // 与原版相同的实现
    }
}

// 辅助函数
async fn check_relay_servers(rs0: Arc<RelayServers>, tx: Sender) {
    // 与原版相同的实现
}

async fn send_rk_res(
    socket: &mut FramedSocket,
    addr: SocketAddr,
    res: register_pk_response::Result,
) -> ResultType<()> {
    let mut msg_out = RendezvousMessage::new();
    msg_out.set_register_pk_response(RegisterPkResponse {
        result: res.into(),
        ..Default::default()
    });
    socket.send(&msg_out, addr).await
}

async fn create_udp_listener(port: i32, rmem: usize) -> ResultType<FramedSocket> {
    let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port as _);
    if let Ok(s) = FramedSocket::new_reuse(&addr, true, rmem).await {
        log::debug!("listen on udp {:?}", s.local_addr());
        return Ok(s);
    }
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port as _);
    let s = FramedSocket::new_reuse(&addr, true, rmem).await?;
    log::debug!("listen on udp {:?}", s.local_addr());
    Ok(s)
}

async fn create_tcp_listener(port: i32) -> ResultType<TcpListener> {
    let s = listen_any(port as _).await?;
    log::debug!("listen on tcp {:?}", s.local_addr());
    Ok(s)
}

// 导入必要的函数
use crate::common::{get_arg, get_arg_or, get_servers, listen_signal};