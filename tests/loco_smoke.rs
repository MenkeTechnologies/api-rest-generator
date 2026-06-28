//! End-to-end smoke test for the Rust/Loco generator target.
//!
//! Drives the public `loco::*` renderers across a minimal in-memory
//! [`Entity`] graph, then asserts the output contains the SeaORM
//! attributes, Loco controller routes, and migration entries we expect.
//!
//! Complements the unit pins inside `src/loco.rs` (which target each
//! renderer in isolation) by ensuring the whole pipeline -- model +
//! controller + migration + module trees + app_routes -- agrees on
//! naming/case for the same set of entities.

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

fn fixtures() -> Vec<Entity> {
    vec![
        Entity {
            table_name: "USERS".into(),
            entity_name: "Users".into(),
            columns: vec![
                col("user_id", None, Some("@Id")),
                col("email", Some("varchar(255)"), None),
                col("created_at", Some("timestamp"), None),
                col("type", Some("varchar(32)"), None), // reserved Rust keyword
            ],
        },
        Entity {
            table_name: "ORDERS".into(),
            entity_name: "Orders".into(),
            columns: vec![
                col("order_id", None, Some("@Id")),
                col("user_id", None, Some("@ManyToOne")),
                col("amount", Some("bigint"), None),
                col("placed_on", Some("date"), None),
            ],
        },
    ]
}

#[test]
fn loco_target_renders_full_skeleton_for_multi_entity_dump() {
    let entities = fixtures();

    // -- per-entity outputs -------------------------------------------------
    let users_entity = loco::render_entity(&entities[0]);
    assert!(users_entity.contains("#[sea_orm(table_name = \"users\")]"));
    assert!(users_entity.contains("#[sea_orm(primary_key)]"));
    assert!(users_entity.contains("pub user_id: i32,"));
    assert!(users_entity.contains("pub email: String,"));
    assert!(users_entity.contains("pub created_at: DateTimeWithTimeZone,"));
    // Reserved keyword must be raw-escaped, not renamed.
    assert!(
        users_entity.contains("pub r#type: String,"),
        "expected r#type escaping, got:\n{users_entity}"
    );

    let users_controller = loco::render_controller(&entities[0]);
    assert!(users_controller.contains(".prefix(\"api/users\")"));
    assert!(users_controller.contains(".add(\"/\", get(list))"));
    assert!(users_controller.contains(".add(\"/{id}\", put(update))"));
    // PK absent from Params.
    assert!(!users_controller.contains("pub user_id: i32,"));
    // Reserved keyword escaping survives into Params/update.
    assert!(users_controller.contains("pub r#type: String,"));
    assert!(users_controller.contains("item.r#type = sea_orm::ActiveValue::set"));

    let orders_entity = loco::render_entity(&entities[1]);
    assert!(orders_entity.contains("pub order_id: i32,"));
    assert!(orders_entity.contains("pub user_id: i32,"));
    assert!(orders_entity.contains("pub amount: i64,"));
    assert!(orders_entity.contains("pub placed_on: Date,"));

    let orders_migration = loco::render_migration(&entities[1]);
    assert!(orders_migration.contains("create_table(m, \"orders\","));
    assert!(orders_migration.contains("(\"order_id\", ColType::PkAuto)"));
    assert!(orders_migration.contains("(\"user_id\", ColType::Integer)"));
    assert!(orders_migration.contains("(\"amount\", ColType::BigInteger)"));
    assert!(orders_migration.contains("(\"placed_on\", ColType::Date)"));
    assert!(orders_migration.contains("drop_table(m, \"orders\")"));

    // -- module aggregators ------------------------------------------------
    let models_mod = loco::render_models_mod(&entities);
    assert!(models_mod.contains("pub mod _entities;"));
    assert!(models_mod.contains("pub mod users;"));
    assert!(models_mod.contains("pub mod orders;"));

    let entities_mod = loco::render_entities_mod(&entities);
    assert!(entities_mod.contains("pub mod prelude;"));
    assert!(entities_mod.contains("pub mod users;"));
    assert!(entities_mod.contains("pub mod orders;"));

    let prelude = loco::render_entities_prelude(&entities);
    assert!(prelude.contains("pub use super::users::Entity as Users;"));
    assert!(prelude.contains("pub use super::orders::Entity as Orders;"));

    let controllers_mod = loco::render_controllers_mod(&entities);
    assert!(controllers_mod.contains("pub mod users;"));
    assert!(controllers_mod.contains("pub mod orders;"));

    // -- migration registry ------------------------------------------------
    let stamps = vec!["m20260101_120000".into(), "m20260101_120001".into()];
    let mig_lib = loco::render_migration_lib(&entities, &stamps);
    assert!(mig_lib.contains("mod m20260101_120000_users;"));
    assert!(mig_lib.contains("mod m20260101_120001_orders;"));
    assert!(mig_lib.contains("Box::new(m20260101_120000_users::Migration)"));
    assert!(mig_lib.contains("Box::new(m20260101_120001_orders::Migration)"));

    // -- routes wiring snippet --------------------------------------------
    let routes = loco::render_app_routes(&entities);
    assert!(routes.contains("crate::controllers::users::routes()"));
    assert!(routes.contains("crate::controllers::orders::routes()"));
}

#[test]
fn snake_table_for_pluralises_and_lowercases_consistently() {
    assert_eq!(loco::snake_table_for("UserAccount"), "user_accounts");
    assert_eq!(loco::snake_table_for("ORDER_ITEM"), "order_items");
    assert_eq!(loco::snake_table_for("users"), "users");
}

#[test]
fn rust_type_for_covers_common_sql_types() {
    let make = |db: Option<&str>, idt: Option<&str>| ColumnToField {
        database_id_type: idt.map(String::from),
        database_column_name: Some("c".into()),
        camel_case_field_name: Some("c".into()),
        database_type: db.map(String::from),
        java_type: None,
    };
    assert_eq!(loco::rust_type_for(&make(None, Some("@Id"))), "i32");
    assert_eq!(loco::rust_type_for(&make(None, Some("@ManyToOne"))), "i32");
    assert_eq!(loco::rust_type_for(&make(Some("bigint"), None)), "i64");
    assert_eq!(loco::rust_type_for(&make(Some("integer"), None)), "i32");
    assert_eq!(loco::rust_type_for(&make(Some("smallint"), None)), "i16");
    assert_eq!(
        loco::rust_type_for(&make(Some("varchar(255)"), None)),
        "String"
    );
    assert_eq!(loco::rust_type_for(&make(Some("text"), None)), "String");
    assert_eq!(loco::rust_type_for(&make(Some("boolean"), None)), "bool");
    assert_eq!(loco::rust_type_for(&make(Some("float"), None)), "f32");
    assert_eq!(loco::rust_type_for(&make(Some("double"), None)), "f64");
    assert_eq!(
        loco::rust_type_for(&make(Some("decimal(10,2)"), None)),
        "f64"
    );
    assert_eq!(loco::rust_type_for(&make(Some("date"), None)), "Date");
    assert_eq!(loco::rust_type_for(&make(Some("time"), None)), "Time");
    assert_eq!(
        loco::rust_type_for(&make(Some("timestamp"), None)),
        "DateTimeWithTimeZone"
    );
    assert_eq!(
        loco::rust_type_for(&make(Some("datetime"), None)),
        "DateTimeWithTimeZone"
    );
    assert_eq!(loco::rust_type_for(&make(Some("blob"), None)), "Vec<u8>");
    // Unknown types fall back to String for safety.
    assert_eq!(loco::rust_type_for(&make(Some("jsonb"), None)), "String");
}
