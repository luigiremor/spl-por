FROM rust:1.94-bookworm
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev libudev-dev clang cmake && rm -rf /var/lib/apt/lists/*
WORKDIR /spl-por
COPY . .
RUN cargo build --release -p verifier 2>&1 | tail -3
CMD ["sh","-c","./target/release/por-verify attestation --mint FsXE6uHbMiwhj8inCAEAVpN3CNbWd5vFwuBy5AyxE2rC --sequence 0 --rpc https://api.devnet.solana.com --dataset results/devnet-datasets/n10 --json; echo EXIT=$?"]
