FROM clux/muslrust AS builder

WORKDIR /kube-node-oos-controller

COPY . .

RUN cargo build --release

FROM alpine

LABEL org.opencontainers.image.source=https://github.com/dustinrouillard/kube-node-oos-controller


COPY --from=builder /kube-node-oos-controller/target/x86_64-unknown-linux-musl/release/kube-node-oos-controller /kube-node-oos-controller

CMD ["/kube-node-oos-controller"]