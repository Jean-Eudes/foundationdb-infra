# simple example of S3 server build in front if foundationdb

  - init project
  - first endpoint with axum
  - save and load file with axum on disk
  - save file with one key on foundationdb (only little file)
  - create a transaction to store middle file in many key.
  - add bucket
  - add habilitation

# commandes

```
podman run -d -p 4500:4500 --network podman foundationdb/foundationdb:7.3.57
podman exec 86a2836285be fdbcli --exec "configure new single memory"
podman build . -t foundationdb-s3
podman run  -d --network podman -p 3000:3000 foundationdb-s3:latest
```
