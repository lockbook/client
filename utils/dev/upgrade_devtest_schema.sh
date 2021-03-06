#!/bin/sh

# brew install postgresql

set -a
. ../containers/devtest.env

PGPASSWORD=$INDEX_DB_PASS psql \
	--host=$INDEX_DB_HOST \
	--username=$INDEX_DB_USER \
	--port=$INDEX_DB_PORT \
	$INDEX_DB_DB \
	--echo-all \
	--file=../index_db/create_db.sql
