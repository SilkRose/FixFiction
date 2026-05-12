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
