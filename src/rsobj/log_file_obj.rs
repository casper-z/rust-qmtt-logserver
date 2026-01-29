use tokio::io::BufWriter;
use tokio::fs::File;

/// 异步日志文件写入器
pub struct LogFile {
    pub writer: BufWriter<File>,
    pub current_size: usize,
    pub file_index: usize,
    pub last_write_time: std::time::Instant, // 记录上次写入时间
}
