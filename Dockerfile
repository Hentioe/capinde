# Use Rust as the builder based on Void Linux
FROM hentioe/rust:1.88.0-void-bindings AS builder
COPY . /src/
WORKDIR /src
RUN set -xe \
  && xbps-install -Sy libmagick-devel \
  && cargo build --release

# Use Void Linux as the runner
FROM ghcr.io/void-linux/void-glibc-busybox:20250701R1 AS runner
COPY --from=builder /src/target/release/capinde /usr/local/bin/capinde
WORKDIR /home/capinde
ENV RUST_LOG=INFO
COPY .docker/cleanup.sh /usr/bin/void-cleanup
RUN set -xe \
  && xbps-install -Sy libmagick \
  && void-cleanup
EXPOSE 8080
ENTRYPOINT [ "capinde" ]
