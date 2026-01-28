[English](./README.md) | [简体中文](./README-CN.md)

# MQTT Log Subscriber Service (mqtt_subscriber)

An MQTT log subscriber service written in Rust that receives MQTT messages and persists data to log files.

## Features

- **Multi-topic Subscription**: Subscribe to multiple MQTT topics simultaneously with independent processing for each topic
- **JSON Message Parsing**: Automatically parse JSON-formatted MQTT messages
- **Log Persistence**: Write logs in JSONL (JSON Lines) format
- **Device Timeout Detection**: Supports device offline detection and automatically creates new log files when devices go offline
- **Automatic Log Rotation**: Rotate logs based on file size and timeout duration
- **Expired Log Cleanup**: Automatically clean up log files beyond retention period
- **Concurrent Processing**: Multi-topic concurrent processing for efficiency and stability

## Use Cases

- **CAN Log Recording** (canlog): Record CAN bus data from vehicles or industrial equipment
- **Event Log Recording** (eventlog): Record system events and status changes
- **Device Data Collection**: Collect sensor data from various IoT devices
- **MQTT Message Persistence**: Store MQTT messages long-term for subsequent analysis

## Requirements

- **Rust Version**: 1.93.0 (using Rust 2024 Edition)
- **Docker**: >= 20.10
- **Docker Compose**: >= v2

## Quick Start

### 1. Clone the Repository

```bash
git clone <repository-url>
cd sample004-b1
```

### 2. Docker Deployment

The project provides Docker deployment managed via `docker-compose`:

```bash
cd docker
```

Execute the following commands from the project root to start the service:

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

The above commands will:
1. Enter the docker directory
2. Get the project root directory path
3. Stop and remove existing containers
4. Build and start new containers (detached mode)
5. View container logs in real-time

### 3. Manual Running (Development/Debug Only)

```bash
# Build the project
cargo build --release

# Run the service
./target/release/mqtt_subscriber
```

## Configuration

The configuration file is `config.toml` located in the project root.

### Configuration Options

```toml
# Log configuration
log_dir = "logs"                    # Log file storage path
max_file_size_mb = 1                # Maximum log file size (MB)

# MQTT topic list
topics = ["subscribe001", "hs/canlog001", "bg/vehicle/state"]

# Device timeout (seconds)
# Devices are considered offline if no new messages received within this time
timeout_secs = 1

# Log retention period (hours)
# Log files older than this will be automatically cleaned up; 0 disables auto-cleanup
log_retention_hours = 24

# MQTT server configuration
host = "192.168.1.13"
port = 41883
```

### Configuration Reference

| Option | Description |
|--------|-------------|
| `log_dir` | Log file directory, supports relative or absolute paths |
| `max_file_size_mb` | Maximum log file size; new file created when exceeded |
| `topics` | List of MQTT topics to subscribe to, supports multiple topics |
| `timeout_secs` | Device timeout in seconds; device considered offline if no data received |
| `log_retention_hours` | Log retention period in hours; 0 disables auto-cleanup |
| `host` | MQTT server address |
| `port` | MQTT server port |

### Log Filename Format

Log files are named using the following format:

```
YYYY-MM-DD_HH-MM-SS-topic_name-NN.jsonl
```

- `/` in topics is automatically converted to `_` (e.g., `device/data` -> `device_data`)
- `-NN` is the file index (00-99), used for log rotation

### Log Content Format

```json
{"ext":{"timestamp":"2025-01-28 10:30:00"},"raw":{original JSON message}}
```

## Docker Compose Configuration

### compose.yml Reference

```yaml
version: '3.5'
services:
    rust-logserver:
        image: rust-logserver:2025
        dns:
            - 8.8.8.8
            - 8.8.4.4
        environment:
            TZ: Asia/Shanghai        # Timezone configuration (Shanghai time)
        volumes:
            - ${project_dir}:${project_dir}
        network_mode: host           # Use host network
        container_name: rust-logserver
        cap_add:
            - SYS_ADMIN
        privileged: true
        working_dir: ${project_dir}
        command: bash run.sh         # Run in release mode
        restart: always
```

### Common Docker Operations

```bash
# View logs
docker-compose logs -f

# Stop service
docker-compose down

# Restart service
docker-compose restart

# Check container status
docker-compose ps
```

## Project Structure

```
sample004-b1/
├── Cargo.toml              # Rust project configuration
├── Cargo.lock              # Dependency lock file
├── config.toml             # Application configuration file
├── README.md               # Project documentation
├── run.sh                  # Startup script
├── docker/
│   ├── compose.yml         # Docker Compose configuration
│   └── Dockerfile          # Docker image build file
├── src/
│   ├── main.rs             # Main program entry point
│   └── rscls/
│       ├── mod.rs          # Module definition
│       ├── mqtt_client.rs  # MQTT client implementation
│       └── async_logger.rs # Async log writer
└── logs/                   # Log file storage directory
```

## Dependencies

| Library | Version | Purpose |
|---------|---------|---------|
| tokio | 1.x | Async task scheduling |
| rumqttc | 0.24 | MQTT client |
| chrono | 0.4 | Date/time handling |
| serde | 1.0 | Serialization framework |
| serde_json | 1.0 | JSON parsing |
| toml | 0.8 | TOML configuration parsing |

## License

[License Type]
