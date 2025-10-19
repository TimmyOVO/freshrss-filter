# 构建阶段
FROM rust:1.90-alpine3.20 AS builder

# 安装构建依赖
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    pkgconfig

# 设置工作目录
WORKDIR /app

# 复制Cargo文件
COPY Cargo.toml Cargo.lock ./

# 复制源代码
COPY src ./src
COPY config.example.toml ./

# 构建应用
RUN cargo build --release

# 运行阶段
FROM alpine:3.20

# 安装运行时依赖
RUN apk add --no-cache \
    ca-certificates \
    openssl

# 创建非root用户
RUN addgroup -S freshrss && adduser -S -G freshrss freshrss

# 创建应用目录
WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/freshrss-filter /usr/local/bin/

# 复制示例配置文件
COPY --from=builder /app/config.example.toml ./

# 创建数据目录
RUN mkdir -p /app/data && chown -R freshrss:freshrss /app

# 切换到非root用户
USER freshrss

# 暴露配置文件路径（可选）
VOLUME ["/app/data"]

# 设置默认配置文件路径
ENV CONFIG_PATH=/app/config.toml

# 默认命令
CMD ["/usr/local/bin/freshrss-filter", "--config", "/app/config.toml"]