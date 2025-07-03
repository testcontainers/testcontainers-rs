FROM lukemathwalker/cargo-chef:latest-rust-latest as chef
WORKDIR /app
# RUN apt-get update -y && apt-get install -y lld clang

FROM chef as planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
# Build our project
RUN cargo build -v --release --bin simple_web_server

FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/simple_web_server foo
COPY --from=builder /app/target/release/simple_web_server bar
EXPOSE 80
ENTRYPOINT ["./foo"]
