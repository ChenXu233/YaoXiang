# 多阶段构建 Dockerfile
# 用法: docker build --build-arg TARGET=x86_64-unknown-linux-gnu -t yaoxiang .

# 构建阶段
FROM rust:1-alpine AS builder

# 接收构建目标平台参数
ARG TARGET=x86_64-unknown-linux-gnu

# 安装必要的依赖
RUN apk add --no-cache musl-dev gcc musl-dev

# 设置工作目录
WORKDIR /app

# 复制依赖文件
COPY Cargo.toml Cargo.lock ./

# 创建虚拟项目以预编译依赖
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --target ${TARGET}

# 复制源代码
COPY . .

# 编译项目
RUN cargo build --release --target ${TARGET}

# 运行阶段 - 使用最小化基础镜像
FROM alpine:latest AS runner

# 安装运行时依赖
RUN apk add --no-cache ca-certificates libgcc

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/${TARGET}/release/yaoxiang /usr/local/bin/yaoxiang

# 设置默认命令
ENTRYPOINT ["/usr/local/bin/yaoxiang"]

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s \
  CMD ["/usr/local/bin/yaoxiang", "--help"]
