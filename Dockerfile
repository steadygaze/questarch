# Example: https://book.leptos.dev/deployment/ssr.html#creating-a-containerfile

FROM rustlang/rust:nightly

RUN apt update && apt install -y pkg-config libssl-dev wget

# Install nvm with node and npm
ENV NODE_VERSION=22.14.0
RUN wget -qO- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
ENV NVM_DIR=/root/.nvm
RUN . "$NVM_DIR/nvm.sh" && nvm install ${NODE_VERSION}
RUN . "$NVM_DIR/nvm.sh" && nvm use v${NODE_VERSION}
RUN . "$NVM_DIR/nvm.sh" && nvm alias default v${NODE_VERSION}
ENV PATH="/root/.nvm/versions/node/v${NODE_VERSION}/bin/:${PATH}"
RUN node --version
RUN npm --version

# cargo-binstall makes it easier to install other cargo extensions
RUN wget https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz
RUN tar -xvf cargo-binstall-x86_64-unknown-linux-musl.tgz
RUN cp cargo-binstall /usr/local/cargo/bin

# Install cargo extensions
# The cargo-leptos version must depend on matching wasm-bindgen (Cargo.toml) and
# wasm-bindgen-cli (here) versions.
RUN cargo binstall cargo-leptos@0.2.26 -y
RUN cargo binstall sqlx-cli -y
# wasm-bindgen-cli version must match Cargo.toml exactly.
RUN cargo binstall wasm-bindgen-cli@0.2.100 -y

# Add the WASM target
RUN rustup target add wasm32-unknown-unknown

# Make an /app dir, which everything will eventually live in
WORKDIR /app
COPY . /app

# Install DaisyUI inputs to Tailwind; uses WORKDIR
RUN npm install

# Set any required env variables and
ENV RUST_LOG="info"
# ENV APP_ENVIRONMENT="production"
ENV LEPTOS_SITE_ADDR="0.0.0.0:3000"
ENV LEPTOS_SITE_ROOT="site"
# 3000 for serving, 3001 for live reload
EXPOSE 3000 3001
# SIGTERM doesn't work for some reason
STOPSIGNAL SIGINT

# Run the server
ENTRYPOINT ["/usr/local/cargo/bin/cargo-leptos"]
CMD ["watch"]
