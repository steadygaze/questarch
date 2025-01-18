# Inksite

## Getting started

To run a development environment, first create a file `.env` that sets some required environment variables.

```
# Important: if set, the DATABASE_URL must match the user, password and DB name.
DATABASE_URL=postgres://postgres:my_db_password@postgres/ink
POSTGRES_USER=postgres
POSTGRES_PASSWORD=my_db_password
POSTGRES_DB=ink
```

Then, run from the project root directory:

```shell
docker compose up --watch --remove-orphans
```

To attach to and run commands in the running container (for example, the `sqlx` CLI):

```shell
docker exec -it inksite bash
```

To wipe Postgres and Valkey data, simply run:

```shell
rm -rf docker/data
```

If you make changes to a `Dockerfile` or the `docker-compose.yml` environment, you may have to pass `--build` and `--remove-orphans` to `docker compose` to rebuild the images.

### Memory overcommit

The option `vm.overcommit_memory` is necessary for Valkey to ensure persistence works ([1](https://redis.io/docs/latest/develop/get-started/faq/#background-saving-fails-with-a-fork-error-on-linux), [2](https://medium.com/@akhshyganesh/redis-enabling-memory-overcommit-is-a-crucial-configuration-68dbb77dae5f)). This is a property of the host OS and not the Valkey container. If unset, Valkey will print a warning when it starts. You can dismiss the warning by running

```shell
# sysctl vm.overcommit_memory=1
```

before starting Valkey. To save this setting between reboots, add

```
vm.overcommit_memory = 1
```

to `/etc/sysctl.conf`.

## Troubleshooting

### `wasm-bindgen` version

You may see an error message like this when building the frontend after a `git pull`.

```
       Front compiling WASM
Error: at `/home/runner/.cargo/registry/src/index.crates.io-6f17d22bba15001f/cargo-leptos-0.2.24/src/compile/front.rs:51:30`

Caused by:
    0: at `/home/runner/.cargo/registry/src/index.crates.io-6f17d22bba15001f/cargo-leptos-0.2.24/src/compile/front.rs:126:10`
    1:

       it looks like the Rust project used to create this Wasm file was linked against
       version of wasm-bindgen that uses a different bindgen format than this binary:

         rust Wasm file schema version: 0.2.100
            this binary schema version: 0.2.99

       Currently the bindgen format is unstable enough that these two schema versions
       must exactly match. You can accomplish this by either updating this binary or
       the wasm-bindgen dependency in the Rust project.

       You should be able to update the wasm-bindgen dependency with:

           cargo update -p wasm-bindgen --precise 0.2.99

       don't forget to recompile your Wasm file! Alternatively, you can update the
       binary with:

           cargo install -f wasm-bindgen-cli --version 0.2.100

       if this warning fails to go away though and you're not sure what to do feel free
       to open an issue at https://github.com/rustwasm/wasm-bindgen/issues!
```

All three of the following need to be in sync:

- `wasm-bindgen` specified in `Cargo.toml`
- `wasm-bindgen-cli` installed at the user/system level in Docker (matches `wasm-bindgen`)
- `cargo-leptos` installed at the user/system level in Docker (depends on same version of `wasm-bindgen`)

All three should be pinned to compatible versions in Docker. You may need to rebuild the main container to install newer versions of the packages by running with `docker compose --watch --build` once.

