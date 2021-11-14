ARG BUILDER_ARCH=armv7-musleabihf
ARG TARGET_ARCH=armv7-unknown-linux-musleabihf

FROM messense/rust-musl-cross:${BUILDER_ARCH} AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG TARGET_ARCH
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target=${TARGET_ARCH} --recipe-path recipe.json
COPY . .
RUN cargo build --release --target=${TARGET_ARCH}
RUN musl-strip -s /app/target/${TARGET_ARCH}/release/october

FROM scratch AS runtime
ARG VERSION
ARG BUILD_DATE
ARG TARGET_ARCH
LABEL version="${VERSION}" \
    description="Wake-on-lan webapp - Wake me up, when september ends" \
    org.label-schema.schema-version="1.0" \
    org.label-schema.name="october" \
    org.label-schema.description="Wake-on-lan webapp - Wake me up, when september ends" \
    org.label-schema.build-date="${BUILD_DATE}" \
    org.label-schema.url="https://github.com/bltavares/october" \
    org.label-schema.version="${VERSION}" \
    org.label-schema.docker.cmd="docker run -d \
    --restart=unless-stopped \
    --network=host \
    -p 3493:3493 \
    -v sample.csv:/opt/sample.csv \
    --name october \
    bltavares/october -a /opt/sample.csv"
COPY --from=builder /app/target/${TARGET_ARCH}/release/october /usr/local/bin/
WORKDIR /app
ENTRYPOINT ["/usr/local/bin/october"]
