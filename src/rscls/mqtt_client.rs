use rumqttc::{AsyncClient, MqttOptions, QoS, Event, Packet};
use std::time::Duration;
use chrono::{Local, TimeZone};
use std::sync::Arc;
use super::async_logger::AsyncLogger;

pub struct MqttClient {
    client: AsyncClient,
    eventloop: rumqttc::EventLoop,  // 完全拥有，无需 Arc
    pub logger: Arc<AsyncLogger>,
}

// 允许 MqttClient 在任务间移动（每个任务完全拥有自己的实例，无共享）
// rumqttc 的类型不是 Send+Sync，但我们的使用方式是每个任务独立拥有，无需共享
unsafe impl Send for MqttClient {}
unsafe impl Sync for MqttClient {}

impl MqttClient {
    pub fn new(host: &str, port: u16, client_id: &str, topic: &str, log_dir: &str, max_file_size: usize, timeout_secs: u64, log_retention_hours: i64) -> Result<Self, Box<dyn std::error::Error>> {
        let mut mqttoptions = MqttOptions::new(client_id, host, port);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
        let logger = Arc::new(AsyncLogger::with_config(topic, max_file_size, log_dir, timeout_secs, log_retention_hours)?);
        Ok(Self { client, eventloop, logger })
    }

    pub async fn subscribe(&self, topic: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.client.subscribe(topic, QoS::AtLeastOnce).await?;
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<Event, Box<dyn std::error::Error>> {
        Ok(self.eventloop.poll().await?)
    }

    pub fn handle_event(event: Event, logger: Arc<AsyncLogger>) {
        tokio::spawn(async move {
            match event {
                Event::Incoming(packet) => {
                    match packet {
                        Packet::Publish(publish) => {
                            let payload_str = String::from_utf8_lossy(&publish.payload);
                            let recv_timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
                            println!("[{}] 订阅成功: {}", recv_timestamp, payload_str);

                            // 解析 payload JSON，尝试提取 timestamp 字段
                            let payload_value: Result<serde_json::Value, _> = serde_json::from_str(&payload_str);
                            let (ext_part, raw_part) = match payload_value {
                                Ok(value) => {
                                    // 检查是否有 timestamp 字段
                                    let timestamp_str = value.get("timestamp").and_then(|v| v.as_f64()).map(|ts| {
                                        // 尝试秒级或毫秒级时间戳
                                        let secs = if ts > 1e11 { ts / 1000.0 } else { ts };
                                        Local.timestamp_opt(secs as i64, 0).single()
                                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                            .unwrap_or_default()
                                    });
                                    let ext = timestamp_str.map(|ts| format!(r#"{{"timestamp":"{}"}}"#, ts)).unwrap_or_else(|| "{}".to_string());
                                    (ext, value.to_string())
                                }
                                Err(_) => {
                                    // JSON 解析失败，记录错误信息
                                    let error_msg = format!(r#"{{"message":"json解析失败"}}"#);
                                    ("{}".to_string(), error_msg)
                                }
                            };

                            // 组装新格式的 JSONL
                            let json_line = format!(
                                r#"{{"ext":{},"raw":{}}}"#,
                                ext_part,
                                raw_part
                            );
                            if let Err(e) = logger.log(&json_line).await {
                                eprintln!("Failed to log message: {}", e);
                            }
                        }
                        Packet::ConnAck(_) => {
                            println!("[System] Connected to MQTT broker");
                        }
                        _ => {}
                    }
                }
                Event::Outgoing(_) => {}
            }
        });
    }
}
