// 企业版主程序入口
use flexi_logger::*;
use hbb_common::{bail, config::RENDEZVOUS_PORT, ResultType};
use hbbs::{common::*, *};

mod auth;
mod enterprise_database;
mod enterprise_rendezvous_server;
mod web_api;

use enterprise_rendezvous_server::EnterpriseRendezvousServer;

const RMEM: usize = 0;

fn main() -> ResultType<()> {
    // 初始化日志系统
    let _logger = Logger::try_with_env_or_str("info")?
        .log_to_stdout()
        .format(opt_format)
        .write_mode(WriteMode::Async)
        .start()?;

    // 解析命令行参数
    let args = format!(
        "-c --config=[FILE] +takes_value 'Sets a custom config file'
        -p, --port=[NUMBER(default={RENDEZVOUS_PORT})] 'Sets the listening port'
        -s, --serial=[NUMBER(default=0)] 'Sets configure update serial number'
        -R, --rendezvous-servers=[HOSTS] 'Sets rendezvous servers, separated by comma'
        -u, --software-url=[URL] 'Sets download url of RustDesk software of newest version'
        -r, --relay-servers=[HOST] 'Sets the default relay servers, separated by comma'
        -M, --rmem=[NUMBER(default={RMEM})] 'Sets UDP recv buffer size, set system rmem_max first, e.g., sudo sysctl -w net.core.rmem_max=52428800. vi /etc/sysctl.conf, net.core.rmem_max=52428800, sudo sysctl –p'
        , --mask=[MASK] 'Determine if the connection comes from LAN, e.g. 192.168.0.0/16'
        -k, --key=[KEY] 'Only allow the client with the same key'
        --enterprise 'Enable enterprise features'
        --web-port=[NUMBER] 'Web management interface port (default: main_port + 3)'
        --jwt-secret=[SECRET] 'JWT secret for authentication'
        --db-url=[URL] 'Enterprise database URL'",
    );

    init_args(&args, "hbbs-enterprise", "RustDesk Enterprise ID/Rendezvous Server");

    // 检查是否启用企业功能
    let enterprise_mode = get_arg("enterprise") == "true" || std::env::var("RUSTDESK_ENTERPRISE").is_ok();
    
    if !enterprise_mode {
        println!("启动标准版服务器...");
        return start_standard_server();
    }

    println!("启动企业版服务器...");
    println!("企业功能包括:");
    println!("  ✓ 用户认证和权限管理");
    println!("  ✓ 设备分组和批量管理");
    println!("  ✓ 审计日志和会话记录");
    println!("  ✓ Web管理界面");
    println!("  ✓ 双因素认证支持");
    println!("  ✓ 企业级安全控制");

    start_enterprise_server()
}

fn start_standard_server() -> ResultType<()> {
    let port = get_arg_or("port", RENDEZVOUS_PORT.to_string()).parse::<i32>()?;
    if port < 3 {
        bail!("Invalid port");
    }
    let rmem = get_arg("rmem").parse::<usize>().unwrap_or(RMEM);
    let serial: i32 = get_arg("serial").parse().unwrap_or(0);
    
    crate::common::check_software_update();
    
    // 使用原版服务器
    crate::rendezvous_server::RendezvousServer::start(
        port, 
        serial, 
        &get_arg_or("key", "-".to_owned()), 
        rmem
    )?;
    
    Ok(())
}

fn start_enterprise_server() -> ResultType<()> {
    // 设置企业版环境变量
    setup_enterprise_environment();
    
    let port = get_arg_or("port", RENDEZVOUS_PORT.to_string()).parse::<i32>()?;
    if port < 3 {
        bail!("Invalid port");
    }
    let rmem = get_arg("rmem").parse::<usize>().unwrap_or(RMEM);
    let serial: i32 = get_arg("serial").parse().unwrap_or(0);
    
    crate::common::check_software_update();
    
    // 使用企业版服务器
    EnterpriseRendezvousServer::start(
        port, 
        serial, 
        &get_arg_or("key", "-".to_owned()), 
        rmem
    )?;
    
    Ok(())
}

fn setup_enterprise_environment() {
    // 设置JWT密钥
    if let Some(jwt_secret) = get_arg_option("jwt-secret") {
        std::env::set_var("JWT_SECRET", jwt_secret);
    } else if std::env::var("JWT_SECRET").is_err() {
        // 生成随机JWT密钥
        let secret = generate_random_secret();
        std::env::set_var("JWT_SECRET", secret);
        println!("警告: 使用随机生成的JWT密钥。生产环境请设置固定密钥！");
    }
    
    // 设置数据库URL
    if let Some(db_url) = get_arg_option("db-url") {
        std::env::set_var("ENTERPRISE_DB_URL", db_url);
    } else if std::env::var("ENTERPRISE_DB_URL").is_err() {
        std::env::set_var("ENTERPRISE_DB_URL", "enterprise.sqlite3");
    }
    
    // 设置Web端口
    if let Some(web_port) = get_arg_option("web-port") {
        std::env::set_var("WEB_PORT", web_port);
    }
    
    // 设置其他企业级配置
    std::env::set_var("RUSTDESK_ENTERPRISE", "1");
    
    // 显示配置信息
    println!("企业版配置:");
    println!("  数据库: {}", std::env::var("ENTERPRISE_DB_URL").unwrap_or_default());
    println!("  JWT密钥: {}", if std::env::var("JWT_SECRET").is_ok() { "已配置" } else { "未配置" });
    
    if let Ok(web_port) = std::env::var("WEB_PORT") {
        println!("  Web管理界面: http://localhost:{}", web_port);
    } else {
        let main_port = get_arg_or("port", RENDEZVOUS_PORT.to_string()).parse::<i32>().unwrap_or(RENDEZVOUS_PORT);
        println!("  Web管理界面: http://localhost:{}", main_port + 3);
    }
    
    println!("  默认管理员账户: admin / admin123 (请立即修改密码!)");
}

fn generate_random_secret() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789\
                            !@#$%^&*";
    let mut rng = rand::thread_rng();
    
    (0..64)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

fn get_arg_option(name: &str) -> Option<String> {
    let value = get_arg(name);
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_random_secret() {
        let secret = generate_random_secret();
        assert_eq!(secret.len(), 64);
        assert!(secret.chars().all(|c| c.is_ascii()));
    }
}