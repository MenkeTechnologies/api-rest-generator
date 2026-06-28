//! End-to-end smoke tests for the Rust port.
//!
//! Each test pins the `Globals` state, runs the full DDL → entity-list
//! pipeline against the corresponding fixture in
//! `src/main/resources/<dialect>_dump.sql`, then checks shape invariants
//! that match what the Kotlin tests already verify.

use std::fs::File;
use std::sync::Mutex;

use api_rest_generator::globals::{Globals, GlobalsInner};
use api_rest_generator::normalize::{
    normalize_mssql_words, normalize_postgresql_words, normalize_sqlite_words,
};
use api_rest_generator::parser::{get_words, parse_words};
use once_cell::sync::Lazy;

// Globals is process-wide; serialize tests so concurrent runs don't
// step on each other's language / db_type configuration.
static GLOBALS_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn run(dialect: &str, language: &str, dump: &str) -> Vec<api_rest_generator::entity::Entity> {
    let _guard = GLOBALS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    Globals::set(GlobalsInner {
        package: "com/example/generated".into(),
        src_folder: "build/generated/".into(),
        file_name: dump.into(),
        language: language.into(),
        db_type: dialect.into(),
    });
    let file = File::open(format!("src/main/resources/{dump}")).expect("dump exists");
    let mut words = get_words(file);
    match dialect {
        "postgresql" => normalize_postgresql_words(&mut words),
        "sqlite" => normalize_sqlite_words(&mut words),
        "mssql" => normalize_mssql_words(&mut words),
        _ => {}
    }
    parse_words(&words)
}

#[test]
fn mysql_dump_parses_to_at_least_one_entity() {
    let entities = run("mysql", "java", "mysql_dump.sql");
    assert!(
        !entities.is_empty(),
        "mysql_dump.sql produced zero entities"
    );
    // Each entity must have at least one column.
    for e in &entities {
        assert!(
            !e.columns.is_empty(),
            "entity {} has zero columns",
            e.entity_name
        );
    }
}

#[test]
fn pg_dump_parses_with_pg_normalisation() {
    let entities = run("postgresql", "java", "pg_dump.sql");
    assert!(!entities.is_empty());
}

#[test]
fn sqlite_dump_parses() {
    let entities = run("sqlite", "java", "sqlite_dump.sql");
    assert!(!entities.is_empty());
}

#[test]
fn mssql_dump_parses() {
    let entities = run("mssql", "java", "mssql_dump.sql");
    assert!(!entities.is_empty());
}

#[test]
fn entity_names_are_pascal_case() {
    let entities = run("mysql", "java", "mysql_dump.sql");
    for e in &entities {
        let first = e.entity_name.chars().next().expect("non-empty name");
        assert!(
            first.is_ascii_uppercase(),
            "entity {} should start with uppercase",
            e.entity_name
        );
    }
}

#[test]
fn primary_keys_map_to_id_annotation() {
    let entities = run("mysql", "java", "mysql_dump.sql");
    let has_id = entities.iter().any(|e| {
        e.columns
            .iter()
            .any(|c| c.database_id_type.as_deref() == Some("@Id"))
    });
    assert!(
        has_id,
        "expected at least one @Id column across all parsed entities"
    );
}

#[test]
fn foreign_keys_map_to_many_to_one() {
    let entities = run("mysql", "java", "mysql_dump.sql");
    let has_fk = entities.iter().any(|e| {
        e.columns
            .iter()
            .any(|c| c.database_id_type.as_deref() == Some("@ManyToOne"))
    });
    assert!(
        has_fk,
        "expected at least one @ManyToOne column across all parsed entities"
    );
}

#[test]
fn kotlin_language_yields_kotlin_int_not_java_integer() {
    let entities = run("postgresql", "kotlin", "pg_dump.sql");
    // At least one INT column should map to "Int" not "Integer".
    let has_int = entities
        .iter()
        .flat_map(|e| &e.columns)
        .any(|c| c.java_type.as_deref() == Some("Int"));
    assert!(has_int, "expected at least one Kotlin Int mapping");
    let no_integer = entities
        .iter()
        .flat_map(|e| &e.columns)
        .all(|c| c.java_type.as_deref() != Some("Integer"));
    assert!(
        no_integer,
        "kotlin mode should never emit `Integer` (use `Int`)"
    );
}
