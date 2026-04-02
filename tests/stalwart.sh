#!/usr/bin/env bash

set -eu

container=$(docker run -d --name himalaya-tests --rm -p 8080:8080 -p 25:25 -p 143:143 stalwartlabs/stalwart:v0.15.5-alpine)
sleep 1
admin_password=$(docker logs $container 2>&1 | grep -oP "(?<=with password ')[^']+")

curl -sX POST \
     -u "admin:${admin_password}" \
     -H 'Content-Type: application/json' \
     -d '{"type":"domain","name":"pimalaya.org"}' \
     http://localhost:8080/api/principal

curl -X POST \
     -u "admin:${admin_password}" \
     -H 'Content-Type: application/json' \
     -d '{"type":"individual","name":"test","emails":["test@pimalaya.org"],"secrets":["test"],"roles":["user"]}' \
     http://localhost:8080/api/principal
