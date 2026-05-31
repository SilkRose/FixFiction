The [Quickstart (Docker)][qs_docker] method shown in the project README is the preferred method to start up this project. It is reproduced here, as well as other startup methods.

## Quickstart (Docker)

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
```

Then compose up with Docker:

```bash
docker compose up --build app postgres
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

<!-- Links -->
[qs_docker]: README.md#quickstart-docker
