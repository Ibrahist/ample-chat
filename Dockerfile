# Multi-stage Dockerfile for ample-chat
# Stage 1: Build static musl binary
FROM rust:1.80-alpine AS builder

WORKDIR /usr/src/ample-chat

RUN apk add --no-cache musl-dev build-base perl

COPY . .

# Build static release binary with musl target
RUN cargo build --release --bin ample-chat

# Stage 2: Distroless minimal scratch container
FROM gcr.io/distroless/static-debian12:nonroot

WORKDIR /app

# Run as non-root user (id 65532 is nonroot in distroless)
USER 65532:65532

COPY --from=builder --chown=65532:65532 /usr/src/ample-chat/target/release/ample-chat /usr/local/bin/ample-chat

EXPOSE 8000

ENV AMPLE_CHAT_HOST=0.0.0.0
ENV AMPLE_CHAT_PORT=8000

ENTRYPOINT ["/usr/local/bin/ample-chat"]
CMD ["serve", "0.0.0.0", "8000"]
