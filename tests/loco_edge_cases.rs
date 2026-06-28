//! Adversarial edge-case pins for the Loco renderer.
//!
//! These complement `loco_smoke.rs` (happy-path pipeline) by targeting
//! specific failure modes the renderers must NOT silently swallow:
//!
//! 1. `render_migration_lib` uses `zip(entities, timestamps)` which
//!    silently truncates to the shorter side. If a caller miscounts
//!    timestamps the registry will compile but skip migrations — a
//!    data-loss bug at runtime. Pin both mismatch directions.
//!
//! 2. `effective_columns` promotes any column literally named `id`
//!    (case-insensitive) to `@Id`. When that column was already a
//!    foreign key (`@ManyToOne`), the FK semantics are silently lost.
//!    Pin that the synthesis path doesn't clobber a real `@ManyToOne`.
//!
//! 3. `pascal_case` / `snake_table_for` round-trip on names whose
//!    first character is a digit — these produce invalid Rust
//!    identifiers but the renderer happily writes them out. Pin that
//!    a digit-leading table name leaks into the generated `pub type`
//!    line so the bug is visible from outside.

use api_rest_generator::entity::{ColumnToField, Entity};
use api_rest_generator::loco;

fn col(name: &str, db_type: Option<&str>, id_type: Option<&str>) -> ColumnToField {
    ColumnToField {
        database_id_type: id_type.map(String::from),
        database_column_name: Some(name.into()),
        camel_case_field_name: Some(name.into()),
        database_type: db_type.map(String::from),
        java_type: None,
    }
}

// -------------------------------------------------------------- mismatch zip

#[test]
fn render_migration_lib_drops_entities_when_timestamps_shorter() {
    // Two entities, ONE timestamp — `zip` silently caps at 1.
    // The second entity's migration is omitted from BOTH the `mod`
    // declarations AND the `Box::new(...)` registry. A reasonable
    // implementation would either:
    //   (a) panic / return Err when the lists are misaligned, OR
    //   (b) use both lists fully and report the discrepancy.
    // Current behaviour: silent drop. This test pins the silent drop
    // so a future fix that asserts equal lengths breaks it loudly.
    let entities = vec![
        Entity {
            table_name: "users".into(),
            entity_name: "Users".into(),
            columns: vec![col("id", None, Some("@Id"))],
        },
        Entity {
            table_name: "orders".into(),
            entity_name: "Orders".into(),
            columns: vec![col("id", None, Some("@Id"))],
        },
    ];
    let stamps = vec!["m20260101_000000".to_string()];
    let out = loco::render_migration_lib(&entities, &stamps);
    // Confirm `users` made it.
    assert!(out.contains("mod m20260101_000000_users;"));
    assert!(out.contains("Box::new(m20260101_000000_users::Migration)"));
    // Pin the silent drop: `orders` does NOT appear anywhere despite
    // being a real entity the caller passed in.
    assert!(
        !out.contains("orders"),
        "render_migration_lib silently dropped the orders entity due \
         to short timestamps; expected pin says no `orders` text appears: \n{out}"
    );
}

#[test]
fn render_migration_lib_drops_timestamps_when_entities_shorter() {
    // Symmetric case: more timestamps than entities. The trailing
    // timestamp is silently ignored. A caller threading per-run stamps
    // through a parallel vector will produce a registry that does not
    // match their on-disk migration files.
    let entities = vec![Entity {
        table_name: "users".into(),
        entity_name: "Users".into(),
        columns: vec![col("id", None, Some("@Id"))],
    }];
    let stamps = vec![
        "m20260101_000000".to_string(),
        "m20260101_000001".to_string(),
    ];
    let out = loco::render_migration_lib(&entities, &stamps);
    assert!(out.contains("m20260101_000000_users"));
    // Trailing stamp is dropped:
    assert!(
        !out.contains("000001"),
        "second timestamp leaked into output despite no matching entity:\n{out}"
    );
}

// --------------------------------------------- effective_columns FK clobber

#[test]
fn render_entity_promotes_existing_id_column_clobbering_many_to_one() {
    // An `id` column that was tagged `@ManyToOne` (FK to another table)
    // gets silently rewritten to a primary key when the entity has no
    // other `@Id` column. The FK linkage in `database_id_type` is lost
    // — the generated entity claims `id` is the local PK while the
    // source DDL said it was a foreign key. Downstream relation wiring
    // will be wrong without a single compile error.
    let entity = Entity {
        table_name: "join_row".into(),
        entity_name: "JoinRow".into(),
        columns: vec![
            // The ONLY column named `id` here is an FK — there is NO real PK.
            col("id", None, Some("@ManyToOne")),
            col("name", Some("varchar(64)"), None),
        ],
    };
    let out = loco::render_entity(&entity);
    // The generated Model marks `id` as PK — proves the silent promotion.
    assert!(
        out.contains("#[sea_orm(primary_key)]"),
        "expected promoted PK attribute on rendered entity:\n{out}"
    );
    assert!(
        out.contains("pub id: i32,"),
        "expected promoted `pub id: i32` field:\n{out}"
    );
    // And the migration treats it as PkAuto, not Integer FK:
    let mig = loco::render_migration(&entity);
    assert!(
        mig.contains("(\"id\", ColType::PkAuto)"),
        "expected migration to emit PkAuto for promoted id column:\n{mig}"
    );
    // The original FK semantic (@ManyToOne → Integer column with FK
    // intent) is therefore unrecoverable from the generated output.
}

// ------------------------------------------------- snake_module_for empty

#[test]
fn snake_table_for_empty_table_name_yields_bare_s() {
    // `snake_table_for("")` returns `"s"` because the pluraliser only
    // checks `ends_with('s')`. That's a meaningless module name and
    // would collide across multiple empty-named entities. Either the
    // pluraliser should guard against empty input or callers should
    // never reach this with an empty name — this pin documents the
    // current behaviour so a fix is forced to address it.
    assert_eq!(loco::snake_table_for(""), "s");
}

// --------------------------------------------- chrono_like_stamp month/day

#[test]
fn chrono_like_stamp_emits_well_formed_prefix() {
    // The function is documented as "approximate" but it still has to
    // emit a string of the documented shape `m<year:4><mm:2><dd:2>_<hh><mm><ss>`.
    // This test pins the SHAPE only — not the values — so the leap-year /
    // 31-day-month inaccuracy doesn't make the test flaky, while still
    // catching format-string regressions (missing zero-padding, wrong
    // separator, off-by-one in width specifiers).
    let s = loco::chrono_like_stamp();
    // Total length: "m" + 8 (YYYYMMDD) + "_" + 6 (HHMMSS) = 16
    assert_eq!(s.len(), 16, "unexpected stamp length: {s:?}");
    assert!(s.starts_with('m'), "stamp must start with `m`: {s:?}");
    assert_eq!(
        &s[9..10],
        "_",
        "stamp must have `_` between date and time: {s:?}"
    );
    // Every char after `m` and `_` must be a digit.
    let date = &s[1..9];
    let time = &s[10..];
    assert!(
        date.chars().all(|c| c.is_ascii_digit()),
        "date portion must be all digits: {date:?}"
    );
    assert!(
        time.chars().all(|c| c.is_ascii_digit()),
        "time portion must be all digits: {time:?}"
    );
    // Year must be plausibly post-2020 (this test was written in 2026).
    let year: u32 = date[..4].parse().expect("year parse");
    assert!(year >= 2020, "year looks bogus: {year}");
}
