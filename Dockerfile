FROM postgres:16

COPY ./src/database/schema.sql /docker-entrypoint-initdb.d/init.sql
COPY ./postgresql.conf /var/lib/postgresql/data/postgresql.conf

EXPOSE 5432