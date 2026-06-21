```
    _    ____ ___   ____  _____ ____ _____
   / \  |  _ \_ _| |  _ \| ____/ ___|_   _|
  / _ \ | |_) | |  | |_) |  _| \___ \ | |
 / ___ \|  __/| |  |  _ <| |___ ___) || |
/_/   \_\_|  |___| |_| \_\_____|____/ |_|

  ____ _____ _   _ _____ ____      _  _____ ___  ____
 / ___| ____| \ | | ____|  _ \    / \|_   _/ _ \|  _ \
| |  _|  _| |  \| |  _| | |_) |  / _ \ | || | | | |_) |
| |_| | |___| |\  | |___|  _ <  / ___ \| || |_| |  _ <
 \____|_____|_| \_|_____|_| \_\/_/   \_\_| \___/|_| \_\
```

<p align="center">
<code>// JACK INTO YOUR DATABASE. GENERATE THE BACKEND. OWN THE GRID.</code>
</p>

---

[![CI](https://github.com/MenkeTechnologies/api-rest-generator/actions/workflows/ci.yml/badge.svg)](https://github.com/MenkeTechnologies/api-rest-generator/actions/workflows/ci.yml)


## `> SYSTEM OVERVIEW`

**API REST Generator** is a zero-config code generation engine that parses raw MySQL, PostgreSQL, SQLite, or Microsoft SQL Server DDL dumps and outputs a fully wired REST backend in your stack of choice — Spring Boot (Java/Kotlin/Groovy) or Loco (Rust/Axum/SeaORM) — entities, controllers, DAOs/repositories or migrations, all of it. You feed it SQL. It feeds you a backend.

No boilerplate. No hand-wiring. Just schema in, API out.

---

## `> CAPABILITIES`

```
[x] Parse MySQL CREATE TABLE statements
[x] Parse PostgreSQL CREATE TABLE + ALTER TABLE statements (pg_dump compatible)
[x] Parse SQLite CREATE TABLE statements (.dump compatible)
[x] Parse MSSQL CREATE TABLE statements (SSMS Generate Scripts compatible)
[x] Auto-detect primary keys, foreign keys, column types
[x] Generate JPA entities with @Id, @ManyToOne, @JoinColumn
[x] Generate SeaORM entities (`#[derive(DeriveEntityModel)]`, `#[sea_orm(primary_key)]`)
[x] Generate REST controllers (GET / POST / PUT / DELETE)
[x] Generate DAO service layer with GenericDao pattern (JVM targets)
[x] Generate Spring Data JPA repositories (JVM targets)
[x] Generate Loco controllers + migrations + module aggregators (Rust target)
[x] Output in Java, Kotlin, Groovy, **or Rust/Loco**
[x] Map MySQL types --> Java/Kotlin/Groovy/Rust types (varchar->String, bigint->Long/i64, datetime->LocalDate/DateTimeWithTimeZone, ...)
[x] Map PostgreSQL types --> Java/Kotlin/Groovy/Rust types (integer, text, boolean, serial, numeric, real, ...)
[x] Map SQLite types --> Java/Kotlin/Groovy/Rust types (INTEGER, TEXT, REAL, NUMERIC, BLOB, ...)
[x] Map MSSQL types --> Java/Kotlin/Groovy/Rust types (nvarchar, uniqueidentifier, money, datetime2, ...)
[x] Java: Lombok-powered (@Data, @AllArgsConstructor, @NoArgsConstructor)
[x] Kotlin: Constructor injection, var properties with defaults, no Lombok
[x] Groovy: @Canonical annotation, field injection, no Lombok
[x] Rust/Loco: SeaORM entities, Axum-flavoured controllers, Loco `create_table` migrations, raw-identifier escaping for Rust keywords (`r#type`)
[x] snake_case tables --> PascalCase entities, camelCase (JVM) / snake_case (Rust) fields
[x] Template-driven codegen with {{placeholder}} substitution (JVM targets)
```

---

## `> TECH STACK`

| Layer          | Tech                          |
|----------------|-------------------------------|
| Language       | Kotlin 2.3.20                 |
| Runtime        | Java 25                       |
| Framework      | Spring Boot 4.0.4             |
| ORM            | Spring Data JPA (Jakarta)     |
| Build          | Gradle 9.4.1 (Kotlin DSL)    |
| Boilerplate    | Lombok (Java), @Canonical (Groovy) |
| Tests          | JUnit 5                       |
| DB Support     | MySQL, PostgreSQL, SQLite, MSSQL |
| Output Languages | Java, Kotlin, Groovy, **Rust/Loco** |

---

## `> QUICKSTART`

### 1. Configure

Edit `src/main/resources/config.properties`:

```properties
target.folder=/absolute/path/to/your/project/src/main/java/
target.package=com/your/package
file.name=mysql_dump.sql
target.language=kotlin
database.type=mysql
```

Set `target.language` to `java`, `kotlin`, `groovy`, or `rust-loco` to control the generated output language. Default is `java`.

When `target.language=rust-loco`, the generator emits a [Loco](https://loco.rs) project tree (SeaORM entities, Axum-style controllers, `loco_rs::schema::create_table` migrations, module aggregators, and an `app_routes.rs` snippet) under `target.folder` — see the **RUST/LOCO TARGET** section below.

Set `database.type` to `mysql`, `postgresql`, `sqlite`, or `mssql` to match your dump file format. Default is `mysql`.

### 2. Drop your SQL

Place your DDL dump file in `src/main/resources/`.

For MySQL, use `mysqldump --no-data` to generate the dump file.
For PostgreSQL, use `pg_dump --schema-only` to generate the dump file.
For SQLite, use `sqlite3 database.db .dump` to generate the dump file.
For MSSQL, use SSMS "Generate Scripts" or `sqlcmd` to export the schema.

### 3. Execute

```bash
./gradlew run
```

Or run `Main.kt` from your IDE. Watch the grid light up.

`./gradlew run` covers the JVM output languages (`java` / `kotlin` / `groovy`). For `target.language=rust-loco`, run the Rust generator binary instead — it reads the same `config.properties`:

```bash
cargo run --bin api-rest-generator
```

---

## `> OUTPUT MATRIX`

File extensions depend on `target.language`: `.java`, `.kt`, `.groovy`, or `.rs`.

The directory tree below applies to the JVM targets (`java`/`kotlin`/`groovy`). For `rust-loco`, see the **RUST/LOCO TARGET** section below for the Loco-flavoured layout.

```
{target.folder}/{target.package}/
 |-- entity/
 |    |-- User.{ext}           @Entity @Table @Column @Id @ManyToOne
 |    |-- Module.{ext}
 |    \-- ...
 |-- rest/
 |    |-- UserResource.{ext}   @RestController with full CRUD
 |    |-- ModuleResource.{ext}
 |    \-- ...
 |-- dao/
 |    |-- UserDao.{ext}        @Service implementing GenericDao<T>
 |    |-- GenericDao.{ext}     Generic interface for all DAOs
 |    \-- ...
 |-- repository/
 |    |-- UserRepository.{ext} extends JpaRepository<T, Long>
 |    \-- ...
 \-- utils/
      \-- GlobalConstants.{ext}
```

### Language Differences

| Feature              | Java                          | Kotlin                         | Groovy                         | Rust/Loco                       |
|----------------------|-------------------------------|--------------------------------|--------------------------------|---------------------------------|
| Entity boilerplate   | Lombok (@Data, @NoArgsConstructor) | var properties with defaults | @Canonical                     | `#[derive(DeriveEntityModel)]` (SeaORM) |
| DI style             | @Autowired field injection    | Constructor injection          | @Autowired field injection     | `State<AppContext>` extractor   |
| Inheritance syntax   | `extends` / `implements`      | `:` (colon)                    | `extends` / `implements`       | trait `impl`                    |
| File extension       | `.java`                       | `.kt`                          | `.groovy`                      | `.rs`                           |
| Semicolons           | Yes                           | No                             | No                             | Yes                             |
| int type             | Integer                       | Int                            | Integer                        | i32                             |
| bigint type          | Long                          | Long                           | Long                           | i64                             |
| bit/boolean type     | String                        | Boolean                        | String                         | bool                            |
| FK type              | Integer                       | Int                            | Integer                        | i32                             |
| datetime / timestamp | LocalDate / LocalDateTime     | LocalDate / LocalDateTime      | LocalDate / LocalDateTime      | DateTimeWithTimeZone (SeaORM)   |

---

## `> RUST/LOCO TARGET`

`target.language=rust-loco` (alias `loco`) skips the JVM template tree entirely and emits a Loco-flavoured Rust project tree under `target.folder` — drop it into a [`loco new`](https://loco.rs) skeleton and you have a working REST API in two steps.

### Layout

```
{target.folder}/
 |-- src/
 |    |-- models/
 |    |    |-- mod.rs                            # aggregator
 |    |    |-- _entities/
 |    |    |    |-- mod.rs
 |    |    |    |-- prelude.rs
 |    |    |    \-- {snake_table}.rs              # SeaORM Model + Relation
 |    |    \-- {snake_table}.rs                   # ActiveModelBehavior + impls
 |    |-- controllers/
 |    |    |-- mod.rs
 |    |    \-- {snake_table}.rs                   # full CRUD (list/get/add/update/remove)
 |    \-- app_routes.rs                            # snippet for your Hooks::routes
 \-- migration/
      \-- src/
           |-- lib.rs                              # Migrator with every migration
           \-- m{datetime}_{snake_table}.rs        # loco_rs::schema::create_table
```

### Type mapping (Rust/Loco target)

| SQL type                                   | Rust (SeaORM)            | Loco `ColType`           |
|--------------------------------------------|--------------------------|--------------------------|
| `bigint`, `int8`, `bigserial`              | `i64`                    | `BigInteger`             |
| `int`, `integer`, `int4`, `serial`         | `i32`                    | `Integer`                |
| `smallint`, `int2`                         | `i16`                    | `SmallInteger`           |
| `tinyint`                                  | `i8`                     | `SmallInteger`           |
| `varchar(n)`, `char`, `text`, `nvarchar`   | `String`                 | `String`                 |
| `bool`, `boolean`, `bit`                   | `bool`                   | `Boolean`                |
| `float`, `real`                            | `f32`                    | `Float`                  |
| `double`, `double precision`               | `f64`                    | `Double`                 |
| `decimal`, `numeric`, `money`              | `f64`                    | `Double`                 |
| `date`                                     | `Date`                   | `Date`                   |
| `time`                                     | `Time`                   | `Time`                   |
| `datetime`, `datetime2`, `timestamp(tz)`   | `DateTimeWithTimeZone`   | `TimestampWithTimeZone`  |
| `blob`, `bytea`, `binary`, `varbinary`     | `Vec<u8>`                | `Blob`                   |
| primary key column (any)                   | `i32`                    | `PkAuto`                 |

Reserved Rust keywords (e.g. column named `type`, `match`, `move`) are emitted as raw identifiers (`r#type`) so the generated structs compile without losing the original column name in JSON serialisation.

### Using the `loco-gen` CLI (recommended)

The repo ships a dedicated Rust CLI binary, `loco-gen`, that scaffolds a Loco project AND emits all entities, controllers, and migrations from a DDL dump in one shot — no config file editing required.

```bash
# build the CLI once
cargo install --path . --bin loco-gen      # or: cargo build --release && cp target/release/loco-gen ~/bin/

# prerequisite: the upstream Loco CLI
cargo install loco

# one-shot: scaffold + generate + wire routes + run migrations
loco-gen new \
  --ddl  ./mysql_dump.sql \
  --name myapi \
  --out  . \
  --db   mysql \
  --loco-db sqlite \
  --wire \
  --migrate

cd myapi && cargo loco start    # CRUD endpoints live at /api/{table_plural}/{id}
```

Subcommands:

| Subcommand | What it does |
|------------|--------------|
| `loco-gen new`      | Runs `loco new -n <name> --db <loco-db>`, parses the DDL, writes all generated files into the new project, and (with `--wire`) patches `src/app.rs` to register every controller's routes. Optional `--migrate` runs `cargo loco db migrate`. |
| `loco-gen generate` | Emits the generated tree into an **existing** Loco project (no scaffold). `--wire` still patches `src/app.rs`. Useful for re-running after schema changes. |

Flags:

| Flag             | Default | Meaning |
|------------------|---------|---------|
| `--ddl FILE`     | _req'd_ | Path to your SQL dump. |
| `--name NAME`    | _req'd (new)_ | Project / crate name. |
| `--out DIR`      | `.`     | Where to create / find the project. |
| `--db DIALECT`   | `mysql` | DDL dialect: `mysql`, `postgresql`, `sqlite`, `mssql`. |
| `--loco-db DB`   | `sqlite`| Backing DB for the new Loco project: `sqlite`, `postgres`. |
| `--wire`         | off     | Patch `src/app.rs` to register every generated controller's routes (idempotent — re-running replaces the same `// BEGIN loco-gen routes` block). |
| `--migrate`      | off     | Run `cargo loco db migrate` after generation. |
| `--loco-bin`     | `loco`  | Path to the upstream loco CLI binary. |

The route-merge is **idempotent and surgical**: existing modules like `controllers::home` (shipped with the Loco scaffold) are preserved. Re-running `loco-gen generate --wire` after a schema change does not duplicate routes — the `// BEGIN loco-gen routes ... // END loco-gen routes` block is rewritten in place.

### Wiring manually (advanced)

If you prefer the original config-file workflow (run the JVM-style generator binary instead of the CLI):

```bash
loco new -n myapi --db sqlite --bg none --assets none
cd myapi

# point the generator at this project
cat > /path/to/api-rest-generator/src/main/resources/config.properties <<EOF
target.folder=$(pwd)
target.package=
file.name=mysql_dump.sql
target.language=rust-loco
database.type=mysql
EOF

# (back in the generator repo)
cargo run --release --bin api-rest-generator

# back in the loco project
cargo loco db migrate
cargo loco start
```

You'll then need to add the generated routes to your `Hooks::routes` impl in `src/app.rs` by hand:

```rust
fn routes(_ctx: &AppContext) -> AppRoutes {
    AppRoutes::with_default_routes()
        .add_route(controllers::home::routes())
        .add_route(controllers::users::routes())
        .add_route(controllers::orders::routes())
        // ... one per generated entity
}
```

(The generator drops a ready-made `register_generated_routes(routes)` helper in `src/app_routes.rs` you can call instead. The `loco-gen` CLI does this automatically when `--wire` is passed.)

### Generated REST shape per table

```
GET     /api/{snake_table_plural}             # list all
POST    /api/{snake_table_plural}             # create (JSON body, PK omitted)
GET     /api/{snake_table_plural}/{id}        # fetch one
PUT     /api/{snake_table_plural}/{id}        # update
DELETE  /api/{snake_table_plural}/{id}        # remove
```

### Not yet implemented

- FK relations are surfaced as plain `i32` columns; the SeaORM `Relation` enum is generated empty. Add `#[sea_orm(belongs_to = ...)]` arms by hand if you need eager-loading.
- No unique / nullable / default-value detection — every column is non-null in the migration. Tweak the generated `ColType::X` → `XNull` / `XUniq` / `XWithDefault(...)` as needed.
- No pagination, search, or filter endpoints — only the five basic CRUD verbs above.
- Composite primary keys are collapsed to a synthetic `id` `PkAuto` column (Loco/SeaORM require a single PK). The original PK columns remain as regular fields — enforce uniqueness via a partial index in a follow-up migration if needed.

### Verified sample DDLs

`loco-gen` is exercised against four real-world open-source schemas living under [`samples/`](samples/). Each one round-trips through scaffold → generate → `cargo build` → `cargo loco db migrate` → live CRUD on `localhost:5150`.

| Sample            | Source                                                                 | `--db`        | Tables | Notes                                                       |
|-------------------|------------------------------------------------------------------------|---------------|--------|-------------------------------------------------------------|
| Sakila            | [jOOQ/sakila](https://github.com/jOOQ/sakila)                          | `mysql`       | 16     | DVD rental store; exercises triggers/views/functions filtering. |
| Chinook           | [lerocha/chinook-database](https://github.com/lerocha/chinook-database)| `sqlite`      | 11     | Digital media store; `[bracketed]` identifiers + composite PKs. |
| Pagila            | [devrimgunduz/pagila](https://github.com/devrimgunduz/pagila)          | `postgresql`  | 22     | Postgres Sakila port; `ALTER TABLE ADD PRIMARY KEY` style.   |
| Northwind         | [microsoft/sql-server-samples](https://github.com/microsoft/sql-server-samples) | `mssql`       | 13     | Classic MSSQL; double-quoted identifiers + `Order Details` space-in-name. |

Reproduce locally:

```bash
cargo build --release
for s in sakila:mysql chinook:sqlite pagila:postgresql northwind:mssql; do
    name=${s%%:*}; db=${s##*:}
    ./target/release/loco-gen new --ddl samples/${name}-*.sql \
        --name ${name}_api --out /tmp/samples --db $db --loco-db sqlite --wire
    ( cd /tmp/samples/${name}_api && cargo build && cargo loco db migrate )
done
```

The integration test [`tests/sample_ddls.rs`](tests/sample_ddls.rs) pins the parse output (entity counts, PK presence, no leaked quotes/brackets/tabs) so regressions in the normalizer/parser are caught in CI without needing the Loco toolchain installed.

---

## `> GENERATED ENDPOINTS` (JVM targets)

Every entity gets a full CRUD interface wired to `/api/v1` (Spring Boot `java`/`kotlin`/`groovy` targets):

```
GET     /api/v1/{entity}        // pull all records
GET     /api/v1/{entity}/{id}   // pull one by id
POST    /api/v1/{entity}        // create
PUT     /api/v1/{entity}        // update
DELETE  /api/v1/{entity}/{id}   // flatline one
DELETE  /api/v1/{entity}        // flatline all
```

For the `rust-loco` target the generated controller wires only the 5 id-scoped CRUD routes — `GET /`, `POST /`, `GET /{id}`, `PUT /{id}`, `DELETE /{id}` (no collection-level `PUT` or `DELETE`) — see the **RUST/LOCO TARGET** section.

---

## `> TYPE MAPPING`

### MySQL

```
MySQL              -->   Java/Kotlin/Groovy
-----------------------------------------
int, tinyint       -->   Integer
bigint             -->   Long
varchar            -->   String
float              -->   Float
double             -->   Double
datetime           -->   LocalDate
timestamp          -->   LocalDateTime
time               -->   LocalTime
bit                -->   String
PRIMARY KEY        -->   Long (default)
FOREIGN KEY        -->   Integer (default)
```

### PostgreSQL

```
PostgreSQL                    -->   Java/Kotlin/Groovy
-------------------------------------------------
integer, smallint, serial     -->   Integer
bigint, bigserial             -->   Long
character varying, varchar    -->   String
text                          -->   String
real                          -->   Float
double precision              -->   Double
numeric                       -->   Double
boolean, bool                 -->   String (Java/Groovy) / Boolean (Kotlin)
date                          -->   LocalDate
timestamp (with/without tz)   -->   LocalDateTime
time (with/without tz)        -->   LocalTime
PRIMARY KEY                   -->   Long (default)
FOREIGN KEY                   -->   Integer (default)
```

### SQLite

```
SQLite               -->   Java/Kotlin/Groovy
-----------------------------------------
INTEGER              -->   Integer
TEXT                 -->   String
VARCHAR(n)           -->   String
REAL                 -->   Float
NUMERIC              -->   Double
BLOB                 -->   String
INTEGER PRIMARY KEY  -->   Long (inline PK detected)
FOREIGN KEY          -->   Integer (default)
```

### MSSQL (SQL Server)

```
MSSQL                         -->   Java/Kotlin/Groovy
-------------------------------------------------
int, tinyint, smallint        -->   Integer
bigint                        -->   Long
nvarchar, nchar, varchar      -->   String
ntext, text, char             -->   String
uniqueidentifier              -->   String
image, xml                    -->   String
float                         -->   Float
real                          -->   Float
money, smallmoney             -->   Double
decimal, numeric              -->   Double
bit                           -->   String (Java/Groovy) / Boolean (Kotlin)
date                          -->   LocalDate
datetime                      -->   LocalDate
datetime2, datetimeoffset     -->   LocalDateTime
smalldatetime                 -->   LocalDateTime
time                          -->   LocalTime
PRIMARY KEY                   -->   Long (default)
FOREIGN KEY                   -->   Integer (default)
```

---

## `> ARCHITECTURE`

```
 mysql_dump.sql
    |
    v
 [ TOKENIZER ] -- strips comments (#, --), splits tokens     (Util.getWords)
    |
    v
 [ NORMALIZER ] -- combines multi-word types, strips          (Util.normalizePostgresqlWords)
    |               punctuation, brackets, filters noise       (Util.normalizeSqliteWords)
    |               (database-specific normalizer per type)    (Util.normalizeMssqlWords)
    v
 [ PARSER ] -- detects CREATE TABLE, columns, keys           (Util.parseWords)
    |           supports ALTER TABLE for PG primary/foreign keys
    |
    v
 [ TEMPLATE ENGINE ] -- fills {{entityName}}, {{fields}}, .. (Templates.kt)
    |
    v
 [ FILE WRITER ] -- outputs layered Spring Boot project      (Main.kt)
```

---

## `> PROJECT STRUCTURE`

```
src/main/kotlin/com/jakobmenke/bootrestgenerator/
 |-- Main.kt                          Entry point & file writer
 |-- dto/
 |    |-- Entity.kt                   Entity data model
 |    \-- ColumnToField.kt            Column-to-field mapping
 |-- templates/
 |    \-- Templates.kt                Template engine & replacements
 \-- utils/
      |-- Configuration.kt            Config reader
      |-- Util.kt                     Parser, type mapper, key detector
      |-- EntityToRESTConstants.kt    Regex patterns & constants
      \-- Globals.kt                  Global state holder

src/main/resources/templates/
 |-- *.tmpl                              Java templates (default)
 |-- kotlin/*.tmpl                       Kotlin templates
 \-- groovy/*.tmpl                       Groovy templates

src/                                     Rust crate (port of the Kotlin generator + Loco target)
 |-- lib.rs                              Crate root
 |-- config.rs                           config.properties reader
 |-- constants.rs                        Regex patterns & constants
 |-- entity.rs                           Entity / column models
 |-- globals.rs                          Global state holder
 |-- normalize.rs                        PG/SQLite/MSSQL normalizers
 |-- parser.rs                           Tokenizer & CREATE/ALTER TABLE parser
 |-- templates.rs                        JVM template engine
 |-- loco.rs                             Loco/SeaORM emitter
 \-- bin/
      |-- main.rs                        api-rest-generator binary (config-driven)
      \-- loco_gen.rs                    loco-gen CLI (clap)
```

---

## `> RUNNING TESTS`

```bash
./gradlew test    # JVM suite (parser, type mappers, templates, pipelines)
cargo test        # Rust suite (parser/normalizer parity, Loco emitter, sample DDLs)
```

### Test Suite Overview

Unit, template, and integration tests cover the parser, type mappers, template engine, and full pipeline across all supported database / language combinations.

| Category | Test Class | What It Covers |
|----------|-----------|----------------|
| **Unit** | `MainTest` | PK/FK identification, string capitalization, camelCase conversion |
| **Unit** | `UtilTest` | `firstLetterToCaps`, `camelName`, `getId`, `getWords`, `parseWords` |
| **Unit** | `KotlinUtilTest` | Kotlin type mapping (`Int` vs `Integer`, `Boolean` vs `String`), PK/FK types |
| **Unit** | `GroovyUtilTest` | Groovy type mapping (matches Java types), cross-database type handling |
| **Unit** | `ColumnToFieldTest` | `ColumnToField` data class constructors, equality, copy, mutability |
| **Unit** | `EntityTest` | `Entity` data class constructors, properties, column management |
| **Unit** | `EntityToRESTConstantsTest` | Regex patterns for all SQL types, PK/FK parsing, MySQL/PG/SQLite/MSSQL type regexes |
| **Unit** | `EdgeCaseParsingTest` | Empty input, special characters, self-referencing FKs, large names, multi-table edge cases |
| **Unit** | `ConfigurationTest` | Config file loading, language-specific default folders, fallback behavior |
| **Unit** | `KotlinConfigurationTest` | Kotlin language property, global flags, case-insensitive matching |
| **Unit** | `GroovyConfigurationTest` | Groovy language property, global flags, mutual exclusion with Kotlin |
| **Template** | `TemplatesTest` | Java templates: entities, DAOs, repositories, REST resources, package declarations |
| **Template** | `KotlinTemplatesTest` | Kotlin templates: colon inheritance, constructor injection, `val`/`var`, no Lombok |
| **Template** | `GroovyTemplatesTest` | Groovy templates: `@Canonical`, `@Autowired`, `implements`, no Lombok |
| **Template** | `KotlinEntityFieldDefaultsTest` | Kotlin field defaults (`""`, `0`, `0L`, `false`), nullable types, Java mode guard |
| **Template** | `GroovyEntityFieldsTest` | Groovy field declarations without defaults or nullable types, language comparison |
| **Template** | `RestRepositoryTemplateTest` | `@RepositoryRestResource` annotation, Spring Data REST imports |
| **Integration** | `FullPipelineTest` | End-to-end MySQL: parse 18 entities, validate columns/keys/types, write all files |
| **Integration** | `PostgresqlPipelineTest` | PostgreSQL pipeline: `ALTER TABLE` constraints, PG type mappings, cross-language |
| **Integration** | `SqlitePipelineTest` | SQLite pipeline: inline PKs, `.dump` noise filtering (`INSERT`, `PRAGMA`, `BEGIN`) |
| **Integration** | `MssqlPipelineTest` | MSSQL pipeline: `[bracket]` stripping, `SET`/`GO` filtering, MSSQL type mappings |
| **Integration** | `KotlinPipelineTest` | Kotlin pipeline: type mapping, `.kt` file generation, template content |
| **Integration** | `GroovyPipelineTest` | Groovy pipeline: type mapping, `.groovy` file generation, `@Canonical` templates |
| **Integration** | `CustomSqlParsingTest` | Complex SQL: mixed comments, multi-FK tables, every data type, end-to-end template gen |
| **Integration** | `EmptyEntityListTest` | Empty entity lists, single entity generation, deep package paths |
| **Integration** | `KotlinContentValidationTest` | Generated Kotlin files: no Java artifacts, no placeholders, correct idioms |
| **Integration** | `GroovyContentValidationTest` | Generated Groovy files: no Java/Kotlin artifacts, pure Groovy syntax |
| **Integration** | `CrossDatabaseLanguageTest` | All db/language combos (databases x languages), parameterized matrix |

### Coverage Matrix

```
              MySQL   PostgreSQL   SQLite   MSSQL
           +--------+------------+--------+--------+
Java       |   ✓    |     ✓      |   ✓    |   ✓    |
Kotlin     |   ✓    |     ✓      |   ✓    |   ✓    |
Groovy     |   ✓    |     ✓      |   ✓    |   ✓    |
Rust/Loco  |   ✓    |     ✓      |   ✓    |   ✓    |
           +--------+------------+--------+--------+
```

---

<p align="center">
<code>// CREATED BY MENKETECHNOLOGIES</code>
</p>
