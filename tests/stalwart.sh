#!/usr/bin/env bash

set -e

docker run -d --rm --name stalwart-test-for-himalaya -p 8080:8080 stalwartlabs/stalwart:latest-alpine

sleep 1

admin_password=$(docker logs stalwart-test-for-himalaya 2>&1 | grep -oP "(?<=with password ')[^']+")

curl -u "admin:${admin_password}" -X POST -H 'Content-Type: application/json' -d '{"type":"domain","name":"pimalaya.org","description":"","quota":0,"secrets":[],"emails":[],"urls":[],"memberOf":[],"roles":[],"lists":[],"members":[],"enabledPermissions":[],"disabledPermissions":[],"externalMembers":[]}' http://localhost:8080/api/principal

curl -u "admin:${admin_password}" -X POST -H 'Content-Type: application/json' -d '{"type":"individual","name":"test","description":"","quota":0,"secrets":["test"],"emails":["test@pimalaya.org"],"memberOf":[],"roles":["user"],"lists":[],"enabledPermissions":[],"disabledPermissions":[],"externalMembers":[]}' http://localhost:8080/api/principal
