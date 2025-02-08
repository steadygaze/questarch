FROM postgres:17

ADD install_pg_uuidv7.sh /install_pg_uuidv7.sh

RUN apt update && apt install -y curl
RUN /install_pg_uuidv7.sh
