rm -r ./pg-data
mkdir pg-data

docker run -p 5432:5432 -e POSTGRES_PASSWORD="very_secret_password" -e POSTGRES_DB=rannasta_suomeen --mount type=bind,source=./pg-data,target=/var/lib/postgresql/data db