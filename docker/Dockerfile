# FROM rust:1.93.0
# FROM rust:1.93.0-bullseye
# FROM docker.m.daocloud.io/rust:1.85.0-bullseye
FROM docker.m.daocloud.io/rust:1.93.0-bullseye
# FROM swr.cn-north-4.myhuaweicloud.com/ddn-k8s/docker.io/rust:1.90-alpine3.20
# FROM swr.cn-north-4.myhuaweicloud.com/ddn-k8s/docker.io/rust:1.85.0-bullseye

# --- 配置国内镜像源
RUN mkdir -p $CARGO_HOME && \
    echo '[source.crates-io]' > $CARGO_HOME/config.toml && \
    echo 'replace-with = "tuna"' >> $CARGO_HOME/config.toml && \
    echo '[source.tuna]' >> $CARGO_HOME/config.toml && \
    echo 'registry = "sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"' >> $CARGO_HOME/config.toml