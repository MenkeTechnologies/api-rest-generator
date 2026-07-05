//! Generator entry point — port of `Main.kt`.
//!
//! Reads `src/main/resources/config.properties`, parses the configured
//! DDL file, and writes the generated entities / controllers / DAOs /
//! repositories under `<target.folder>/<target.package>/`.

use std::fs;
use std::path::{Path, PathBuf};

use api_rest_generator::config::Configuration;
use api_rest_generator::entity::Entity;
use api_rest_generator::globals::Globals;
use api_rest_generator::loco;
use api_rest_generator::normalize::{
    normalize_mssql_words, normalize_postgresql_words, normalize_sqlite_words,
};
use api_rest_generator::parser::{get_words, parse_words};
use api_rest_generator::templates::Templates;

const RESOURCES_DIR: &str = "src/main/resources";

/// Print the `--help` / `-h` screen in the MenkeTechnologies house style (see
/// `tp -h`): ANSI-Shadow banner, a status box padded at runtime so its right
/// border never drifts as the version grows, yellow `USAGE:`, cyan section
/// rules, green `//` comment separators, and a SYSTEM footer.
fn print_help() {
    const BOX_W: usize = 54;
    let ver = env!("CARGO_PKG_VERSION");
    let status = format!(" STATUS: ONLINE  // SIGNAL: ████████░░ // v{ver}");
    let space = " ".repeat(BOX_W.saturating_sub(status.chars().count()));
    let rule = "─".repeat(BOX_W);
    print!(
        concat!(
            "\n",
            "\x1b[36m  █████╗ ██████╗ ██╗██████╗ ███████╗███████╗████████╗\x1b[0m\n",
            "\x1b[36m ██╔══██╗██╔══██╗██║██╔══██╗██╔════╝██╔════╝╚══██╔══╝\x1b[0m\n",
            "\x1b[35m ███████║██████╔╝██║██████╔╝█████╗  ███████╗   ██║\x1b[0m\n",
            "\x1b[35m ██╔══██║██╔═══╝ ██║██╔══██╗██╔══╝  ╚════██║   ██║\x1b[0m\n",
            "\x1b[31m ██║  ██║██║     ██║██║  ██║███████╗███████║   ██║\x1b[0m\n",
            "\x1b[31m ╚═╝  ╚═╝╚═╝     ╚═╝╚═╝  ╚═╝╚══════╝╚══════╝   ╚═╝\x1b[0m\n",
            " \x1b[36m┌{rule}┐\x1b[0m\n",
            " \x1b[36m│\x1b[0m{status}{space}\x1b[36m│\x1b[0m\n",
            " \x1b[36m└{rule}┘\x1b[0m\n",
            "\x1b[35m  >> REST API CODE GENERATOR // FULL SPECTRUM <<\x1b[0m\n",
            "\n",
            "  Generate entities / controllers / DAOs / repositories (or a Loco app)\n",
            "  from a SQL DDL dump, driven by a properties file.\n",
            "\n",
            "\x1b[33m  USAGE:\x1b[0m api-rest-generator [OPTIONS]\n",
            "\n",
            "\x1b[36m  ── OPTIONS ─────────────────────────────────────────────\x1b[0m\n",
            "  -h, --help               \x1b[32m//\x1b[0m print this help\n",
            "  -V, --version            \x1b[32m//\x1b[0m print version\n",
            "\n",
            "\x1b[36m  ── CONFIG ──────────────────────────────────────────────\x1b[0m\n",
            "  src/main/resources/config.properties  \x1b[32m//\x1b[0m generation settings (DDL file, target folder/package, dialect)\n",
            "\n",
            "\x1b[36m  ── SYSTEM ──────────────────────────────────────────────\x1b[0m\n",
            "  \x1b[35mv{ver} \x1b[0m// \x1b[33m(c) Jacob Menke and contributors\x1b[0m\n",
            "  \x1b[35mThe schema is the spec. The code writes itself.\x1b[0m\n",
            "  \x1b[33m>>> JACK IN. FEED THE SCHEMA. SHIP THE API. <<<\x1b[0m\n",
            " \x1b[36m░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░\x1b[0m\n",
        ),
        rule = rule,
        status = status,
        space = space,
        ver = ver,
    );
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("api-rest-generator {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let resources = PathBuf::from(RESOURCES_DIR);
    let config_path = resources.join("config.properties");
    let props = Configuration::read_config(&config_path)?;
    let cfg = Configuration::from_properties(&props);
    Globals::set(cfg.to_globals());

    let dump_path = resources.join(&cfg.file_name);
    let file = fs::File::open(&dump_path)?;
    let mut words = get_words(file);

    if Globals::is_postgresql() {
        normalize_postgresql_words(&mut words);
    } else if Globals::is_sqlite() {
        normalize_sqlite_words(&mut words);
    } else if Globals::is_mssql() {
        normalize_mssql_words(&mut words);
    }

    let entities = parse_words(&words);

    if Globals::is_rust_loco() {
        write_loco(&entities, &cfg)?;
        eprintln!(
            "Generated {} Loco entit{} (models + controllers + migrations) under {}",
            entities.len(),
            if entities.len() == 1 { "y" } else { "ies" },
            cfg.src_folder,
        );
        return Ok(());
    }

    let templates = Templates::from_resources_dir(resources);
    write_templates(&templates, &entities, &cfg)?;

    eprintln!(
        "Generated {} entit{} under {}/{}",
        entities.len(),
        if entities.len() == 1 { "y" } else { "ies" },
        cfg.src_folder,
        cfg.target_package
    );
    Ok(())
}

fn write_loco(entities: &[Entity], cfg: &Configuration) -> std::io::Result<()> {
    let root = PathBuf::from(&cfg.src_folder);
    loco::write_loco_project(entities, &root)?;
    Ok(())
}

fn write_templates(
    templates: &Templates,
    entities: &[Entity],
    cfg: &Configuration,
) -> std::io::Result<()> {
    let ext = Globals::file_extension();
    for entity in entities {
        let entity_tmpl = templates.get_entity_template(entity, &cfg.target_package)?;
        write_file(
            cfg,
            "entity",
            &format!("{}{}", entity.entity_name, ext),
            &entity_tmpl,
        )?;

        let service_tmpl =
            templates.get_resource_template(&cfg.target_package, &entity.entity_name)?;
        write_file(
            cfg,
            "rest",
            &format!("{}Resource{}", entity.entity_name, ext),
            &service_tmpl,
        )?;

        let dao_tmpl = templates.get_dao_template(&cfg.target_package, &entity.entity_name)?;
        write_file(
            cfg,
            "dao",
            &format!("{}Dao{}", entity.entity_name, ext),
            &dao_tmpl,
        )?;

        let repo_tmpl =
            templates.get_repository_template(&cfg.target_package, &entity.entity_name)?;
        write_file(
            cfg,
            "repository",
            &format!("{}Repository{}", entity.entity_name, ext),
            &repo_tmpl,
        )?;
    }
    let constants_tmpl = templates.get_file_template_by_name(&cfg.target_package, "constants")?;
    write_file(
        cfg,
        "utils",
        &format!("GlobalConstants{}", ext),
        &constants_tmpl,
    )?;

    let generic_dao_tmpl =
        templates.get_file_template_by_name(&cfg.target_package, "genericdao")?;
    write_file(cfg, "dao", &format!("GenericDao{}", ext), &generic_dao_tmpl)?;
    Ok(())
}

fn write_file(cfg: &Configuration, folder: &str, file: &str, body: &str) -> std::io::Result<()> {
    let dir: PathBuf = Path::new(&cfg.src_folder)
        .join(&cfg.target_package)
        .join(folder);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(file), body)?;
    Ok(())
}
