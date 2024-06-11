FROM postgres:16

COPY ./src/database/schema.sql /docker-entrypoint-initdb.d/init.sql
COPY ./init.sh /docker-entrypoint-initdb.d/init.sh

COPY ./postgresql.conf /tmp/postgresql.conf
COPY ./pg_hba.conf /tmp/pg_hba.conf

RUN chmod -R 755 /var
RUN chmod -R 755 /tmp

EXPOSE 5432