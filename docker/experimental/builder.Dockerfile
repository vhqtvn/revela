#syntax=docker/dockerfile:1.4

FROM rust as rust-base
WORKDIR /aptos

RUN rm -f /etc/apt/apt.conf.d/docker-clean; echo 'Binary::apt::APT::Keep-Downloaded-Packages "true";' > /etc/apt/apt.conf.d/keep-cache
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt update && apt-get --no-install-recommends install -y \
        cmake \
        curl \
        clang \
        git \
        pkg-config \
        libssl-dev \
        libpq-dev \
        binutils \
        lld

# install cargo chef to cache dependencies
RUN --mount=type=cache,target=/root/.cargo <<EOT
    cargo install cargo-chef@0.1.51
    cargo install cargo-cache --no-default-features --features ci-autoclean
EOT

### Build Rust dependencies ###
FROM rust-base as planner

COPY --link . .
RUN cargo chef prepare --recipe-path recipe.json

### Build Rust code ###
FROM rust-base as builder-base

# Confirm that this Dockerfile is being invoked from an appropriate builder.
# See https://github.com/aptos-labs/aptos-core/pull/2471
# See https://github.com/aptos-labs/aptos-core/pull/2472
ARG BUILT_VIA_BUILDKIT
ENV BUILT_VIA_BUILDKIT $BUILT_VIA_BUILDKIT
RUN test -n "$BUILT_VIA_BUILDKIT" || (printf "===\nREAD ME\n===\n\nYou likely just tried run a docker build using this Dockerfile using\nthe standard docker builder (e.g. docker build). The standard docker\nbuild command uses a builder that does not respect our .dockerignore\nfile, which will lead to a build failure. To build, you should instead\nrun a command like one of these:\n\ndocker/docker-bake-rust-all.sh\ndocker/docker-bake-rust-all.sh indexer\n\nIf you are 100 percent sure you know what you're doing, you can add this flag:\n--build-arg BUILT_VIA_BUILDKIT=true\n\nFor more information, see https://github.com/aptos-labs/aptos-core/pull/2472\n\nThanks!" && false)

# cargo profile and features
ARG PROFILE
ENV PROFILE ${PROFILE}
ARG FEATURES
ENV FEATURES ${FEATURES}

RUN ARCHITECTURE=$(uname -m | sed -e "s/arm64/arm_64/g" | sed -e "s/aarch64/aarch_64/g") \
    && curl -LOs "https://github.com/protocolbuffers/protobuf/releases/download/v21.5/protoc-21.5-linux-$ARCHITECTURE.zip" \
    && unzip -o "protoc-21.5-linux-$ARCHITECTURE.zip" -d /usr/local bin/protoc \
    && unzip -o "protoc-21.5-linux-$ARCHITECTURE.zip" -d /usr/local 'include/*' \
    && chmod +x "/usr/local/bin/protoc" \
    && rm "protoc-21.5-linux-$ARCHITECTURE.zip"

# Use cargo chef
COPY  --link --from=planner /aptos/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer!
RUN <<EOT
    cargo chef cook --profile ${PROFILE} --workspace --recipe-path recipe.json
    cargo cache
EOT

COPY --link . /aptos/

FROM builder-base as aptos-node-builder

RUN --mount=type=secret,id=git-credentials,target=/root/.git-credentials \
    --mount=type=cache,target=/root/.cargo,id=node-cargo-cache \
    --mount=type=cache,target=/aptos/target,id=node-target-cache \
        docker/experimental/build-node.sh

FROM builder-base as tools-builder

RUN --mount=type=secret,id=git-credentials,target=/root/.git-credentials \
    --mount=type=cache,target=/root/.cargo,id=tools-cargo-cache \
    --mount=type=cache,target=/aptos/target,id=tools-target-cache \
        docker/experimental/build-tools.sh