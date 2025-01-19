FROM postgres:17

RUN apt update && apt install -y wget

RUN wget https://github.com/pksunkara/pgx_ulid/releases/download/v0.2.0/pgx_ulid-v0.2.0-pg17-amd64-linux-gnu.deb
RUN apt install ./pgx_ulid-v0.2.0-pg17-amd64-linux-gnu.deb
