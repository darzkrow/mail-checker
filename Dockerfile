# Multi-stage build usando Rust stable oficial (actualmente Rust 1.88+)
FROM rust:latest AS builder

WORKDIR /app

# Instalar dependencias necesarias para compilar paquetes nativos
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copiar configuración de workspace y paquetes modulares
COPY Cargo.toml ./
COPY packages ./packages

# Compilar en modo release
RUN cargo build --release --bin email-validator

# Runtime: Debian Slim con glibc compatible
FROM debian:bookworm-slim

WORKDIR /app

# Certificados SSL para resolución segura de DNS y curl para healthcheck
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 1000 appuser

# Copiar el binario compilado
COPY --from=builder --chown=appuser:appuser /app/target/release/email-validator .

# Permisos de ejecución explícitos
RUN chmod +x ./email-validator

USER appuser

EXPOSE 3000

# Health check usando curl
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

CMD ["./email-validator"]