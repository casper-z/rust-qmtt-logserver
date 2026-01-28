mod rscls;
use rscls::mqtt_client::MqttClient;
use rscls::async_logger::AsyncLogger;
use std::sync::Arc;

#[derive(serde::Deserialize)]
struct Config {
    log_dir: String,
    max_file_size_mb: usize,
    topics: Vec<String>,
    timeout_secs: u64,
    log_retention_hours: i64,
    host: String,
    port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_dir: "logs".to_string(),
            max_file_size_mb: 100,
            topics: vec!["subscribe001".to_string()],
            timeout_secs: 1,
            log_retention_hours: 0,
            host: "192.168.1.13".to_string(),
            port: 41883,
        }
    }
}

fn load_config() -> Config {
    let config_path = "config.toml";
    match std::fs::read_to_string(config_path) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

// 客户端运行函数（只负责循环，不需要处理创建失败）
async fn run_client(mut client: MqttClient, topic: String) {
    if let Err(e) = client.subscribe(&topic).await {
        println!("Failed to subscribe to {}: {:?}", topic, e);
        return;
    }
    println!("Subscribed to topic: {}", topic);

    // 获取 logger 的 Arc 引用（完全拥有，无需共享）
    let logger: Arc<AsyncLogger> = Arc::clone(&client.logger);

    // 轮询 MQTT 事件
    loop {
        match client.next_event().await {
            Ok(event) => {
                MqttClient::handle_event(event, logger.clone());
            }
            Err(e) => {
                eprintln!("Event error for {}: {:?}", topic, e);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- 读取配置
    let config = load_config();

    // --- define
    let max_file_size = config.max_file_size_mb * 1024 * 1024; // MB 转字节

    // 为每个话题创建客户端，然后在独立任务中运行
    // 关键：先创建客户端，只 spawn 已成功的实例
    let mut handles = Vec::new();
    for topic in &config.topics {
        let host = config.host.clone();
        let port = config.port;
        let topic = topic.clone();
        let log_dir = config.log_dir.clone();
        let max_file_size = max_file_size;
        let timeout_secs = config.timeout_secs;
        let log_retention_hours = config.log_retention_hours;

        // 为每个话题生成唯一的 client_id（topic 中的 / 转为 _）
        let client_id = format!("mqtt_subscriber_{}", topic.replace('/', "_"));

        // 在 spawn 之前创建客户端并订阅，只 spawn 已成功的运行实例
        match MqttClient::new(&host, port, &client_id, &topic, &log_dir, max_file_size, timeout_secs, log_retention_hours) {
            Ok(client) => {
                // 只 spawn 已成功创建的客户端
                let handle = tokio::spawn(async move {
                    run_client(client, topic).await;
                });
                handles.push(handle);
            }
            Err(e) => {
                println!("Failed to create client for {}: {:?}", topic, e);
            }
        }
    }

    // 主线程保持运行
    if handles.is_empty() {
        println!("No MQTT clients created. Exiting.");
        return Ok(());
    }
    println!("MQTT subscribers started. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await.ok();

    // 等待所有任务完成
    for handle in handles {
        handle.abort();
    }

    Ok(())
}
