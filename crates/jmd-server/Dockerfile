# Export API plus the browser build, in one small image.
#   docker build -f crates/jmd-server/Dockerfile -t jmd-server .
#   docker run -p 8787:8787 jmd-server
FROM node:22-bookworm-slim AS web
WORKDIR /src
RUN corepack enable
COPY . .
RUN pnpm install --frozen-lockfile && pnpm build

FROM rust:1-bookworm AS server
WORKDIR /src
COPY . .
COPY --from=web /src/apps/desktop/dist apps/desktop/dist
RUN cargo build --release -p jmd-server && bash scripts/fetch-fonts.sh /fonts

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates fonts-dejavu-core && rm -rf /var/lib/apt/lists/*
COPY --from=server /src/target/release/jmd-server /usr/local/bin/jmd-server
COPY --from=server /fonts /fonts
COPY --from=web /src/apps/desktop/dist /web
ENV JMD_ADDR=0.0.0.0:8787 JMD_STATIC=/web JMD_FONT_DIRS=/fonts
EXPOSE 8787
USER nobody
CMD ["jmd-server"]
