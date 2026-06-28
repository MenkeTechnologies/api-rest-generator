//! Rust/Loco code generator — emits SeaORM entities, Loco controllers,
//! and Loco migrations from parsed [`Entity`] structs.
//!
//! Output layout (rooted at `target.folder`):
//!
//! ```text
//! <target.folder>/
//! ├── src/
//! │   ├── models/
//! │   │   ├── mod.rs
//! │   │   ├── _entities/
//! │   │   │   ├── mod.rs
//! │   │   │   ├── prelude.rs
//! │   │   │   └── <snake_table>.rs           # SeaORM Model + Relation
//! │   │   └── <snake_table>.rs                # ActiveModelBehavior + impls
//! │   ├── controllers/
//! │   │   ├── mod.rs
//! │   │   └── <snake_table>.rs                # CRUD endpoints
//! │   └── app_routes.rs                        # snippet to drop into app.rs
//! └── migration/
//!     └── src/
//!         ├── lib.rs
//!         └── m<datetime>_<snake_table>.rs    # Loco schema::create_table
//! ```

use crate::entity::{ColumnToField, Entity};

/// snake_case the table name (e.g. `ALPHABET_CHARACTER` → `alphabet_characters`,
/// `LearningCollection` → `learning_collections`). Loco pluralises so we
/// preserve any trailing `s`; otherwise add one.
pub fn snake_module_for(table: &str) -> String {
    let snake = to_snake_case(table);
    if snake.ends_with('s') {
        snake
    } else {
        format!("{snake}s")
    }
}

/// snake_case the table name with NO pluralisation — used for the entity
/// `#[sea_orm(table_name = ...)]` attribute and matching migration name.
pub fn snake_table_for(table: &str) -> String {
    snake_module_for(table)
}

fn to_snake_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    let mut prev_lower = false;
    for c in s.chars() {
        if c == '_' || c == ' ' {
            if !out.ends_with('_') {
                out.push('_');
            }
            prev_lower = false;
        } else if c.is_uppercase() {
            if prev_lower && !out.ends_with('_') {
                out.push('_');
            }
            for lc in c.to_lowercase() {
                out.push(lc);
            }
            prev_lower = false;
        } else {
            out.push(c);
            prev_lower = true;
        }
    }
    out.trim_matches('_').to_string()
}

fn ascii_lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

/// Strip a leading `(...)` length qualifier and lowercase the type token.
fn base_type(raw: &str) -> String {
    let lower = ascii_lower(raw);
    match lower.find('(') {
        Some(i) => lower[..i].to_string(),
        None => lower,
    }
}

/// Returns `true` if `entity` already has a column tagged as the
/// primary key (`@Id`). Used by [`needs_synthetic_pk`].
fn has_pk(entity: &Entity) -> bool {
    entity.columns.iter().any(|c| {
        c.database_id_type
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case("@Id"))
            .unwrap_or(false)
    })
}

/// Loco requires every entity to have a primary key (see
/// <https://github.com/SeaQL/sea-orm/issues/485>). When the source DDL
/// declares a composite PK via `PRIMARY KEY (a, b)` or no PK at all
/// (common for join tables), we synthesize an `id` `PkAuto` column so
/// the generated entity, migration, and controller all compile. The
/// original "PK" columns become normal `i32` foreign-key-like columns.
pub fn needs_synthetic_pk(entity: &Entity) -> bool {
    !has_pk(entity)
}

/// Returns the synthetic PK column to prepend when [`needs_synthetic_pk`]
/// is true.
fn synthetic_pk_column() -> ColumnToField {
    ColumnToField {
        database_id_type: Some("@Id".to_string()),
        database_column_name: Some("id".to_string()),
        camel_case_field_name: Some("id".to_string()),
        database_type: None,
        java_type: Some("Integer".to_string()),
    }
}

/// Returns the effective column list for rendering: prepends a synthetic
/// `id` PkAuto column when the source DDL didn't declare one. If a
/// non-PK column already named `id` exists (case-insensitive), promote
/// it to PK in place rather than creating a duplicate.
fn effective_columns(entity: &Entity) -> Vec<ColumnToField> {
    if has_pk(entity) {
        return entity.columns.clone();
    }

    let id_idx = entity.columns.iter().position(|c| {
        c.database_column_name
            .as_deref()
            .map(|n| n.eq_ignore_ascii_case("id"))
            .unwrap_or(false)
    });

    if let Some(idx) = id_idx {
        let mut cols = entity.columns.clone();
        cols[idx].database_id_type = Some("@Id".to_string());
        cols[idx].database_type = None;
        cols[idx].java_type = Some("Integer".to_string());
        cols[idx].camel_case_field_name = Some("id".to_string());
        return cols;
    }

    let mut cols = Vec::with_capacity(entity.columns.len() + 1);
    cols.push(synthetic_pk_column());
    cols.extend(entity.columns.iter().cloned());
    cols
}

/// Per-column SeaORM Rust type derived from the raw DDL `database_type`
/// plus the `database_id_type` (PK/FK hints).
pub fn rust_type_for(col: &ColumnToField) -> &'static str {
    if let Some(idt) = &col.database_id_type {
        if idt.eq_ignore_ascii_case("@Id") {
            return "i32"; // matches Loco's `ColType::PkAuto` (INTEGER)
        }
        if idt.eq_ignore_ascii_case("@ManyToOne") {
            return "i32";
        }
    }
    let Some(raw) = col.database_type.as_deref() else {
        return "String";
    };
    match base_type(raw).as_str() {
        "bigint" | "int8" | "long" | "bigserial" => "i64",
        "int" | "integer" | "int4" | "serial" | "mediumint" => "i32",
        "smallint" | "int2" | "smallserial" => "i16",
        "tinyint" => "i8",
        "bool" | "boolean" | "bit" => "bool",
        "float" | "real" | "float4" => "f32",
        "double" | "double precision" | "float8" => "f64",
        "decimal" | "numeric" | "money" | "smallmoney" => "f64",
        "date" => "Date",
        "time" => "Time",
        "datetime" | "datetime2" | "timestamp" | "timestamptz" => "DateTimeWithTimeZone",
        "blob" | "bytea" | "binary" | "varbinary" | "image" => "Vec<u8>",
        _ => "String",
    }
}

/// Loco `ColType::*` variant for a column.
pub fn loco_col_type_for(col: &ColumnToField) -> &'static str {
    if col
        .database_id_type
        .as_deref()
        .map(|s| s.eq_ignore_ascii_case("@Id"))
        .unwrap_or(false)
    {
        return "PkAuto";
    }
    match rust_type_for(col) {
        "i64" => "BigInteger",
        "i32" => "Integer",
        "i16" => "SmallInteger",
        "i8" => "SmallInteger",
        "bool" => "Boolean",
        "f32" => "Float",
        "f64" => "Double",
        "Date" => "Date",
        "Time" => "Time",
        "DateTimeWithTimeZone" => "TimestampWithTimeZone",
        "Vec<u8>" => "Blob",
        _ => "String",
    }
}

/// Field name (snake_case) the SeaORM model uses for a column. Reserved
/// Rust keywords are escaped with the raw-identifier prefix `r#` so the
/// generated structs compile (and `serde` still serialises them with
/// their original name).
fn field_name(col: &ColumnToField) -> String {
    let raw = if let Some(name) = &col.database_column_name {
        to_snake_case(name)
    } else if let Some(name) = &col.camel_case_field_name {
        to_snake_case(name)
    } else {
        "_unknown".to_string()
    };
    escape_rust_keyword(&raw)
}

/// Database column name for emission into SQL string literals (migration
/// `(\"col\", ColType::*)` rows). Snake-cases the DDL-source name (so
/// `ACTIVITY_ID` → `activity_id` matches the Rust field name) but does NOT
/// route through `escape_rust_keyword` — that's only correct for Rust
/// identifier positions. Pre-fix this used `field_name(col)` which produced
/// `r#type` / `type_field` for reserved-word columns and corrupted the
/// migration's CREATE TABLE statement.
fn raw_db_column_name(col: &ColumnToField) -> String {
    if let Some(name) = &col.database_column_name {
        to_snake_case(name)
    } else if let Some(name) = &col.camel_case_field_name {
        to_snake_case(name)
    } else {
        "_unknown".to_string()
    }
}

/// Wrap reserved Rust keywords in `r#` so they can be used as identifiers.
///
/// `self`, `Self`, `crate`, `super` are explicitly disallowed as raw
/// identifiers by the Rust language reference, so columns literally named
/// one of those four must be suffixed (`_field`) instead of `r#`-wrapped.
/// `union` and `dyn` are also context-sensitive — they're contextual keywords
/// and don't need escaping in field-name position, but the escape is
/// harmless so we keep them.
fn escape_rust_keyword(s: &str) -> String {
    // Reserved + can be raw-identified: wrap in `r#`.
    const RESERVED_RAW_OK: &[&str] = &[
        "as", "break", "const", "continue", "else", "enum", "extern", "false", "fn", "for", "if",
        "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
        "static", "struct", "trait", "true", "type", "unsafe", "use", "where", "while", "async",
        "await", "dyn", "abstract", "become", "box", "do", "final", "macro", "override", "priv",
        "typeof", "unsized", "virtual", "yield", "try", "union",
    ];
    // Reserved + NOT allowed as raw idents per the Rust reference: suffix
    // with `_field` instead of `r#`. A column literally named `self` etc.
    // would otherwise generate code that fails to compile (`pub r#self: …`
    // — rejected by the parser).
    const RESERVED_NEEDS_SUFFIX: &[&str] = &["self", "Self", "crate", "super"];
    if RESERVED_RAW_OK.contains(&s) {
        format!("r#{s}")
    } else if RESERVED_NEEDS_SUFFIX.contains(&s) {
        format!("{s}_field")
    } else {
        s.to_string()
    }
}

/// Render `src/models/_entities/<snake>.rs` — the generated SeaORM entity
/// (`Model` struct + `Relation` enum + `ActiveModelBehavior`-free shell).
pub fn render_entity(entity: &Entity) -> String {
    let table = snake_table_for(&entity.table_name);
    let mut s = String::new();
    s.push_str("//! `SeaORM` Entity, generated by api-rest-generator (rust-loco target).\n\n");
    s.push_str("use sea_orm::entity::prelude::*;\n");
    s.push_str("use serde::{Deserialize, Serialize};\n\n");
    s.push_str("#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]\n");
    s.push_str(&format!("#[sea_orm(table_name = \"{table}\")]\n"));
    s.push_str("pub struct Model {\n");
    for col in effective_columns(entity).iter() {
        let name = field_name(col);
        let ty = rust_type_for(col);
        if col
            .database_id_type
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case("@Id"))
            .unwrap_or(false)
        {
            s.push_str("    #[sea_orm(primary_key)]\n");
        }
        s.push_str(&format!("    pub {name}: {ty},\n"));
    }
    s.push_str("}\n\n");
    s.push_str("#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]\n");
    s.push_str("pub enum Relation {}\n");
    s
}

/// Render `src/models/<snake>.rs` — re-exports + `ActiveModelBehavior` impl.
pub fn render_model(entity: &Entity) -> String {
    let snake = snake_table_for(&entity.table_name);
    let pascal = pascal_case(&snake);
    let mut s = String::new();
    s.push_str("use sea_orm::entity::prelude::*;\n");
    s.push_str(&format!(
        "pub use super::_entities::{snake}::{{ActiveModel, Model, Entity}};\n"
    ));
    s.push_str(&format!("pub type {pascal} = Entity;\n\n"));
    s.push_str("#[async_trait::async_trait]\n");
    s.push_str("impl ActiveModelBehavior for ActiveModel {}\n\n");
    s.push_str("impl Model {}\n");
    s.push_str("impl ActiveModel {}\n");
    s.push_str("impl Entity {}\n");
    s
}

/// Render `src/controllers/<snake>.rs` — CRUD REST endpoints.
pub fn render_controller(entity: &Entity) -> String {
    let snake = snake_table_for(&entity.table_name);
    let pascal_entity = pascal_case(&snake);
    let path_prefix = format!("api/{snake}");

    let mut params_fields = String::new();
    let mut params_update = String::new();
    let mut new_active = String::from("        let item = ActiveModel {\n");
    for col in effective_columns(entity).iter() {
        let name = field_name(col);
        let ty = rust_type_for(col);
        let is_pk = col
            .database_id_type
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case("@Id"))
            .unwrap_or(false);
        if is_pk {
            continue;
        }
        params_fields.push_str(&format!("    pub {name}: {ty},\n"));
        params_update.push_str(&format!(
            "        item.{name} = sea_orm::ActiveValue::set(self.{name}.clone());\n"
        ));
        new_active.push_str(&format!(
            "            {name}: sea_orm::ActiveValue::set(params.{name}.clone()),\n"
        ));
    }
    new_active.push_str("            ..Default::default()\n        };\n");

    format!(
        r#"#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]

use loco_rs::prelude::*;
#[allow(unused_imports)]
use sea_orm::prelude::{{Date, DateTimeWithTimeZone, Time}};
use sea_orm::{{ActiveModelTrait, EntityTrait, IntoActiveModel, ModelTrait}};
use serde::{{Deserialize, Serialize}};

use crate::models::_entities::{snake}::{{ActiveModel, Entity as {pascal_entity}, Model}};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Params {{
{params_fields}}}

impl Params {{
    fn update(&self, item: &mut ActiveModel) {{
{params_update}    }}
}}

async fn load_item(ctx: &AppContext, id: i32) -> Result<Model> {{
    {pascal_entity}::find_by_id(id)
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::NotFound)
}}

#[debug_handler]
pub async fn list(State(ctx): State<AppContext>) -> Result<Response> {{
    format::json({pascal_entity}::find().all(&ctx.db).await?)
}}

#[debug_handler]
pub async fn add(State(ctx): State<AppContext>, Json(params): Json<Params>) -> Result<Response> {{
{new_active}        let item = item.insert(&ctx.db).await?;
    format::json(item)
}}

#[debug_handler]
pub async fn update(
    Path(id): Path<i32>,
    State(ctx): State<AppContext>,
    Json(params): Json<Params>,
) -> Result<Response> {{
    let item = load_item(&ctx, id).await?;
    let mut item = item.into_active_model();
    params.update(&mut item);
    let item = item.update(&ctx.db).await?;
    format::json(item)
}}

#[debug_handler]
pub async fn remove(Path(id): Path<i32>, State(ctx): State<AppContext>) -> Result<Response> {{
    load_item(&ctx, id).await?.delete(&ctx.db).await?;
    format::empty()
}}

#[debug_handler]
pub async fn get_one(Path(id): Path<i32>, State(ctx): State<AppContext>) -> Result<Response> {{
    format::json(load_item(&ctx, id).await?)
}}

pub fn routes() -> Routes {{
    Routes::new()
        .prefix("{path_prefix}")
        .add("/", get(list))
        .add("/", post(add))
        .add("/{{id}}", get(get_one))
        .add("/{{id}}", delete(remove))
        .add("/{{id}}", put(update))
}}
"#,
    )
}

/// Render `migration/src/m<datetime>_<snake>.rs`.
pub fn render_migration(entity: &Entity) -> String {
    let snake = snake_table_for(&entity.table_name);
    let cols = {
        let mut s = String::new();
        for col in effective_columns(entity).iter() {
            // Pre-fix this used `field_name(col)` which routes through
            // `escape_rust_keyword` and produces `r#type` / `type_field` for
            // reserved-word column names. That string then landed in a SQL
            // column-name STRING LITERAL — the DB column was literally
            // created with the escaped name, and SeaORM later couldn't match
            // the Rust-side `type` field to the DB-side `r#type` column.
            // The migration must emit the RAW database column name (with the
            // original case if the source DDL preserved it). Rust-identifier
            // escaping is correct only for Rust-identifier positions.
            let name = raw_db_column_name(col);
            let ct = loco_col_type_for(col);
            s.push_str(&format!("            (\"{name}\", ColType::{ct}),\n"));
        }
        s
    };
    format!(
        r#"use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {{
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {{
        create_table(m, "{snake}",
            &[
{cols}            ],
            &[],
        ).await
    }}

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {{
        drop_table(m, "{snake}").await
    }}
}}
"#,
    )
}

/// Detect `snake_table_for` collisions before any file is written.
/// Two distinct `table_name` values can collide (`User` and `users` both
/// snake-case to `users`); a literal duplicate (`users` twice in `entities`)
/// also counts. Returns an `Other` IO error naming the colliding inputs.
pub(crate) fn check_no_snake_collision(entities: &[Entity]) -> std::io::Result<()> {
    use std::collections::HashMap;
    let mut seen: HashMap<String, String> = HashMap::with_capacity(entities.len());
    for e in entities {
        let snake = snake_table_for(&e.table_name);
        if let Some(prior) = seen.get(&snake) {
            return Err(std::io::Error::other(format!(
                "snake_table_for collision: `{}` and `{}` both produce `{}` \
                 — rename one to avoid silent overwrite + duplicate `pub mod` (E0428)",
                prior, e.table_name, snake
            )));
        }
        seen.insert(snake, e.table_name.clone());
    }
    Ok(())
}

/// Render `src/models/_entities/mod.rs`.
pub fn render_entities_mod(entities: &[Entity]) -> String {
    let mut s = String::from("pub mod prelude;\n\n");
    for e in entities {
        s.push_str(&format!("pub mod {};\n", snake_table_for(&e.table_name)));
    }
    s
}

/// Render `src/models/_entities/prelude.rs`.
pub fn render_entities_prelude(entities: &[Entity]) -> String {
    let mut s = String::new();
    for e in entities {
        let snake = snake_table_for(&e.table_name);
        let pascal = pascal_case(&snake);
        s.push_str(&format!("pub use super::{snake}::Entity as {pascal};\n"));
    }
    s
}

/// Render `src/models/mod.rs`.
pub fn render_models_mod(entities: &[Entity]) -> String {
    let mut s = String::from("pub mod _entities;\n");
    for e in entities {
        s.push_str(&format!("pub mod {};\n", snake_table_for(&e.table_name)));
    }
    s
}

/// Render `src/controllers/mod.rs`.
pub fn render_controllers_mod(entities: &[Entity]) -> String {
    let mut s = String::new();
    for e in entities {
        s.push_str(&format!("pub mod {};\n", snake_table_for(&e.table_name)));
    }
    s
}

/// Render `migration/src/lib.rs`.
pub fn render_migration_lib(entities: &[Entity], timestamps: &[String]) -> String {
    let mut s = String::from(
        "#![allow(elided_lifetimes_in_paths)]\n\
         #![allow(clippy::wildcard_imports)]\n\
         pub use sea_orm_migration::prelude::*;\n\n",
    );
    for (e, ts) in entities.iter().zip(timestamps.iter()) {
        s.push_str(&format!("mod {ts}_{};\n", snake_table_for(&e.table_name)));
    }
    s.push_str("\npub struct Migrator;\n\n");
    s.push_str("#[async_trait::async_trait]\n");
    s.push_str("impl MigratorTrait for Migrator {\n");
    s.push_str("    fn migrations() -> Vec<Box<dyn MigrationTrait>> {\n");
    s.push_str("        vec![\n");
    for (e, ts) in entities.iter().zip(timestamps.iter()) {
        s.push_str(&format!(
            "            Box::new({ts}_{}::Migration),\n",
            snake_table_for(&e.table_name)
        ));
    }
    s.push_str("        ]\n    }\n}\n");
    s
}

/// Render a `src/app_routes.rs` snippet the user can paste into their
/// app's `Hooks::routes` impl.
pub fn render_app_routes(entities: &[Entity]) -> String {
    let mut s = String::from(
        "// Drop the body of this fn into your `Hooks::routes` impl in `src/app.rs`,\n\
         // or call `register_generated_routes(routes)` from there.\n\n\
         use loco_rs::controller::AppRoutes;\n\n\
         pub fn register_generated_routes(routes: AppRoutes) -> AppRoutes {\n    routes\n",
    );
    for e in entities {
        s.push_str(&format!(
            "        .add_route(crate::controllers::{}::routes())\n",
            snake_table_for(&e.table_name)
        ));
    }
    s.push_str("}\n");
    s
}

/// PascalCase a snake_case name (`learning_collections` → `LearningCollections`).
fn pascal_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut up = true;
    for c in s.chars() {
        if c == '_' {
            up = true;
        } else if up {
            for uc in c.to_uppercase() {
                out.push(uc);
            }
            up = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Write the full Loco project tree (entities, models, controllers,
/// migrations, module aggregators, and `src/app_routes.rs`) under `root`.
/// Creates any missing directories. Returns the per-entity migration
/// timestamps in declaration order so callers can wire them into
/// `migration/src/lib.rs` if needed.
pub fn write_loco_project(
    entities: &[Entity],
    root: &std::path::Path,
) -> std::io::Result<Vec<String>> {
    use std::fs;
    // Pre-fix, two entities whose `table_name` snake-collided (e.g. `User`
    // and `users` both → `users`) silently overwrote each other's
    // generated files via the truncating `fs::write` calls below, AND
    // produced a `mod.rs` with two `pub mod users;` lines (Rust E0428).
    // Detect collision up front and refuse — the caller chose the table
    // names, so this is user error.
    check_no_snake_collision(entities)?;
    let entities_dir = root.join("src/models/_entities");
    let models_dir = root.join("src/models");
    let controllers_dir = root.join("src/controllers");
    let migration_dir = root.join("migration/src");
    for d in [&entities_dir, &models_dir, &controllers_dir, &migration_dir] {
        fs::create_dir_all(d)?;
    }

    let base_ts = chrono_like_stamp();
    let mut timestamps: Vec<String> = Vec::with_capacity(entities.len());

    for (i, entity) in entities.iter().enumerate() {
        let snake = snake_table_for(&entity.table_name);
        fs::write(
            entities_dir.join(format!("{snake}.rs")),
            render_entity(entity),
        )?;
        fs::write(models_dir.join(format!("{snake}.rs")), render_model(entity))?;
        fs::write(
            controllers_dir.join(format!("{snake}.rs")),
            render_controller(entity),
        )?;
        let ts = format!("{}{:02}", base_ts, i);
        fs::write(
            migration_dir.join(format!("{ts}_{snake}.rs")),
            render_migration(entity),
        )?;
        timestamps.push(ts);
    }

    fs::write(entities_dir.join("mod.rs"), render_entities_mod(entities))?;
    fs::write(
        entities_dir.join("prelude.rs"),
        render_entities_prelude(entities),
    )?;
    merge_mod_rs(
        &models_dir.join("mod.rs"),
        entities,
        /*include_entities=*/ true,
    )?;
    merge_mod_rs(&controllers_dir.join("mod.rs"), entities, false)?;
    fs::write(
        migration_dir.join("lib.rs"),
        render_migration_lib(entities, &timestamps),
    )?;
    fs::write(root.join("src/app_routes.rs"), render_app_routes(entities))?;

    Ok(timestamps)
}

/// Merge generated `pub mod <table>;` declarations into an existing
/// `mod.rs`, preserving any modules already declared by the scaffold
/// (e.g. `home` controller, `users` model). Idempotent: re-running adds
/// nothing new. If the file doesn't exist yet, writes a fresh aggregator
/// using [`render_models_mod`] / [`render_controllers_mod`].
fn merge_mod_rs(
    path: &std::path::Path,
    entities: &[Entity],
    include_entities: bool,
) -> std::io::Result<()> {
    use std::fs;
    let existing = fs::read_to_string(path).unwrap_or_default();
    let mut out = existing.clone();
    if include_entities && !existing.contains("pub mod _entities;") {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("pub mod _entities;\n");
    }
    for e in entities {
        let snake = snake_table_for(&e.table_name);
        let decl = format!("pub mod {snake};");
        if !existing.lines().any(|l| l.trim() == decl) {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&decl);
            out.push('\n');
        }
    }
    if out != existing {
        fs::write(path, out)?;
    }
    Ok(())
}

/// Stable per-run migration timestamp prefix, e.g. `m20260601_120000`.
/// Approximate (epoch math, no chrono dep) — caller appends a per-entity
/// suffix for uniqueness.
pub fn chrono_like_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    let year = 1970 + (days / 365);
    let day_of_year = days % 365;
    let month = (day_of_year / 31) + 1;
    let day = (day_of_year % 31) + 1;
    let h = (secs % 86_400) / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("m{year:04}{month:02}{day:02}_{h:02}{m:02}{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{ColumnToField, Entity};

    fn col(name: &str, db_type: Option<&str>, id_type: Option<&str>) -> ColumnToField {
        ColumnToField {
            database_id_type: id_type.map(String::from),
            database_column_name: Some(name.into()),
            camel_case_field_name: Some(name.into()),
            database_type: db_type.map(String::from),
            java_type: None,
        }
    }

    fn sample() -> Entity {
        Entity {
            table_name: "ACTIVITY".into(),
            entity_name: "Activity".into(),
            columns: vec![
                col("ACTIVITY_ID", None, Some("@Id")),
                col("MODULE_ID", None, Some("@ManyToOne")),
                col("NAME", Some("varchar(150)"), None),
                col("ACTIVE", Some("varchar(1)"), None),
                col("CREATE_DATE", Some("datetime"), None),
                col("UPDATE_DATE", Some("timestamp"), None),
            ],
        }
    }

    #[test]
    fn snake_table_pluralises_and_lowercases() {
        assert_eq!(snake_table_for("ACTIVITY"), "activitys");
        assert_eq!(snake_table_for("AlphabetCharacter"), "alphabet_characters");
        assert_eq!(snake_table_for("users"), "users");
    }

    #[test]
    fn rust_type_pk_is_i32() {
        let c = col("id", None, Some("@Id"));
        assert_eq!(rust_type_for(&c), "i32");
    }

    #[test]
    fn rust_type_for_varchar_is_string() {
        let c = col("name", Some("varchar(150)"), None);
        assert_eq!(rust_type_for(&c), "String");
    }

    #[test]
    fn rust_type_for_bigint_is_i64() {
        let c = col("amount", Some("BIGINT"), None);
        assert_eq!(rust_type_for(&c), "i64");
    }

    #[test]
    fn rust_type_for_datetime_is_dttz() {
        let c = col("created", Some("datetime"), None);
        assert_eq!(rust_type_for(&c), "DateTimeWithTimeZone");
    }

    #[test]
    fn loco_col_type_pk_is_pkauto() {
        let c = col("id", None, Some("@Id"));
        assert_eq!(loco_col_type_for(&c), "PkAuto");
    }

    #[test]
    fn loco_col_type_varchar_is_string() {
        let c = col("name", Some("varchar(150)"), None);
        assert_eq!(loco_col_type_for(&c), "String");
    }

    #[test]
    fn loco_col_type_timestamp_is_tstz() {
        let c = col("when", Some("timestamp"), None);
        assert_eq!(loco_col_type_for(&c), "TimestampWithTimeZone");
    }

    #[test]
    fn render_entity_has_primary_key_attr_and_table_name() {
        let out = render_entity(&sample());
        assert!(out.contains("#[sea_orm(table_name = \"activitys\")]"));
        assert!(out.contains("#[sea_orm(primary_key)]"));
        assert!(out.contains("pub activity_id: i32,"));
        assert!(out.contains("pub name: String,"));
        assert!(out.contains("pub create_date: DateTimeWithTimeZone,"));
    }

    #[test]
    fn render_model_reexports_and_impls_active_model_behavior() {
        let out = render_model(&sample());
        assert!(out.contains("pub use super::_entities::activitys::"));
        assert!(out.contains("pub type Activitys = Entity;"));
        assert!(out.contains("impl ActiveModelBehavior for ActiveModel {}"));
    }

    #[test]
    fn render_controller_has_crud_routes_and_skips_pk_in_params() {
        let out = render_controller(&sample());
        assert!(out.contains(".prefix(\"api/activitys\")"));
        assert!(out.contains(".add(\"/\", get(list))"));
        assert!(out.contains(".add(\"/\", post(add))"));
        assert!(out.contains(".add(\"/{id}\", get(get_one))"));
        assert!(out.contains(".add(\"/{id}\", delete(remove))"));
        assert!(out.contains(".add(\"/{id}\", put(update))"));
        // PK must be absent from POST params
        assert!(!out.contains("pub activity_id: i32,"));
        assert!(out.contains("pub name: String,"));
    }

    #[test]
    fn render_migration_emits_create_table_with_pkauto() {
        let out = render_migration(&sample());
        assert!(out.contains("create_table(m, \"activitys\","));
        assert!(out.contains("(\"activity_id\", ColType::PkAuto)"));
        assert!(out.contains("(\"name\", ColType::String)"));
        assert!(out.contains("(\"create_date\", ColType::TimestampWithTimeZone)"));
        assert!(out.contains("drop_table(m, \"activitys\")"));
    }

    #[test]
    fn render_migration_lib_lists_every_migration_in_order() {
        let entities = vec![sample()];
        let stamps = vec!["m20260601_120000".to_string()];
        let out = render_migration_lib(&entities, &stamps);
        assert!(out.contains("mod m20260601_120000_activitys;"));
        assert!(out.contains("Box::new(m20260601_120000_activitys::Migration)"));
    }

    #[test]
    fn render_controllers_mod_lists_every_table() {
        let entities = vec![sample()];
        let out = render_controllers_mod(&entities);
        assert!(out.contains("pub mod activitys;"));
    }

    #[test]
    fn render_app_routes_wires_every_entity() {
        let entities = vec![sample()];
        let out = render_app_routes(&entities);
        assert!(out.contains("crate::controllers::activitys::routes()"));
    }
}
