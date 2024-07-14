FROM postgres:16

COPY ./src/database/schema.sql /docker-entrypoint-initdb.d/init.sql

COPY ./postgresql.conf /etc/postgresql.conf
COPY ./pg_hba.conf /etc/pg_hba.conf

EXPOSE 5432

CMD ["postgres", "-c", "config_file=/etc/postgresql.conf", "-c", "hba_file=/etc/pg_hba.conf"]