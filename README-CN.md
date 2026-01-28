[English](./README.md) | 简体中文
# MQTT 日志订阅服务 (mqtt_subscriber)

一个基于 Rust 语言编写的 MQTT 日志订阅服务，用于接收 MQTT 消息并将数据持久化保存到日志文件中。

## 功能特性

- **多话题订阅**：支持同时订阅多个 MQTT 话题，每个话题独立处理
- **JSON 消息解析**：自动解析 JSON 格式的 MQTT 消息
- **日志持久化**：以 JSONL (JSON Lines) 格式写入日志文件
- **设备超时检测**：支持设备离线检测，离线后自动创建新的日志文件
- **日志自动轮转**：按文件大小和超时时间自动轮转
- **过期日志清理**：自动清理超过保留时间的日志文件
- **并发处理**：多话题并发处理，高效稳定

## 适用场景

- **CAN 日志记录** (canlog)：记录车辆或工业设备的 CAN 总线数据
- **事件日志记录** (eventlog)：记录系统事件和状态变化
- **设备数据采集**：采集各类物联网设备的传感器数据
- **MQTT 消息持久化**：将 MQTT 消息长期存储用于后续分析

## 环境要求

- **Rust 版本**：1.93.0 (使用 Rust 2024 Edition)
- **Docker**：>= 20.10
- **Docker Compose**：>= v2

## 快速开始

### 1. 克隆项目

```bash
git clone <repository-url>
cd sample004-b1
```

### 2. Docker 方式部署

项目提供了 Docker 部署方式，通过 `docker-compose` 进行管理：

```bash
cd docker
```

在项目根目录下执行以下命令启动服务：

```bash
echo "RUN: $(date)" \
&& project_path="$(pwd)/docker" \
&& cd "${project_path}" \
&& project_dir=$(dirname "$(pwd)") \
&& export project_dir \
&& sudo -E docker-compose --file compose.yml down \
&& sudo -E docker-compose --file compose.yml up --detach --build \
&& sudo -E docker-compose --file compose.yml logs --follow
```

上述命令将：
1. 进入 docker 目录
2. 获取项目根目录路径
3. 停止并删除已有容器
4. 构建并启动新容器（后台模式）
5. 实时查看容器日志

### 3. 手动运行（仅限开发调试）

```bash
# 编译项目
cargo build --release

# 运行服务
./target/release/mqtt_subscriber
```

## 配置文件

配置文件为 `config.toml`，位于项目根目录。

### 配置项说明

```toml
# 日志配置
log_dir = "logs"                    # 日志文件存放路径
max_file_size_mb = 1                # 单个日志文件最大大小（MB）

# MQTT 话题列表
topics = ["subscribe001", "hs/canlog001", "bg/vehicle/state"]

# 设备超时时间（秒）
# 超过此时间没有新消息则认为设备离线
timeout_secs = 1

# 日志保留时间（小时）
# 超过此时间的日志文件将被自动清理，0 表示不自动清理
log_retention_hours = 24

# MQTT 服务器配置
host = "192.168.1.13"
port = 41883
```

### 配置说明

| 配置项 | 说明 |
|--------|------|
| `log_dir` | 日志文件存放目录，支持相对路径或绝对路径 |
| `max_file_size_mb` | 单个日志文件的最大大小，超过后自动创建新文件 |
| `topics` | 要订阅的 MQTT 话题列表，支持多个话题 |
| `timeout_secs` | 设备超时时间（秒），超过此时间无新数据则判定设备离线 |
| `log_retention_hours` | 日志保留时间（小时），0 表示不自动清理 |
| `host` | MQTT 服务器地址 |
| `port` | MQTT 服务器端口 |

### 日志文件名格式

日志文件按以下格式命名：

```
YYYY-MM-DD_HH-MM-SS-topic_name-NN.jsonl
```

- `/` 在话题中会自动转为 `_` (如 `device/data` -> `device_data`)
- `-NN` 为文件索引 (00-99)，用于日志轮转

### 日志内容格式

```json
{"ext":{"timestamp":"2025-01-28 10:30:00"},"raw":{原始JSON消息}}
```

## Docker Compose 配置

### compose.yml 说明

```yaml
version: '3.5'
services:
    rust-logserver:
        image: rust-logserver:2025
        dns:
            - 8.8.8.8
            - 8.8.4.4
        environment:
            TZ: Asia/Shanghai        # 时区配置（上海时间）
        volumes:
            - ${project_dir}:${project_dir}
        network_mode: host           # 使用宿主机网络
        container_name: rust-logserver
        cap_add:
            - SYS_ADMIN
        privileged: true
        working_dir: ${project_dir}
        command: bash run.sh         # 运行 release 模式
        restart: always
```

### Docker 常用操作

```bash
# 查看日志
docker-compose logs -f

# 停止服务
docker-compose down

# 重启服务
docker-compose restart

# 查看容器状态
docker-compose ps
```

## 项目结构

```
sample004-b1/
├── Cargo.toml              # Rust 项目配置
├── Cargo.lock              # 依赖锁定文件
├── config.toml             # 应用配置文件
├── README.md               # 项目说明
├── run.sh                  # 启动脚本
├── docker/
│   ├── compose.yml         # Docker Compose 配置
│   └── Dockerfile          # Docker 镜像构建文件
├── src/
│   ├── main.rs             # 主程序入口
│   └── rscls/
│       ├── mod.rs          # 模块定义
│       ├── mqtt_client.rs  # MQTT 客户端实现
│       └── async_logger.rs # 异步日志写入器
└── logs/                   # 日志文件存放目录
```

## 依赖库

| 库名 | 版本 | 用途 |
|------|------|------|
| tokio | 1.x | 异步任务调度 |
| rumqttc | 0.24 | MQTT 客户端 |
| chrono | 0.4 | 日期时间处理 |
| serde | 1.0 | 序列化框架 |
| serde_json | 1.0 | JSON 解析 |
| toml | 0.8 | TOML 配置解析 |

## 许可证

[许可证类型]
