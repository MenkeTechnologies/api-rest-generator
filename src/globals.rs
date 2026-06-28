//! Mutable global state — port of `utils/Globals.kt`.
//!
//! The Kotlin generator uses a `Globals` object as a process-wide
//! configuration sink; mirroring that here keeps the Util / Templates
//! ports straightforward (each peeks at the active language / DB type
//! without threading config through every call site).
//!
//! Reset between runs via [`Globals::set`].

use std::sync::RwLock;

use once_cell::sync::Lazy;

use crate::constants;

/// One snapshot of the generator's global config.
#[derive(Debug, Clone, Default)]
pub struct GlobalsInner {
    /// Target Java package, e.g. `com/example/users`.
    pub package: String,
    /// Output folder root, e.g. `build/generated/src/main/java`.
    pub src_folder: String,
    /// DDL source file (classpath-resource style filename).
    pub file_name: String,
    /// Target language: `java` | `kotlin` | `groovy`.
    pub language: String,
    /// Source database flavour: `mysql` | `postgresql` | `sqlite` | `mssql`.
    pub db_type: String,
}

static G: Lazy<RwLock<GlobalsInner>> = Lazy::new(|| {
    RwLock::new(GlobalsInner {
        language: "java".into(),
        db_type: "mysql".into(),
        ..Default::default()
    })
});

/// Static façade matching the Kotlin `Globals` object.
pub struct Globals;

impl Globals {
    /// Snapshot the current globals.
    pub fn get() -> GlobalsInner {
        G.read().expect("globals read lock").clone()
    }

    /// Replace the current globals.
    pub fn set(new: GlobalsInner) {
        *G.write().expect("globals write lock") = new;
    }

    /// True if the current target language is Kotlin.
    pub fn is_kotlin() -> bool {
        Self::get().language.eq_ignore_ascii_case("kotlin")
    }
    /// True if the current target language is Groovy.
    pub fn is_groovy() -> bool {
        Self::get().language.eq_ignore_ascii_case("groovy")
    }
    /// True if the current target language is Rust/Loco.
    pub fn is_rust_loco() -> bool {
        let lang = Self::get().language;
        lang.eq_ignore_ascii_case("rust-loco") || lang.eq_ignore_ascii_case("loco")
    }
    /// True if the source DDL is PostgreSQL.
    pub fn is_postgresql() -> bool {
        Self::get().db_type.eq_ignore_ascii_case("postgresql")
    }
    /// True if the source DDL is SQLite.
    pub fn is_sqlite() -> bool {
        Self::get().db_type.eq_ignore_ascii_case("sqlite")
    }
    /// True if the source DDL is MSSQL.
    pub fn is_mssql() -> bool {
        Self::get().db_type.eq_ignore_ascii_case("mssql")
    }

    /// Identifier-escape character for the current source DB.
    pub fn escape_character() -> &'static str {
        if Self::is_postgresql() || Self::is_sqlite() {
            constants::PG_DB_ESCAPE_CHARACTER
        } else if Self::is_mssql() {
            constants::MSSQL_DB_ESCAPE_OPEN
        } else {
            constants::DB_ESCAPE_CHARACTER
        }
    }

    /// File extension for the current target language.
    pub fn file_extension() -> &'static str {
        if Self::is_kotlin() {
            ".kt"
        } else if Self::is_groovy() {
            ".groovy"
        } else if Self::is_rust_loco() {
            ".rs"
        } else {
            ".java"
        }
    }
}

#[cfg(test)]
mod tests {
    //! Globals pins. The singleton gates is_kotlin / is_groovy / is_mssql /
    //! is_postgresql / is_sqlite branches across the parser AND template
    //! renderer, plus the escape-char + file-extension dispatch. A regression
    //! here misroutes EVERY downstream generation step.
    //!
    //! Because `G` is process-wide, every test must serialise via a Mutex
    //! AND save+restore the previous state so it doesn't bleed into siblings
    //! (`parity_smoke.rs` runs in the same process and uses Globals::set too).

    use super::*;
    use std::sync::Mutex;

    static GLOBALS_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    /// Run `f` with the globals locked + restored after.
    fn with_globals<F: FnOnce()>(initial: GlobalsInner, f: F) {
        let _guard = GLOBALS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let saved = Globals::get();
        Globals::set(initial);
        f();
        Globals::set(saved);
    }

    fn lang(language: &str) -> GlobalsInner {
        GlobalsInner {
            language: language.into(),
            ..Default::default()
        }
    }

    fn db(db_type: &str) -> GlobalsInner {
        GlobalsInner {
            db_type: db_type.into(),
            ..Default::default()
        }
    }

    // ---------------------------------------- get / set round-trip

    #[test]
    fn set_then_get_returns_the_written_snapshot() {
        with_globals(GlobalsInner::default(), || {
            let snap = GlobalsInner {
                package: "com/r".into(),
                src_folder: "out/".into(),
                file_name: "r.sql".into(),
                language: "kotlin".into(),
                db_type: "postgresql".into(),
            };
            Globals::set(snap.clone());
            let got = Globals::get();
            assert_eq!(got.package, snap.package);
            assert_eq!(got.src_folder, snap.src_folder);
            assert_eq!(got.file_name, snap.file_name);
            assert_eq!(got.language, snap.language);
            assert_eq!(got.db_type, snap.db_type);
        });
    }

    #[test]
    fn get_returns_independent_clone_not_shared_reference() {
        with_globals(lang("java"), || {
            let mut a = Globals::get();
            a.language = "kotlin".into();
            // Mutating the cloned value must NOT affect the singleton.
            assert_eq!(Globals::get().language, "java");
        });
    }

    // -------------------------------------------------- is_kotlin

    #[test]
    fn is_kotlin_true_for_lowercase_kotlin() {
        with_globals(lang("kotlin"), || {
            assert!(Globals::is_kotlin());
        });
    }

    #[test]
    fn is_kotlin_case_insensitive_uppercase_mixed() {
        for v in ["KOTLIN", "Kotlin", "kOtLiN"] {
            with_globals(lang(v), || {
                assert!(Globals::is_kotlin(), "is_kotlin must accept {v}");
            });
        }
    }

    #[test]
    fn is_kotlin_false_for_java_groovy_other() {
        for v in ["java", "groovy", "scala", ""] {
            with_globals(lang(v), || {
                assert!(!Globals::is_kotlin(), "is_kotlin must reject {v}");
            });
        }
    }

    // -------------------------------------------------- is_groovy

    #[test]
    fn is_groovy_true_for_groovy_variants() {
        for v in ["groovy", "GROOVY", "Groovy"] {
            with_globals(lang(v), || {
                assert!(Globals::is_groovy(), "is_groovy must accept {v}");
            });
        }
    }

    #[test]
    fn is_groovy_false_for_java() {
        with_globals(lang("java"), || {
            assert!(!Globals::is_groovy());
        });
    }

    // ----------------------------------------------- DB-type predicates

    #[test]
    fn is_postgresql_matches_case_insensitive() {
        for v in ["postgresql", "PostgreSQL", "POSTGRESQL"] {
            with_globals(db(v), || {
                assert!(Globals::is_postgresql(), "is_postgresql must accept {v}");
            });
        }
    }

    #[test]
    fn is_postgresql_rejects_other_db_types() {
        for v in ["mysql", "mssql", "sqlite", ""] {
            with_globals(db(v), || {
                assert!(!Globals::is_postgresql(), "is_postgresql must reject {v}");
            });
        }
    }

    #[test]
    fn is_sqlite_matches_case_insensitive() {
        for v in ["sqlite", "SQLite", "SQLITE"] {
            with_globals(db(v), || {
                assert!(Globals::is_sqlite(), "is_sqlite must accept {v}");
            });
        }
    }

    #[test]
    fn is_mssql_matches_case_insensitive() {
        for v in ["mssql", "MsSql", "MSSQL"] {
            with_globals(db(v), || {
                assert!(Globals::is_mssql(), "is_mssql must accept {v}");
            });
        }
    }

    #[test]
    fn db_predicates_are_mutually_exclusive() {
        with_globals(db("mysql"), || {
            assert!(!Globals::is_postgresql());
            assert!(!Globals::is_sqlite());
            assert!(!Globals::is_mssql());
        });
    }

    // -------------------------------------------------- escape_character

    #[test]
    fn escape_character_for_postgresql_is_double_quote() {
        with_globals(db("postgresql"), || {
            assert_eq!(
                Globals::escape_character(),
                constants::PG_DB_ESCAPE_CHARACTER
            );
            assert_eq!(Globals::escape_character(), "\"");
        });
    }

    #[test]
    fn escape_character_for_sqlite_is_double_quote() {
        // SQLite shares the PG escape char per the cascade.
        with_globals(db("sqlite"), || {
            assert_eq!(
                Globals::escape_character(),
                constants::PG_DB_ESCAPE_CHARACTER
            );
        });
    }

    #[test]
    fn escape_character_for_mssql_is_open_bracket() {
        with_globals(db("mssql"), || {
            assert_eq!(Globals::escape_character(), constants::MSSQL_DB_ESCAPE_OPEN);
            assert_eq!(Globals::escape_character(), "[");
        });
    }

    #[test]
    fn escape_character_for_mysql_default_is_backtick() {
        with_globals(db("mysql"), || {
            assert_eq!(Globals::escape_character(), constants::DB_ESCAPE_CHARACTER);
            assert_eq!(Globals::escape_character(), "`");
        });
    }

    #[test]
    fn escape_character_for_unknown_db_falls_through_to_backtick() {
        // Else-branch safety: an unrecognised db_type defaults to MySQL.
        with_globals(db("oracle"), || {
            assert_eq!(Globals::escape_character(), "`");
        });
    }

    // -------------------------------------------------- file_extension

    #[test]
    fn file_extension_for_kotlin_is_kt() {
        with_globals(lang("kotlin"), || {
            assert_eq!(Globals::file_extension(), ".kt");
        });
    }

    #[test]
    fn file_extension_for_groovy_is_groovy() {
        with_globals(lang("groovy"), || {
            assert_eq!(Globals::file_extension(), ".groovy");
        });
    }

    #[test]
    fn file_extension_for_java_is_java() {
        with_globals(lang("java"), || {
            assert_eq!(Globals::file_extension(), ".java");
        });
    }

    #[test]
    fn file_extension_for_unknown_language_falls_through_to_java() {
        with_globals(lang("rust"), || {
            assert_eq!(Globals::file_extension(), ".java");
        });
    }
}
