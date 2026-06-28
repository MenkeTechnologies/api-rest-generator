//! Unit-level pins for the per-dialect word-stream normalisers.
//!
//! The existing `parity_smoke.rs` exercises the full DDL pipeline on real
//! fixtures, so it can't pin individual multi-word folds (collapse rules,
//! noise stripping, bracket removal) in isolation. These tests run the
//! pure `normalize_*_words` functions directly on hand-crafted token
//! lists so a regression in a single fold lands on a single failing test
//! instead of a diffuse pipeline-level shape break.

use api_rest_generator::normalize::{
    normalize_mssql_words, normalize_postgresql_words, normalize_sqlite_words,
};
use api_rest_generator::parser::get_words;

fn words(s: &str) -> Vec<String> {
    s.split_whitespace().map(String::from).collect()
}

// ---------------------------------------------------------------- PostgreSQL

#[test]
fn pg_character_varying_collapses_to_varchar_with_size() {
    let mut w = words("name character varying(255)");
    normalize_postgresql_words(&mut w);
    assert_eq!(w, vec!["name", "varchar(255)"]);
}

#[test]
fn pg_character_varying_collapses_when_size_absent() {
    let mut w = words("name character varying");
    normalize_postgresql_words(&mut w);
    assert_eq!(w, vec!["name", "varchar"]);
}

#[test]
fn pg_character_alone_is_left_untouched() {
    // `character` without a following `varying` token must not be rewritten.
    let mut w = words("name character(10)");
    normalize_postgresql_words(&mut w);
    assert_eq!(w, vec!["name", "character(10)"]);
}

#[test]
fn pg_timestamp_without_time_zone_collapses_to_timestamp() {
    let mut w = words("created timestamp without time zone");
    normalize_postgresql_words(&mut w);
    assert_eq!(w, vec!["created", "timestamp"]);
}

#[test]
fn pg_timestamp_with_time_zone_collapses_to_timestamp() {
    let mut w = words("created timestamp with time zone");
    normalize_postgresql_words(&mut w);
    assert_eq!(w, vec!["created", "timestamp"]);
}

#[test]
fn pg_double_precision_collapses_to_double() {
    let mut w = words("ratio double precision");
    normalize_postgresql_words(&mut w);
    assert_eq!(w, vec!["ratio", "double"]);
}

#[test]
fn pg_trailing_semicolon_and_comma_are_stripped() {
    let mut w = words("col1, col2;");
    normalize_postgresql_words(&mut w);
    assert_eq!(w, vec!["col1", "col2"]);
}

#[test]
fn pg_normalize_is_case_insensitive_on_keywords() {
    let mut w = words("name CHARACTER VARYING(100)");
    normalize_postgresql_words(&mut w);
    assert_eq!(w, vec!["name", "varchar(100)"]);
}

#[test]
fn pg_multiple_folds_in_one_stream() {
    let mut w = words("a character varying(64) b double precision c timestamp without time zone");
    normalize_postgresql_words(&mut w);
    assert_eq!(w, vec!["a", "varchar(64)", "b", "double", "c", "timestamp"]);
}

// ---------------------------------------------------------------- SQLite

#[test]
fn sqlite_if_not_exists_is_removed() {
    // SQLite normaliser strips trailing `;` and `,` only (not `(` / `)`),
    // so `(id` and `INT)` keep their parens — what matters here is that
    // the three IF / NOT / EXISTS tokens are gone.
    let mut w = words("CREATE TABLE IF NOT EXISTS users (id INT)");
    normalize_sqlite_words(&mut w);
    assert_eq!(w, vec!["CREATE", "TABLE", "users", "(id", "INT)"]);
}

#[test]
fn sqlite_pragma_block_drops_until_create() {
    let mut w = words("PRAGMA foreign_keys=ON; CREATE TABLE t (id INT)");
    normalize_sqlite_words(&mut w);
    // PRAGMA run is dropped; tokens from CREATE forward are preserved.
    assert_eq!(w[0], "CREATE");
    assert!(w.contains(&"TABLE".to_string()));
}

#[test]
fn sqlite_insert_block_drops_until_alter() {
    let mut w = words("INSERT INTO t VALUES (1); ALTER TABLE t ADD col");
    normalize_sqlite_words(&mut w);
    assert_eq!(w[0], "ALTER");
}

#[test]
fn sqlite_begin_commit_pair_drops_when_followed_by_create() {
    let mut w = words("BEGIN; COMMIT; CREATE TABLE t (id INT)");
    normalize_sqlite_words(&mut w);
    assert_eq!(w[0], "CREATE");
}

#[test]
fn sqlite_delete_block_drops_until_create() {
    let mut w = words("DELETE FROM t WHERE id=1; CREATE TABLE u (x INT)");
    normalize_sqlite_words(&mut w);
    assert_eq!(w[0], "CREATE");
}

#[test]
fn sqlite_if_not_exists_lowercase_also_removed() {
    let mut w = words("CREATE TABLE if not exists t (id INT)");
    normalize_sqlite_words(&mut w);
    assert!(!w.iter().any(|t| t.eq_ignore_ascii_case("IF")));
    assert!(!w.iter().any(|t| t.eq_ignore_ascii_case("NOT")));
    assert!(!w.iter().any(|t| t.eq_ignore_ascii_case("EXISTS")));
}

// ---------------------------------------------------------------- MSSQL

#[test]
fn mssql_square_brackets_are_stripped() {
    let mut w = words("CREATE TABLE [dbo].[Users] ( [Id] INT )");
    normalize_mssql_words(&mut w);
    // No remaining `[` or `]` characters anywhere in the stream.
    for t in &w {
        assert!(
            !t.contains('[') && !t.contains(']'),
            "bracket leaked through: {t:?}"
        );
    }
}

#[test]
fn mssql_varchar_max_paren_is_dropped() {
    let mut w = words("name varchar(max)");
    normalize_mssql_words(&mut w);
    assert_eq!(w, vec!["name", "varchar"]);
}

#[test]
fn mssql_varchar_max_uppercase_paren_is_dropped() {
    let mut w = words("name VARCHAR(MAX)");
    normalize_mssql_words(&mut w);
    assert_eq!(w[0], "name");
    assert!(w[1].eq_ignore_ascii_case("varchar"));
}

#[test]
fn mssql_clustered_keyword_is_dropped() {
    let mut w = words("PRIMARY KEY CLUSTERED (Id)");
    normalize_mssql_words(&mut w);
    assert!(!w.iter().any(|t| t.eq_ignore_ascii_case("CLUSTERED")));
}

#[test]
fn mssql_nonclustered_keyword_is_dropped() {
    let mut w = words("INDEX IX_Foo NONCLUSTERED (col)");
    normalize_mssql_words(&mut w);
    assert!(!w.iter().any(|t| t.eq_ignore_ascii_case("NONCLUSTERED")));
}

#[test]
fn mssql_asc_desc_in_index_decls_are_dropped() {
    let mut w = words("INDEX IX_A (col ASC) INDEX IX_B (col DESC)");
    normalize_mssql_words(&mut w);
    assert!(!w.iter().any(|t| t.eq_ignore_ascii_case("ASC")));
    assert!(!w.iter().any(|t| t.eq_ignore_ascii_case("DESC")));
}

#[test]
fn mssql_identity_token_is_dropped() {
    let mut w = words("Id INT IDENTITY(1,1)");
    normalize_mssql_words(&mut w);
    assert!(
        !w.iter().any(|t| t.to_uppercase().starts_with("IDENTITY")),
        "IDENTITY token leaked: {w:?}"
    );
}

#[test]
fn mssql_set_block_drops_until_create() {
    let mut w = words("SET ANSI_NULLS ON; CREATE TABLE t (id INT)");
    normalize_mssql_words(&mut w);
    assert_eq!(w[0], "CREATE");
}

#[test]
fn mssql_use_block_drops_until_create() {
    let mut w = words("USE master GO CREATE TABLE t (id INT)");
    normalize_mssql_words(&mut w);
    assert_eq!(w[0], "CREATE");
}

#[test]
fn mssql_exec_block_drops_until_create() {
    let mut w = words("EXEC sp_addtype 'foo' CREATE TABLE t (id INT)");
    normalize_mssql_words(&mut w);
    assert_eq!(w[0], "CREATE");
}

#[test]
fn mssql_print_block_drops_until_alter() {
    let mut w = words("PRINT 'building' ALTER TABLE t ADD col");
    normalize_mssql_words(&mut w);
    assert_eq!(w[0], "ALTER");
}

// ---------------------------------------------------------------- get_words

#[test]
fn get_words_strips_hash_comment_lines() {
    let input = "# this is a comment\nCREATE TABLE t (id INT)";
    let w = get_words(input.as_bytes());
    assert!(!w.iter().any(|t| t.starts_with('#')));
    assert!(w.contains(&"CREATE".to_string()));
}

#[test]
fn get_words_strips_double_dash_sql_comment_lines() {
    let input = "-- SQL comment line\nCREATE TABLE t (id INT)";
    let w = get_words(input.as_bytes());
    assert!(!w.iter().any(|t| t.starts_with("--")));
    assert!(w.contains(&"CREATE".to_string()));
}

#[test]
fn get_words_drops_empty_tokens_from_multispace_runs() {
    // Double/triple spaces should not produce empty tokens.
    let input = "CREATE   TABLE    t";
    let w = get_words(input.as_bytes());
    assert!(!w.iter().any(|t| t.is_empty()));
    assert_eq!(w, vec!["CREATE", "TABLE", "t"]);
}

#[test]
fn get_words_preserves_token_order_across_lines() {
    let input = "CREATE TABLE\nfoo (\nid INT\n)";
    let w = get_words(input.as_bytes());
    assert_eq!(w[0], "CREATE");
    assert_eq!(w[1], "TABLE");
    assert_eq!(w[2], "foo");
}

#[test]
fn get_words_comment_detection_respects_leading_whitespace() {
    // Indented `--` and `#` lines must still be treated as comments
    // (matches the Kotlin `trim` semantics in the source tokenizer).
    let input = "   -- indented comment\n\t# tab-indented comment\nCREATE";
    let w = get_words(input.as_bytes());
    assert_eq!(w, vec!["CREATE"]);
}
