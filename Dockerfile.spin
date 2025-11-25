FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y curl git && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://spinframework.dev/downloads/install.sh | bash && \
    mv spin /usr/local/bin/
WORKDIR /app
COPY spin.toml .
COPY target target
COPY static static
EXPOSE 80
CMD ["spin", "up", "--listen", "0.0.0.0:80"]