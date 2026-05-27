use crate::Cli;
use crate::commands::cst::root_from_green;
use crate::error::CliError;
use crate::input::resolve_inputs;
use crate::output::OutputEnvelope;
use compactp_ast::{
    AstNode, Block, CircuitDecl, CircuitDef, ConstructorDef, ContractDecl, EnumDef, Item,
    LedgerDecl, ModuleDef, Param, Pat, SourceFile, Stmt, StructDef, Type, TypeDecl, WitnessDecl,
    expr::{Expr, NameExpr, StructFieldInit},
};
use compactp_parser::{ParseOptions, parse_with};
use serde_json::json;
use std::path::PathBuf;

pub fn run(cli: &Cli, paths: &[PathBuf], include_bodies: bool) -> Result<i32, CliError> {
    let inputs = resolve_inputs(paths, cli.stdin_filename.as_deref())?;

    for input in inputs {
        let result = parse_with(
            &input.source,
            ParseOptions {
                recover: !cli.no_recover,
                max_errors: cli.max_errors.unwrap_or(256),
                ..ParseOptions::default()
            },
        );
        let root = root_from_green(result.green);
        let file = SourceFile::cast(root)
            .ok_or_else(|| CliError::internal("root node was not SOURCE_FILE"))?;

        match cli.format {
            crate::OutputFormat::Human => print!("{}", dump_source_file(&file, include_bodies)),
            crate::OutputFormat::Json => {
                let envelope =
                    OutputEnvelope::new(input.label.clone(), to_json(&file, include_bodies), None);
                crate::output::print_json(&envelope, cli.pretty)?;
            }
        }
    }

    Ok(0)
}

fn dump_source_file(file: &SourceFile, include_bodies: bool) -> String {
    let mut out = String::from("SourceFile\n");
    for item in file.items() {
        out.push_str(&format!("  {}\n", item_summary(&item)));
        if include_bodies {
            dump_item_body(&item, 2, &mut out);
        }
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

fn to_json(file: &SourceFile, include_bodies: bool) -> serde_json::Value {
    json!({
        "kind": "SourceFile",
        "items": file
            .items()
            .map(|item| item_json(item, include_bodies))
            .collect::<Vec<_>>(),
    })
}

fn item_json(item: Item, include_bodies: bool) -> serde_json::Value {
    match item {
        Item::Pragma(_) => json!({ "kind": "Pragma" }),
        Item::Include(_) => json!({ "kind": "Include" }),
        Item::Import(_) => json!({ "kind": "Import" }),
        Item::ExportList(_) => json!({ "kind": "ExportList" }),
        Item::LedgerDecl(n) => ledger_json(&n),
        Item::ConstructorDef(n) => constructor_json(&n, include_bodies),
        Item::CircuitDef(n) => circuit_def_json(&n, include_bodies),
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

fn constructor_json(n: &ConstructorDef, include_bodies: bool) -> serde_json::Value {
    if !include_bodies {
        return json!({ "kind": "ConstructorDef" });
    }
    let mut v = json!({
        "kind": "ConstructorDef",
        "params": n.params().map(param_json).collect::<Vec<_>>(),
        "has_body": n.body().is_some(),
    });
    if let Some(body) = n.body() {
        v["body"] = block_json(&body);
    }
    v
}

fn circuit_def_json(n: &CircuitDef, include_bodies: bool) -> serde_json::Value {
    let mut v = json!({
        "kind": "CircuitDef",
        "name": text(n.name()),
        "exported": n.is_exported(),
        "pure": n.is_pure(),
        "has_body": n.body().is_some(),
    });
    if include_bodies {
        v["params"] = json!(n.params().map(param_json).collect::<Vec<_>>());
        if let Some(rt) = n.return_type() {
            v["return_type"] = type_json(&rt);
        }
        if let Some(body) = n.body() {
            v["body"] = block_json(&body);
        }
    }
    v
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

// ---------------------------------------------------------------------------
// Deep walk: --include-bodies
// ---------------------------------------------------------------------------

fn indent(depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn dump_item_body(item: &Item, depth: usize, out: &mut String) {
    match item {
        Item::ConstructorDef(n) => {
            for p in n.params() {
                indent(depth, out);
                out.push_str(&format!("Param {}\n", param_summary(&p)));
            }
            if let Some(body) = n.body() {
                dump_block(&body, depth, out);
            }
        }
        Item::CircuitDef(n) => {
            for p in n.params() {
                indent(depth, out);
                out.push_str(&format!("Param {}\n", param_summary(&p)));
            }
            if let Some(rt) = n.return_type() {
                indent(depth, out);
                out.push_str(&format!("ReturnType {}\n", type_summary(&rt)));
            }
            if let Some(body) = n.body() {
                dump_block(&body, depth, out);
            }
        }
        _ => {}
    }
}

fn dump_block(block: &Block, depth: usize, out: &mut String) {
    indent(depth, out);
    out.push_str("Block\n");
    for stmt in block.stmts() {
        dump_stmt(&stmt, depth + 1, out);
    }
}

fn dump_stmt(stmt: &Stmt, depth: usize, out: &mut String) {
    indent(depth, out);
    match stmt {
        Stmt::Block(b) => {
            out.push_str("Stmt::Block\n");
            for s in b.stmts() {
                dump_stmt(&s, depth + 1, out);
            }
        }
        Stmt::Assign(a) => {
            out.push_str(&format!("Stmt::Assign op={}\n", text(a.op())));
            dump_expr_descendants(a.syntax(), depth + 1, out);
        }
        Stmt::Const(c) => {
            out.push_str("Stmt::Const\n");
            if let Some(p) = c.pattern() {
                dump_pat(&p, depth + 1, out);
            }
            if let Some(t) = c.ty() {
                indent(depth + 1, out);
                out.push_str(&format!("Type {}\n", type_summary(&t)));
            }
            if let Some(e) = c.value() {
                dump_expr(&e, depth + 1, out);
            }
        }
        Stmt::MultiConst(_) => {
            out.push_str("Stmt::MultiConst\n");
        }
        Stmt::Expr(e) => {
            out.push_str("Stmt::Expr\n");
            dump_expr_descendants(e.syntax(), depth + 1, out);
        }
        Stmt::Return(r) => {
            out.push_str("Stmt::Return\n");
            if let Some(e) = r.value() {
                dump_expr(&e, depth + 1, out);
            }
        }
        Stmt::If(i) => {
            out.push_str(&format!("Stmt::If has_else={}\n", i.else_kw().is_some()));
            if let Some(b) = i.then_branch() {
                dump_block(&b, depth + 1, out);
            }
        }
        Stmt::For(f) => {
            out.push_str(&format!("Stmt::For var={}\n", text(f.var_name())));
            if let Some(b) = f.body() {
                dump_block(&b, depth + 1, out);
            }
        }
        Stmt::Assert(a) => {
            out.push_str(&format!("Stmt::Assert message={}\n", text(a.message())));
        }
    }
}

fn dump_pat(pat: &Pat, depth: usize, out: &mut String) {
    indent(depth, out);
    match pat {
        Pat::Ident(i) => {
            out.push_str(&format!("Pat::Ident name={}\n", text(i.name())));
        }
        Pat::Tuple(t) => {
            out.push_str("Pat::Tuple\n");
            for elt in t.elements() {
                if let Some(inner) = elt.pattern() {
                    dump_pat(&inner, depth + 1, out);
                }
            }
        }
        Pat::Struct(s) => {
            out.push_str("Pat::Struct\n");
            for f in s.fields() {
                indent(depth + 1, out);
                out.push_str(&format!("Field name={}\n", text(f.name())));
                if let Some(inner) = f.pattern() {
                    dump_pat(&inner, depth + 2, out);
                }
            }
        }
    }
}

/// Print every Expr-shaped descendant of a node, one per line.
///
/// Used for statement kinds whose CST stores the expression(s) as descendants
/// rather than as a direct `Expr` accessor on the typed node.
fn dump_expr_descendants(node: &compactp_syntax::SyntaxNode, depth: usize, out: &mut String) {
    for n in node.descendants() {
        if let Some(e) = Expr::cast(n) {
            dump_expr(&e, depth, out);
        }
    }
}

fn dump_expr(expr: &Expr, depth: usize, out: &mut String) {
    indent(depth, out);
    match expr {
        Expr::Literal(_) => out.push_str("Expr::Literal\n"),
        Expr::Name(n) => {
            out.push_str(&format!("Expr::Name {}\n", text(NameExpr::ident(n))));
        }
        Expr::Ternary(t) => {
            out.push_str(&format!(
                "Expr::Ternary has_question={}\n",
                t.question().is_some()
            ));
        }
        Expr::Binary(b) => {
            out.push_str(&format!("Expr::Binary op={}\n", text(b.op())));
        }
        Expr::Unary(u) => {
            out.push_str(&format!("Expr::Unary op={}\n", text(u.op())));
        }
        Expr::Cast(c) => {
            out.push_str("Expr::Cast\n");
            if let Some(t) = c.ty() {
                indent(depth + 1, out);
                out.push_str(&format!("Type {}\n", type_summary(&t)));
            }
        }
        Expr::Call(c) => {
            out.push_str(&format!("Expr::Call name={}\n", text(c.name())));
        }
        Expr::Member(m) => {
            out.push_str(&format!("Expr::Member field={}\n", text(m.field())));
        }
        Expr::Index(_) => out.push_str("Expr::Index\n"),
        Expr::Array(_) => out.push_str("Expr::Array\n"),
        Expr::Bytes(_) => out.push_str("Expr::Bytes\n"),
        Expr::Spread(_) => out.push_str("Expr::Spread\n"),
        Expr::Struct(s) => {
            out.push_str(&format!(
                "Expr::Struct name={} field_count={}\n",
                text(s.name()),
                s.field_inits().count()
            ));
        }
        Expr::Default(d) => {
            out.push_str("Expr::Default\n");
            if let Some(t) = d.ty() {
                indent(depth + 1, out);
                out.push_str(&format!("Type {}\n", type_summary(&t)));
            }
        }
        Expr::Map(_) => out.push_str("Expr::Map\n"),
        Expr::Fold(_) => out.push_str("Expr::Fold\n"),
        Expr::Disclose(_) => out.push_str("Expr::Disclose\n"),
        Expr::Pad(_) => out.push_str("Expr::Pad\n"),
        Expr::Slice(_) => out.push_str("Expr::Slice\n"),
        Expr::Lambda(l) => {
            out.push_str(&format!(
                "Expr::Lambda has_block_body={}\n",
                l.body_block().is_some()
            ));
            if let Some(b) = l.body_block() {
                dump_block(&b, depth + 1, out);
            }
        }
        Expr::Paren(_) => out.push_str("Expr::Paren\n"),
    }
}

fn param_summary(p: &Param) -> String {
    let pat = p
        .pattern()
        .map(|pat| pat_one_line(&pat))
        .unwrap_or_else(|| "?".to_string());
    let ty = p
        .ty()
        .map(|t| type_summary(&t))
        .unwrap_or_else(|| "?".to_string());
    format!("{pat}: {ty}")
}

fn param_json(p: Param) -> serde_json::Value {
    json!({
        "pattern": p.pattern().map(|pat| pat_one_line(&pat)),
        "type": p.ty().map(|t| type_summary(&t)),
    })
}

fn pat_one_line(pat: &Pat) -> String {
    match pat {
        Pat::Ident(i) => text(i.name()),
        Pat::Tuple(_) => "<tuple>".to_string(),
        Pat::Struct(_) => "<struct>".to_string(),
    }
}

fn type_summary(t: &Type) -> String {
    match t {
        Type::Ref(r) => format!("Ref({})", text(r.name())),
        Type::Boolean(_) => "Boolean".to_string(),
        Type::Field(_) => "Field".to_string(),
        Type::Uint(u) => format!("Uint<{}>", u.sizes().count()),
        Type::Bytes(_) => "Bytes".to_string(),
        Type::Opaque(o) => format!("Opaque({})", text(o.tag())),
        Type::Vector(_) => "Vector".to_string(),
        Type::Tuple(t) => format!("Tuple<{}>", t.element_types().count()),
        Type::UnsignedInteger(_) => "UnsignedInteger".to_string(),
        Type::Record(_) => "Record".to_string(),
    }
}

fn type_json(t: &Type) -> serde_json::Value {
    serde_json::Value::String(type_summary(t))
}

fn block_json(block: &Block) -> serde_json::Value {
    json!({
        "kind": "Block",
        "stmts": block.stmts().map(stmt_json).collect::<Vec<_>>(),
    })
}

fn stmt_json(stmt: Stmt) -> serde_json::Value {
    match stmt {
        Stmt::Block(b) => json!({ "kind": "Block", "stmts": b.stmts().map(stmt_json).collect::<Vec<_>>() }),
        Stmt::Assign(a) => json!({
            "kind": "Assign",
            "op": text(a.op()),
            "exprs": expr_descendants_json(a.syntax()),
        }),
        Stmt::Const(c) => json!({
            "kind": "Const",
            "pattern": c.pattern().map(|p| pat_json(&p)),
            "type": c.ty().map(|t| type_summary(&t)),
            "value": c.value().map(|e| expr_json(&e)),
        }),
        Stmt::MultiConst(_) => json!({ "kind": "MultiConst" }),
        Stmt::Expr(e) => json!({
            "kind": "Expr",
            "exprs": expr_descendants_json(e.syntax()),
        }),
        Stmt::Return(r) => json!({
            "kind": "Return",
            "value": r.value().map(|e| expr_json(&e)),
        }),
        Stmt::If(i) => json!({
            "kind": "If",
            "has_else": i.else_kw().is_some(),
            "then": i.then_branch().map(|b| block_json(&b)),
        }),
        Stmt::For(f) => json!({
            "kind": "For",
            "var": text(f.var_name()),
            "body": f.body().map(|b| block_json(&b)),
        }),
        Stmt::Assert(a) => json!({
            "kind": "Assert",
            "message": text(a.message()),
        }),
    }
}

fn pat_json(pat: &Pat) -> serde_json::Value {
    match pat {
        Pat::Ident(i) => json!({ "kind": "Ident", "name": text(i.name()) }),
        Pat::Tuple(t) => json!({
            "kind": "Tuple",
            "elements": t.elements().filter_map(|e| e.pattern()).map(|p| pat_json(&p)).collect::<Vec<_>>(),
        }),
        Pat::Struct(s) => json!({
            "kind": "Struct",
            "fields": s.fields().map(|f| json!({
                "name": text(f.name()),
                "pattern": f.pattern().map(|p| pat_json(&p)),
            })).collect::<Vec<_>>(),
        }),
    }
}

fn expr_descendants_json(node: &compactp_syntax::SyntaxNode) -> Vec<serde_json::Value> {
    node.descendants()
        .filter_map(Expr::cast)
        .map(|e| expr_json(&e))
        .collect()
}

fn expr_json(expr: &Expr) -> serde_json::Value {
    match expr {
        Expr::Literal(_) => json!({ "kind": "Literal" }),
        Expr::Name(n) => json!({ "kind": "Name", "ident": text(NameExpr::ident(n)) }),
        Expr::Ternary(t) => json!({ "kind": "Ternary", "has_question": t.question().is_some() }),
        Expr::Binary(b) => json!({ "kind": "Binary", "op": text(b.op()) }),
        Expr::Unary(u) => json!({ "kind": "Unary", "op": text(u.op()) }),
        Expr::Cast(c) => json!({ "kind": "Cast", "type": c.ty().map(|t| type_summary(&t)) }),
        Expr::Call(c) => json!({
            "kind": "Call",
            "name": text(c.name()),
            "has_generic_args": c.generic_args().is_some(),
        }),
        Expr::Member(m) => json!({ "kind": "Member", "field": text(m.field()) }),
        Expr::Index(_) => json!({ "kind": "Index" }),
        Expr::Array(_) => json!({ "kind": "Array" }),
        Expr::Bytes(_) => json!({ "kind": "Bytes" }),
        Expr::Spread(_) => json!({ "kind": "Spread" }),
        Expr::Struct(s) => json!({
            "kind": "Struct",
            "name": text(s.name()),
            "field_inits": s.field_inits().map(|f| text(StructFieldInit::name(&f))).collect::<Vec<_>>(),
            "has_update": s.update().is_some(),
        }),
        Expr::Default(d) => json!({ "kind": "Default", "type": d.ty().map(|t| type_summary(&t)) }),
        Expr::Map(_) => json!({ "kind": "Map" }),
        Expr::Fold(_) => json!({ "kind": "Fold" }),
        Expr::Disclose(_) => json!({ "kind": "Disclose" }),
        Expr::Pad(_) => json!({ "kind": "Pad" }),
        Expr::Slice(s) => json!({
            "kind": "Slice",
            "has_generic_args": s.generic_args().is_some(),
        }),
        Expr::Lambda(l) => json!({
            "kind": "Lambda",
            "has_param_list": l.param_list().is_some(),
            "has_return_type": l.return_type().is_some(),
            "has_body_block": l.body_block().is_some(),
        }),
        Expr::Paren(_) => json!({ "kind": "Paren" }),
    }
}

