# FixFiction

FixFiction is a service that fixes embedded content from FimFiction.net.

Maintained by [Silk Rose](https://github.com/SilkRose).


## Quickstart (Docker) (Preferred Method)

Clone this repo:

```bash
git clone https://github.com/SilkRose/FixFiction.git
cd FixFiction
```

Add a .env file with details for the postgres database, and a Fimfiction API bearer token, like so:

```
DATABASE_URL=postgres://user:pass@postgres:5432/fixfiction
BEARER_TOKEN=your_fimfiction_api_token
POSTGRES_USER=user
POSTGRES_PASSWORD=pass
POSTGRES_DB=fixfiction
POSTGRES_DATA_DIR=./postgres-data
FIXFICTION_LOGS_DIR=./logs
```

Then compose up with Docker:

```bash
docker compose up --build app postgres
```

PostgreSQL data is persisted in `POSTGRES_DATA_DIR`, which defaults to `./postgres-data`.
Application logs are persisted in `FIXFICTION_LOGS_DIR`, which defaults to `./logs`.

### Cleanup

Stop the stack:

```bash
docker compose down
```

Remove persisted PostgreSQL data and logs as needed:

```bash
rm -rf postgres-data logs
```

## Quickstart (Cargo)

Clone this repo and build it with Cargo:

```bash
git clone https://github.com/SilkRose/FixFiction.git

cd FixFiction
rustup toolchain install # <-- for windows users
cargo build
```

No env file is required to build the project.

## Docker w/ prebuilt binary

An alternate profile is provided to compose-up this project with a prebuilt binary. The binary is assumed to be present in `./target/release/fixfiction`, and it must be compatible with the container runtime.

```bash
docker compose --profile prebuilt up app-prebuilt postgres
```

## No container

The FixFiction application can be started up as a standalone binary, without a container, though in that case it is left to the user to host a suitable Postgres database.

A config file like as shown below is required:

```
DATABASE_URL=postgres://user:pass@postgres:5432/fixfiction
BEARER_TOKEN=your_fimfiction_api_token
```

Just make sure to configure the DATABASE_URL property so the application can access the database during execution.

## License

This project is licensed under the GNU Affero General Public License 3.0 only.

Copyright (C) 2025-2026  Silk Rose

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, version 3 of the License only.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

The full license is available to read [here](./license.md).
