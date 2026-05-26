# FixFiction

FixFiction is a service that fixes embedded content from FimFiction.net.

Maintained by [Silk Rose](https://github.com/SilkRose).

## Quickstart

Clone this repo and build it with Cargo:

```bash
git clone https://github.com/SilkRose/FixFiction.git

cd FixFiction
cargo build
```

If you wish to run FixFiction, or change the database schema, you will need:

- A PostgreSQL server with a suitable (empty) database
- A `.env` file at the project root with DATABASE_URL = a [Connection URI](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING-URIS), e.g.:
  `DATABASE_URL=postgres://user:password@host/database`

## Docker

FixFiction reads runtime configuration from a workdir env file. Compose mounts
your project `.env` into the container at `/app/.env`; the file is not copied
into any image.

Because the container runs as a non-root user, the mounted `.env` must be
readable by that user.

### External PostgreSQL

To run only FixFiction and connect to an existing PostgreSQL server, set
`DATABASE_URL` to that server and start only `app`:

```bash
docker compose up --build app
```

Name the `app` service explicitly when you do not want the bundled PostgreSQL
container. Plain `docker compose up` starts all default services in
`compose.yml`, including `postgres`. Add `-d` after `up` to run detached.

The `.env` file only needs the app settings:

```env
DATABASE_URL=postgres://user:password@database-host:5432/fixfiction
BEARER_TOKEN=your_fimfiction_api_token
```

Keep Docker hostnames in mind:

- If PostgreSQL is another Compose service, the hostname is the service name,
  such as `postgres`.
- If PostgreSQL is running on another machine, use that machine's DNS name or IP
  address.
- If PostgreSQL is running directly on the Docker host, see the next section.

The service will be available at <http://localhost:7669>.

### Host PostgreSQL

Use this mode when PostgreSQL is installed directly on the same machine that is
running Docker, not as a Compose service.

From inside a container, `localhost` means the container itself. Use
`host.docker.internal` to reach the Docker host. On Linux, the included
`compose.host-postgres.yml` file maps that hostname to Docker's host gateway.

Set `.env` like this:

```env
DATABASE_URL=postgres://user:password@host.docker.internal:5432/fixfiction
BEARER_TOKEN=your_fimfiction_api_token
```

Run only the app with the host-PostgreSQL override:

```bash
docker compose -f compose.yml -f compose.host-postgres.yml up --build app
```

For a detached run:

```bash
docker compose -f compose.yml -f compose.host-postgres.yml up -d app
```

The host PostgreSQL server must listen on an address reachable from Docker. Find
the Compose network gateway:

```bash
docker compose -f compose.yml -f compose.host-postgres.yml up -d app
docker network inspect fixfiction_default --format '{{range .IPAM.Config}}{{println .Gateway}}{{end}}'
```

Use that gateway in PostgreSQL's `postgresql.conf`, or use `'*'` for local
testing:

```conf
listen_addresses = 'localhost,172.18.0.1'
```

Then allow Docker clients in `pg_hba.conf`. Use the Compose network subnet:

```bash
docker network inspect fixfiction_default --format '{{range .IPAM.Config}}{{println .Subnet}}{{end}}'
```

Add a matching `pg_hba.conf` entry:

```conf
host    fixfiction    user    172.18.0.0/16    scram-sha-256
```

Replace `fixfiction`, `user`, and the subnet with your actual database, user,
and Docker subnet. Restart PostgreSQL after changing `listen_addresses`; reload
is enough after changing only `pg_hba.conf`.

Verify connectivity from a container:

```bash
docker run --rm --add-host=host.docker.internal:host-gateway postgres:16-alpine \
  pg_isready -h host.docker.internal -p 5432 -U user -d fixfiction
```

### Bundled PostgreSQL

To run FixFiction with the bundled Compose PostgreSQL service, use the local
Postgres override:

```bash
docker compose -f compose.yml -f compose.local-postgres.yml up --build app postgres
```

Use matching values in `.env`:

```env
DATABASE_URL=postgres://db:pass@postgres:5432/fixfiction
BEARER_TOKEN=your_fimfiction_api_token
POSTGRES_USER=db
POSTGRES_PASSWORD=pass
POSTGRES_DB=fixfiction
```

In this mode, `postgres` is the Compose service hostname. The `POSTGRES_*`
values configure the bundled PostgreSQL container when its data volume is first
created; they are not needed when using an external database.

### Prebuilt Binary

To use a host-built Linux release binary instead of compiling Rust in Docker:

```bash
cargo build --release
docker compose --profile prebuilt up app-prebuilt
```

With bundled PostgreSQL:

```bash
cargo build --release
docker compose -f compose.yml -f compose.local-postgres.yml --profile prebuilt up app-prebuilt postgres
```

The mounted `./target/release/fixfiction` binary must be compatible with the
container runtime OS and libc. Since `Dockerfile.prebuilt` uses Debian, avoid
prebuilts from a different OS/libc or architecture unless you know they match.

### Cleanup

Stop the stack:

```bash
docker compose down
```

Remove the bundled PostgreSQL volume as well:

```bash
docker compose down -v
```
