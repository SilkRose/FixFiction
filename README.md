# FixFiction

FixFiction is a service that fixes embedded content from FimFiction.net.

Maintained by [Silk Rose](https://github.com/SilkRose).


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

### Cleanup

Stop the stack:

```bash
docker compose down
```

Remove the bundled PostgreSQL volume as well:

```bash
docker compose down -v
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

Other startup methods are described in [STARTUP.md](STARTUP.md).
