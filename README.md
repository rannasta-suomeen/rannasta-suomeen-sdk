# Rannasta Suomeen SDK
Stantard development kit for rannasta_suomeen project, that is used by all `rannasta_suomeen` related services.

Includes standardized bindings and types for:
- Authentication and sessions
- Permissions
- Database schema related typings
- Database related actions
- Servise-wide error asbtraction

Also includes all of the database related configurations and schemas. This ensures that all of the services stay up-to-date with cyrrent database schema. 

Generates cargo-crate with rust bindings as well as docker-container for database related infrastrcuture that seemlesly integrates into the cd-pipeline.

## Building
```bash
docker build -t db
```

## Running docker-container locally
Initialize postgresql service
```bash
# pull or build `db`
mkdir ./pg-data # NOTE: this directory will be owned by 'postgres' aftre initialization
docker run -p 5432:5432 -e POSTGRES_PASSWORD="very_secret_password" -e POSTGRES_DB=rannasta_suomeen --mount type=bind,source=./pg-data,target=/var/lib/postgresql/data db
```
Access the postgres service
```bash
psql postgres://postgres:very_secret_password@localhost:5432/rannasta_suomeen
```


