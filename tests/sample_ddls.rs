//! Real-world sample DDL smoke test.
//!
//! Drives the full parse + Loco-render pipeline against every dump in
//! `samples/` and asserts each one produces the expected entity count
//! and well-formed output (no `Entity must have a primary key`-style
//! anomalies, no empty struct bodies, etc).
//!
//! This is a pure parser/renderer test — it does NOT invoke `cargo` on
//! the generated project (that's covered by manual end-to-end runs in
//! the README). It pins the parsing/normalising behaviour so we don't
//! regress the open-source schemas we've validated against.

use std::fs::File;

use api_rest_generator::config::Configuration;
use api_rest_generator::globals::Globals;
use api_rest_generator::loco;
use api_rest_generator::normalize::{
    normalize_mssql_words, normalize_postgresql_words, normalize_sqlite_words,
};
use api_rest_generator::parser::{get_words, parse_words};

fn parse_sample(path: &str, db: &str) -> Vec<api_rest_generator::entity::Entity> {
    let mut props = std::collections::HashMap::new();
    props.insert("target.folder".to_string(), ".".to_string());
    props.insert("target.package".to_string(), "".to_string());
    props.insert("file.name".to_string(), path.to_string());
    props.insert("target.language".to_string(), "rust-loco".to_string());
    props.insert("database.type".to_string(), db.to_string());
    let cfg = Configuration::from_properties(&props);
    Globals::set(cfg.to_globals());

    let file = File::open(path).unwrap_or_else(|e| panic!("open {path}: {e}"));
    let mut words = get_words(file);
    match db {
        "postgresql" => normalize_postgresql_words(&mut words),
        "sqlite" => normalize_sqlite_words(&mut words),
        "mssql" => normalize_mssql_words(&mut words),
        _ => {}
    }
    parse_words(&words)
}

/// Returns `true` if the sample file exists. The samples are committed
/// to the repo but skip gracefully if someone runs `cargo test` from a
/// shallow clone that excluded them.
fn sample_present(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

#[test]
fn sakila_mysql_parses_and_renders() {
    let path = "samples/sakila-mysql.sql";
    if !sample_present(path) {
        eprintln!("skipping: {path} not present");
        return;
    }
    let entities = parse_sample(path, "mysql");
    assert!(
        entities.len() >= 15,
        "expected at least 15 Sakila tables, got {}",
        entities.len()
    );
    // every entity must produce a renderable controller / model / migration
    for e in &entities {
        let c = loco::render_controller(e);
        let m = loco::render_migration(e);
        let ent = loco::render_entity(e);
        assert!(
            c.contains("pub struct Params"),
            "missing Params for {}",
            e.table_name
        );
        assert!(
            m.contains("create_table"),
            "missing create_table for {}",
            e.table_name
        );
        assert!(
            ent.contains("primary_key"),
            "missing PK for {} (sakila)",
            e.table_name
        );
    }
}

#[test]
fn chinook_sqlite_parses_and_renders() {
    let path = "samples/chinook-sqlite.sql";
    if !sample_present(path) {
        eprintln!("skipping: {path} not present");
        return;
    }
    let entities = parse_sample(path, "sqlite");
    assert!(
        entities.len() >= 11,
        "expected at least 11 Chinook tables, got {}",
        entities.len()
    );
    // Chinook uses [bracketed] identifiers — make sure none leak.
    for e in &entities {
        assert!(
            !e.table_name.contains('[') && !e.table_name.contains(']'),
            "leaked brackets in {}",
            e.table_name
        );
        let c = loco::render_controller(e);
        assert!(
            !c.contains('\t'),
            "tabs leaked into {} columns",
            e.table_name
        );
        // Composite PK tables (playlist_track) MUST get a synthetic PK so
        // the SeaORM entity compiles.
        let ent = loco::render_entity(e);
        assert!(
            ent.contains("primary_key"),
            "missing synthetic PK for {} (chinook)",
            e.table_name
        );
    }
}

#[test]
fn pagila_postgresql_parses_and_renders() {
    let path = "samples/pagila-postgresql.sql";
    if !sample_present(path) {
        eprintln!("skipping: {path} not present");
        return;
    }
    let entities = parse_sample(path, "postgresql");
    assert!(
        entities.len() >= 15,
        "expected at least 15 Pagila tables, got {}",
        entities.len()
    );
    // Pagila declares PKs via ALTER TABLE — every table should still get
    // a PK (either real or synthetic) so the entity compiles.
    for e in &entities {
        let ent = loco::render_entity(e);
        assert!(
            ent.contains("primary_key"),
            "missing PK for {} (pagila)",
            e.table_name
        );
    }
}

#[test]
fn northwind_mssql_parses_and_renders() {
    let path = "samples/northwind-mssql.sql";
    if !sample_present(path) {
        eprintln!("skipping: {path} not present");
        return;
    }
    let entities = parse_sample(path, "mssql");
    assert!(
        entities.len() >= 10,
        "expected at least 10 Northwind tables, got {}",
        entities.len()
    );
    let table_names: Vec<&str> = entities.iter().map(|e| e.table_name.as_str()).collect();
    // "Order Details" must be collapsed into a single identifier so it
    // doesn't collide with "Orders" after pluralisation.
    assert!(
        table_names
            .iter()
            .any(|n| n.to_ascii_lowercase().contains("order_details")
                || n.to_ascii_lowercase().contains("orderdetails")),
        "Order Details table missing, got: {table_names:?}"
    );
    // No leaked quotes / brackets / tabs in any column.
    for e in &entities {
        let c = loco::render_controller(e);
        assert!(
            !c.contains('\t'),
            "tabs leaked into {} columns",
            e.table_name
        );
        assert!(
            !c.contains("pub \"") && !c.contains("pub ["),
            "quoted identifier leaked into {} controller",
            e.table_name
        );
    }
}
