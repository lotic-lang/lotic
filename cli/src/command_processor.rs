use {
    anyhow::{Context, Result},
    camino::Utf8PathBuf,
    cargo_metadata::MetadataCommand,
    heck::ToKebabCase,
    serde::Serialize,
    std::{collections::HashMap, path::Path, process::Stdio},
    syn::{Attribute, Item, Type, TypePath},
    walkdir::WalkDir,
};

#[derive(Serialize)]
#[serde(untagged)]
enum ArgDetail {
    Simple(String),
    Complex {
        name: String,
        fields: Vec<FieldDetail>,
    },
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
    },
}

#[derive(Serialize)]
struct EnumVariant {
    name: String,
    fields: Option<Vec<FieldDetail>>,
    is_named: bool,
}

#[derive(Serialize)]
struct FieldDetail {
    name: String,
    r#type: ArgDetail,
    is_option: bool,
    is_result: bool,
    is_vec: bool,
    is_set: bool,
    is_map: bool,
    key_type: Option<Box<ArgDetail>>,
    array_length: Option<usize>,
    error_type: Option<Box<ArgDetail>>,
}

impl Default for FieldDetail {
    fn default() -> Self {
        FieldDetail {
            name: String::new(),
            r#type: ArgDetail::Simple(String::new()),
            is_option: false,
            is_result: false,
            is_vec: false,
            is_set: false,
            is_map: false,
            key_type: None,
            array_length: None,
            error_type: None,
        }
    }
}

#[derive(Serialize)]
struct InstructionFn {
    ix_name: String,
    ix_args: Vec<FieldDetail>,
}

pub fn run_build(cargo_args: Vec<String>) -> Result<()> {
    let mut manifest_path_raw = None;

    for (i, arg) in cargo_args.iter().enumerate() {
        if arg == "--manifest-path" {
            if let Some(path) = cargo_args.get(i + 1) {
                manifest_path_raw = Some(path.clone());
            }
            break;
        } else if arg.starts_with("--manifest-path=") {
            if let Some((_, path)) = arg.split_once('=') {
                manifest_path_raw = Some(path.to_string());
            }
            break;
        }
    }

    let manifest_path = match manifest_path_raw {
        Some(p) => Utf8PathBuf::from(p),
        None => std::env::current_dir()?.join("Cargo.toml").try_into()?,
    };

    if !manifest_path.exists() {
        anyhow::bail!("Manifest file not found: {}", manifest_path);
    }

    let absolute_manifest_path = manifest_path
        .canonicalize_utf8()
        .context("Failed to canonicalize manifest path")?;

    let rust_files = collect_workspace_rust_files(&absolute_manifest_path)?;
    let instructions = collect_instruction_functions(&rust_files)?;
    let json = serde_json::to_string_pretty(&instructions)?;

    let metadata = MetadataCommand::new()
        .manifest_path(&absolute_manifest_path)
        .exec()?;

    let package_name = package_name_from_manifest(&absolute_manifest_path)?;
    let file_name = format!("{package_name}-instructions.json");
    let output_path = metadata.target_directory.join(&file_name);

    std::fs::create_dir_all(&metadata.target_directory)
        .map_err(|e| anyhow::anyhow!("Failed to create target directory: {e}"))?;

    std::fs::write(&output_path, &json)
        .map_err(|e| anyhow::anyhow!("Failed to write `{file_name}`: {e}"))?;

    let exit = std::process::Command::new("cargo")
        .arg("build-sbf")
        .args(&cargo_args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| anyhow::format_err!("{}", e))?;

    std::fs::remove_file(output_path)?;

    if !exit.status.success() {
        std::process::exit(exit.status.code().unwrap_or(1));
    }

    Ok(())
}

fn collect_workspace_rust_files(manifest_path: &Utf8PathBuf) -> Result<Vec<Utf8PathBuf>> {
    let metadata = MetadataCommand::new()
        .manifest_path(manifest_path)
        .exec()
        .with_context(|| format!("Failed to parse metadata for {}", manifest_path))?;

    let mut rust_files = Vec::new();

    // Find the specific package that matches the manifest path
    let package = metadata
        .packages
        .iter()
        .find(|pkg| pkg.manifest_path == *manifest_path)
        .context("Package not found in metadata")?;

    let package_root = package
        .manifest_path
        .parent()
        .context("Package manifest has no parent")?;

    let src_dir = package_root.join("src");

    if src_dir.exists() {
        let entries = WalkDir::new(src_dir.as_std_path())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
            .filter_map(|e| Utf8PathBuf::from_path_buf(e.path().to_path_buf()).ok());

        rust_files.extend(entries);
    }

    rust_files.sort();
    rust_files.dedup();

    Ok(rust_files)
}

fn package_name_from_manifest(path: &Utf8PathBuf) -> Result<String> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read manifest at: {path}"))?;

    let manifest: toml::Value =
        toml::from_str(&content).with_context(|| format!("Invalid TOML format in: {path}"))?;

    manifest["package"]["name"]
        .as_str()
        .map(|s| s.to_string())
        .with_context(|| format!("Could not find [package.name] in: {path}"))
}

fn has_instruction_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("instruction"))
}

fn is_context_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty
        && let Some(seg) = tp.path.segments.last()
    {
        return seg.ident == "Context";
    }
    false
}

fn collect_instruction_functions(rust_files: &[Utf8PathBuf]) -> Result<Vec<InstructionFn>> {
    let mut all_items = Vec::new();

    for file in rust_files {
        let source = std::fs::read_to_string(file)?;
        let syntax = syn::parse_file(&source)?;
        all_items.extend(syntax.items);
    }

    let registry: HashMap<String, &Item> = all_items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(s) => Some((s.ident.to_string(), item)),
            Item::Enum(e) => Some((e.ident.to_string(), item)),
            _ => None,
        })
        .collect();

    let mut instructions = Vec::new();
    for item in &all_items {
        if let Item::Fn(func) = item
            && has_instruction_attr(&func.attrs)
        {
            let ix_args = func
                .sig
                .inputs
                .iter()
                .filter_map(|arg| {
                    if let syn::FnArg::Typed(pat_type) = arg {
                        let arg_name = match &*pat_type.pat {
                            syn::Pat::Ident(id) => id.ident.to_string(),
                            _ => "_".to_string(),
                        };

                        let mut visited = Vec::new();
                        let target_type = &*pat_type.ty;

                        if is_context_type(target_type) {
                            let inner =
                                get_wrapper_inner(target_type, "Context", 0).unwrap_or(target_type);
                            return Some(peel_and_build(arg_name, inner, &registry, &mut visited));
                        }

                        Some(peel_and_build(
                            arg_name,
                            target_type,
                            &registry,
                            &mut visited,
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            instructions.push(InstructionFn {
                ix_name: func.sig.ident.to_string(),
                ix_args,
            });
        }
    }

    instructions.sort_by(|a, b| a.ix_name.cmp(&b.ix_name));
    Ok(instructions)
}

fn peel_and_build(
    arg_name: String,
    ty: &syn::Type,
    registry: &HashMap<String, &Item>,
    visited: &mut Vec<String>,
) -> FieldDetail {
    if let (Some(inner), Some(len)) = get_array_info(ty) {
        return FieldDetail {
            name: arg_name.clone(),
            array_length: Some(len),
            r#type: ArgDetail::Complex {
                name: "Array".to_string(),
                fields: vec![peel_and_build(arg_name, inner, registry, visited)],
            },
            ..FieldDetail::default()
        };
    }

    if let Some(inner) = get_wrapper_inner(ty, "Vec", 0) {
        return FieldDetail {
            name: arg_name.clone(),
            is_vec: true,
            r#type: ArgDetail::Complex {
                name: "Vec".to_string(),
                fields: vec![peel_and_build(arg_name, inner, registry, visited)],
            },
            ..FieldDetail::default()
        };
    }

    if let Some(inner) = get_wrapper_inner(ty, "Option", 0) {
        return FieldDetail {
            name: arg_name.clone(),
            is_option: true,
            r#type: ArgDetail::Complex {
                name: "Option".to_string(),
                fields: vec![peel_and_build(arg_name, inner, registry, visited)],
            },
            ..FieldDetail::default()
        };
    }

    if let Some(inner_ok) = get_wrapper_inner(ty, "Result", 0) {
        let err_ty = get_wrapper_inner(ty, "Result", 1);
        return FieldDetail {
            name: arg_name.clone(),
            is_result: true,
            error_type: err_ty.map(|e| Box::new(resolve_type(e, registry, &mut visited.clone()))),
            r#type: ArgDetail::Complex {
                name: "Result".to_string(),
                fields: vec![peel_and_build(arg_name, inner_ok, registry, visited)],
            },
            ..FieldDetail::default()
        };
    }

    if let Some(inner) =
        get_wrapper_inner(ty, "HashSet", 0).or_else(|| get_wrapper_inner(ty, "BTreeSet", 0))
    {
        return FieldDetail {
            name: arg_name.clone(),
            is_set: true,
            r#type: ArgDetail::Complex {
                name: "Set".to_string(),
                fields: vec![peel_and_build(arg_name, inner, registry, visited)],
            },
            ..FieldDetail::default()
        };
    }

    if let Some(inner_val) =
        get_wrapper_inner(ty, "HashMap", 1).or_else(|| get_wrapper_inner(ty, "BTreeMap", 1))
    {
        let key_ty =
            get_wrapper_inner(ty, "HashMap", 0).or_else(|| get_wrapper_inner(ty, "BTreeMap", 0));

        return FieldDetail {
            name: arg_name.clone(),
            is_map: true,
            key_type: key_ty.map(|k| Box::new(resolve_type(k, registry, &mut visited.clone()))),
            r#type: ArgDetail::Complex {
                name: "Map".to_string(),
                fields: vec![peel_and_build(arg_name, inner_val, registry, visited)],
            },
            ..FieldDetail::default()
        };
    }

    FieldDetail {
        name: arg_name,
        r#type: resolve_type(ty, registry, visited),
        ..FieldDetail::default()
    }
}

fn get_array_info(ty: &syn::Type) -> (Option<&syn::Type>, Option<usize>) {
    if let syn::Type::Array(array) = ty
        && let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(len),
            ..
        }) = &array.len
        && let Ok(n) = len.base10_parse::<usize>()
    {
        return (Some(&*array.elem), Some(n));
    }
    (None, None)
}

fn resolve_type(
    ty: &syn::Type,
    registry: &HashMap<String, &Item>,
    visited: &mut Vec<String>,
) -> ArgDetail {
    let type_name = type_to_simple_string(ty);

    if let Some(item) = registry.get(&type_name) {
        if visited.contains(&type_name) {
            return ArgDetail::Simple(type_name);
        }
        visited.push(type_name.clone());

        let result = match item {
            Item::Struct(s) => {
                let fields = s
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        let name = f
                            .ident
                            .as_ref()
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| i.to_string());
                        peel_and_build(name, &f.ty, registry, visited)
                    })
                    .collect();
                ArgDetail::Complex {
                    name: type_name,
                    fields,
                }
            }
            Item::Enum(e) => {
                let variants = e
                    .variants
                    .iter()
                    .map(|v| {
                        let is_named = matches!(v.fields, syn::Fields::Named(_)); // Fix for is_named
                        let fields = match &v.fields {
                            syn::Fields::Unnamed(f) => Some(
                                f.unnamed
                                    .iter()
                                    .enumerate()
                                    .map(|(i, field)| {
                                        peel_and_build(i.to_string(), &field.ty, registry, visited)
                                    })
                                    .collect(),
                            ),
                            syn::Fields::Named(f) => Some(
                                f.named
                                    .iter()
                                    .map(|field| {
                                        let name = field.ident.as_ref().unwrap().to_string();
                                        peel_and_build(name, &field.ty, registry, visited)
                                    })
                                    .collect(),
                            ),
                            syn::Fields::Unit => None,
                        };
                        EnumVariant {
                            name: v.ident.to_string(),
                            fields,
                            is_named,
                        }
                    })
                    .collect();
                ArgDetail::Enum {
                    name: type_name,
                    variants,
                }
            }
            _ => ArgDetail::Simple(type_name),
        };
        visited.pop();
        result
    } else {
        ArgDetail::Simple(type_name)
    }
}

fn get_wrapper_inner<'a>(ty: &'a syn::Type, name: &str, index: usize) -> Option<&'a syn::Type> {
    if let syn::Type::Path(tp) = ty
        && let Some(seg) = tp.path.segments.last()
        && seg.ident == name
        && let syn::PathArguments::AngleBracketed(args) = &seg.arguments
        && let Some(syn::GenericArgument::Type(inner)) = args.args.get(index)
    {
        return Some(inner);
    }
    None
}

fn type_to_simple_string(ty: &Type) -> String {
    let ty = match ty {
        Type::Reference(r) => &r.elem,
        Type::Paren(p) => &p.elem,
        other => other,
    };

    if let Type::Path(TypePath { path, .. }) = ty
        && let Some(seg) = path.segments.last()
    {
        return seg.ident.to_string();
    }

    "_".to_string()
}

pub fn run_init(project_name: String) -> Result<()> {
    let crate_name = project_name.to_kebab_case();
    let root_path = Path::new(&crate_name);
    let src_path = root_path.join("src");

    if root_path.exists() {
        anyhow::bail!("Directory '{}' already exists", crate_name);
    }

    std::fs::create_dir_all(&src_path)
        .with_context(|| format!("Failed to create directory structure at {src_path:?}"))?;

    let cargo_toml = format!(
        r#"[package]
name = "{crate_name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "lib"]

[dependencies]
lotic = "0.0.1"

[lints.rust.unexpected_cfgs]
level = "warn"
check-cfg = ['cfg(target_os, values("solana"))']
"#
    );

    let lib_rs = r#"use lotic::{
    declare_program, instruction,
    pinocchio::ProgramResult,
    Context, InstructionAccounts,
};

declare_program!("");

#[instruction]
fn initialize(_ctx: Context<Hello>) -> ProgramResult {
    Ok(())
}

#[derive(InstructionAccounts)]
pub struct Hello {}
"#;

    std::fs::write(root_path.join("Cargo.toml"), cargo_toml)
        .context("Failed to write Cargo.toml")?;
    std::fs::write(src_path.join("lib.rs"), lib_rs).context("Failed to write src/lib.rs")?;

    println!("Successfully initialized '{crate_name}'");
    Ok(())
}
