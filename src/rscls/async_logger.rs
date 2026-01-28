use tokio::fs::{OpenOptions, File, read_dir, remove_file};
use tokio::io::{AsyncWriteExt, BufWriter};
use std::sync::Arc;
use tokio::sync::mpsc;
use std::path::PathBuf;
use chrono::{Local, DateTime};
use std::time::Duration;

/// 异步日志写入器
pub struct AsyncLogger {
    tx: mpsc::Sender<String>,
}

struct LogFile {
    writer: BufWriter<File>,
    current_size: usize,
    file_index: usize,
    last_write_time: std::time::Instant,  // 记录上次写入时间
}

/// 解析文件名中的时间戳，格式：YYYY-MM-DD_HH-MM-SS-topic-NN.jsonl
fn parse_log_filename(filename: &str) -> Option<DateTime<Local>> {
    // 匹配格式：2024-01-15_10-30-45_topic_name-00.jsonl
    // 先找到第一个下划线之前的时间部分
    if let Some(end_pos) = filename.find('_') {
        let timestamp_part = &filename[..end_pos]; // "2024-01-15_10-30-45"
        // 将下划线替换为空格以符合 datetime 格式
        let ts = timestamp_part.replace('_', " ");
        DateTime::parse_from_str(&ts, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|dt| dt.with_timezone(&Local))
    } else {
        None
    }
}

/// 清理过期日志文件
async fn cleanup_expired_logs(base_dir: &str, retention_hours: i64) {
    if retention_hours <= 0 {
        return;
    }

    let dir_path = PathBuf::from(base_dir);

    let Ok(mut entries) = read_dir(&dir_path).await else {
        return;
    };

    let now = Local::now();
    let threshold = chrono::Duration::hours(retention_hours);

    let mut cleaned_count = 0;
    let mut cleaned_size = 0u64;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        // 只处理 .jsonl 文件
        if !filename.ends_with(".jsonl") {
            continue;
        }

        if let Some(file_time) = parse_log_filename(filename) {
            if file_time < now - threshold {
                if let Ok(metadata) = entry.metadata().await {
                    cleaned_size += metadata.len();
                }
                if let Err(e) = remove_file(&path).await {
                    eprintln!("Failed to remove old log file {}: {}", filename, e);
                } else {
                    cleaned_count += 1;
                    println!("[{}] 清理过期日志: {}", now.format("%Y-%m-%d %H:%M:%S"), filename);
                }
            }
        }
    }

    if cleaned_count > 0 {
        let size_mb = cleaned_size as f64 / 1024.0 / 1024.0;
        println!("[{}] 日志清理完成: 删除了 {} 个文件，释放 {:.2} MB",
            now.format("%Y-%m-%d %H:%M:%S"), cleaned_count, size_mb);
    }
}

impl AsyncLogger {
    /// 创建日志写入器，指定最大文件大小和目录
    /// timeout_secs: 超过多少秒没新数据则创建新日志文件
    /// log_retention_hours: 日志保留小时数，0 表示不自动清理
    pub fn with_config(topic: &str, max_file_size: usize, base_dir: &str, timeout_secs: u64, log_retention_hours: i64) -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, mut rx) = mpsc::channel::<String>(100);
        let topic_name = topic.to_string();
        let max_file_size = Arc::new(max_file_size);
        let base_dir = Arc::new(base_dir.to_string());
        let timeout = Duration::from_secs(timeout_secs);

        // 异步写入任务
        let base_dir_clone = base_dir.clone();
        let retention_hours_clone = log_retention_hours;

        tokio::spawn(async move {
            let mut current_file: Option<LogFile> = None;
            let mut file_index = 0usize;

            // 话题中的 / 转为 _ 用于文件名
            let safe_topic_name = topic_name.replace('/', "_");

            // 启动后台定时清理任务（每小时执行一次）
            if retention_hours_clone > 0 {
                let cleanup_base_dir = base_dir_clone.to_string();
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600)); // 1小时
                    loop {
                        interval.tick().await;
                        cleanup_expired_logs(&cleanup_base_dir, retention_hours_clone).await;
                    }
                });
            }

            while let Some(line) = rx.recv().await {
                // 确保 logs 目录存在
                let dir_path = PathBuf::from(base_dir.as_str());
                if let Err(e) = tokio::fs::create_dir_all(&dir_path).await {
                    eprintln!("Failed to create log directory: {}", e);
                    continue;
                }

                // 检查是否需要超时轮转（超过1秒没新数据则创建新文件）
                if let Some(ref mut file) = current_file {
                    if file.last_write_time.elapsed() > timeout {
                        // 关闭当前文件，下次写入时创建新的
                        if let Err(e) = file.writer.flush().await {
                            eprintln!("Failed to flush on timeout: {}", e);
                        }
                        // 超时离线后重新连接，索引重置为 00 重新开始
                        file_index = 0;
                        current_file = None;
                        println!("[{}] 设备超时离线，创建新日志文件", Local::now().format("%Y-%m-%d %H:%M:%S"));
                    }
                }

                // 检查是否需要打开文件
                if current_file.is_none() {
                    // 在第一条消息到达时才获取当前时间
                    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
                    let filename = format!("{}-{}-{:02}.jsonl", timestamp, safe_topic_name, file_index);
                    let filepath = dir_path.join(&filename);

                    let file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&filepath)
                        .await
                        .expect("Failed to open log file");

                    let metadata = file.metadata().await.expect("Failed to get metadata");
                    let current_size = metadata.len() as usize;

                    current_file = Some(LogFile {
                        writer: BufWriter::new(file),
                        current_size,
                        file_index,
                        last_write_time: std::time::Instant::now(),
                    });
                }

                let line_size = line.len() + 1; // +1 for newline
                let logfile = current_file.as_mut().unwrap();

                // 检查是否需要文件轮转（大小超限）
                if logfile.current_size + line_size >= *max_file_size {
                    // 刷新并关闭当前文件
                    if let Err(e) = logfile.writer.flush().await {
                        eprintln!("Failed to flush file: {}", e);
                    }

                    // 索引达到 99 时重置为 0
                    file_index = (file_index + 1) % 100;
                    // 使用当前时间作为新文件名
                    let new_timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
                    let filename = format!("{}-{}-{:02}.jsonl", new_timestamp, safe_topic_name, file_index);
                    let filepath = dir_path.join(&filename);

                    let new_file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&filepath)
                        .await
                        .expect("Failed to create new log file");

                    logfile.writer = BufWriter::new(new_file);
                    logfile.current_size = 0;
                    logfile.file_index = file_index;
                }

                // 写入日志
                if let Err(e) = logfile.writer.write_all(line.as_bytes()).await {
                    eprintln!("Failed to write to log file: {}", e);
                    continue;
                }
                if let Err(e) = logfile.writer.write_all(b"\n").await {
                    eprintln!("Failed to write newline: {}", e);
                    continue;
                }

                // 每次写入后刷新，确保数据不丢失
                if let Err(e) = logfile.writer.flush().await {
                    eprintln!("Failed to flush: {}", e);
                }

                logfile.current_size += line_size;
                logfile.last_write_time = std::time::Instant::now();  // 更新最后写入时间
            }
        });

        Ok(Self { tx })
    }

    /// 异步记录 JSON 行
    pub async fn log(&self, json_line: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.tx.send(json_line.to_string()).await?;
        Ok(())
    }
}

