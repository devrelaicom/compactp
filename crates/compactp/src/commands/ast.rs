use crate::Cli;
use crate::commands::cst::root_from_green;
use crate::error::CliError;
use crate::input::resolve_inputs;
use crate::output::OutputEnvelope;
use compactp_ast::{
    AstNode, CircuitDecl, CircuitDef, ContractDecl, EnumDef, Item, LedgerDecl, ModuleDef,
    SourceFile, StructDef, TypeDecl, WitnessDecl,
};
use compactp_parser::{ParseOptions, parse_with};
use serde_json::json;
use std::path::PathBuf;

pub fn run(cli: &Cli, paths: &[PathBuf]) -> Result<i32, CliError> {
    let inputs = resolve_inputs(paths, cli.stdin_filename.as_deref())?;

    for input in inputs {
        let result = parse_with(
            &input.source,
            ParseOptions {
                recover: !cli.no_recover,
                max_errors: cli.max_errors.unwrap_or(256),
            },
        );
        let root = root_from_green(result.green);
        let file = SourceFile::cast(root)
            .ok_or_else(|| CliError::internal("root node was not SOURCE_FILE"))?;

        match cli.format {
            crate::OutputFormat::Human => print!("{}", dump_source_file(&file)),
            crate::OutputFormat::Json => {
                let envelope = OutputEnvelope::new(input.label.clone(), to_json(&file), None);
                crate::output::print_json(&envelope, cli.pretty)?;
            }
        }
    }

    Ok(0)
}

fn dump_source_file(file: &SourceFile) -> String {
    let mut out = String::from("SourceFile\n");
    for item in file.items() {
        out.push_str(&format!("  {}\n", item_summary(&item)));
    }
    out
}

fn item_summary(item: &Item) -> String {
    match item {
        Item::Pragma(_) => "Pragma".to_string(),
        Item::Include(_) => "Include".to_string(),
        Item::Import(_) => "Import".to_string(),
        Item::ExportList(_) => "ExportList".to_string(),
        Item::LedgerDecl(n) => format!("LedgerDecl name={}", text(n.name())),
        Item::ConstructorDef(_) => "ConstructorDef".to_string(),
        Item::CircuitDef(n) => format!(
            "CircuitDef name={} exported={} pure={}",
            text(n.name()),
            n.is_exported(),
            n.is_pure()
        ),
        Item::CircuitDecl(n) => format!("CircuitDecl name={}", text(n.name())),
        Item::WitnessDecl(n) => format!("WitnessDecl name={}", text(n.name())),
        Item::ContractDecl(n) => format!("ContractDecl name={}", text(n.name())),
        Item::StructDef(n) => format!("StructDef name={}", text(n.name())),
        Item::EnumDef(n) => format!("EnumDef name={}", text(n.name())),
        Item::ModuleDef(n) => format!("ModuleDef name={}", text(n.name())),
        Item::TypeDecl(n) => format!(
            "TypeDecl name={} exported={} new={}",
            text(n.name()),
            n.is_exported(),
            n.is_newtype()
        ),
    }
}

fn to_json(file: &SourceFile) -> serde_json::Value {
    json!({
        "kind": "SourceFile",
        "items": file.items().map(item_json).collect::<Vec<_>>(),
    })
}

fn item_json(item: Item) -> serde_json::Value {
    match item {
        Item::Pragma(_) => json!({ "kind": "Pragma" }),
        Item::Include(_) => json!({ "kind": "Include" }),
        Item::Import(_) => json!({ "kind": "Import" }),
        Item::ExportList(_) => json!({ "kind": "ExportList" }),
        Item::LedgerDecl(n) => ledger_json(&n),
        Item::ConstructorDef(_) => json!({ "kind": "ConstructorDef" }),
        Item::CircuitDef(n) => circuit_def_json(&n),
        Item::CircuitDecl(n) => circuit_decl_json(&n),
        Item::WitnessDecl(n) => witness_json(&n),
        Item::ContractDecl(n) => contract_json(&n),
        Item::StructDef(n) => struct_json(&n),
        Item::EnumDef(n) => enum_json(&n),
        Item::ModuleDef(n) => module_json(&n),
        Item::TypeDecl(n) => type_decl_json(&n),
    }
}

fn ledger_json(n: &LedgerDecl) -> serde_json::Value {
    json!({
        "kind": "LedgerDecl",
        "name": text(n.name()),
        "exported": n.is_exported(),
        "sealed": n.is_sealed(),
    })
}

fn circuit_def_json(n: &CircuitDef) -> serde_json::Value {
    json!({
        "kind": "CircuitDef",
        "name": text(n.name()),
        "exported": n.is_exported(),
        "pure": n.is_pure(),
        "has_body": n.body().is_some(),
    })
}

fn circuit_decl_json(n: &CircuitDecl) -> serde_json::Value {
    json!({
        "kind": "CircuitDecl",
        "name": text(n.name()),
        "exported": n.is_exported(),
    })
}

fn witness_json(n: &WitnessDecl) -> serde_json::Value {
    json!({
        "kind": "WitnessDecl",
        "name": text(n.name()),
        "exported": n.is_exported(),
    })
}

fn contract_json(n: &ContractDecl) -> serde_json::Value {
    json!({
        "kind": "ContractDecl",
        "name": text(n.name()),
        "exported": n.is_exported(),
        "circuits": n.circuits().map(|c| text(c.name())).collect::<Vec<_>>(),
    })
}

fn struct_json(n: &StructDef) -> serde_json::Value {
    json!({
        "kind": "StructDef",
        "name": text(n.name()),
        "exported": n.is_exported(),
        "fields": n.fields().map(|f| text(f.name())).collect::<Vec<_>>(),
    })
}

fn enum_json(n: &EnumDef) -> serde_json::Value {
    json!({
        "kind": "EnumDef",
        "name": text(n.name()),
        "exported": n.is_exported(),
        "variants": n.variants().map(|v| text(v.name())).collect::<Vec<_>>(),
    })
}

fn module_json(n: &ModuleDef) -> serde_json::Value {
    json!({
        "kind": "ModuleDef",
        "name": text(n.name()),
        "exported": n.is_exported(),
    })
}

fn type_decl_json(n: &TypeDecl) -> serde_json::Value {
    json!({
        "kind": "TypeDecl",
        "name": text(n.name()),
        "exported": n.is_exported(),
        "new": n.is_newtype(),
        "has_generic_params": n.generic_params().is_some(),
    })
}

fn text(token: Option<compactp_ast::SyntaxToken>) -> String {
    token.map(|t| t.text().to_string()).unwrap_or_default()
}
