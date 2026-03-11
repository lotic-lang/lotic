use {
    anyhow::{Context, Result},
    camino::Utf8PathBuf,
    cargo_metadata::MetadataCommand,
    heck::ToKebabCase,
    serde::Serialize,
    std::{fs, path::Path, process::Stdio},
    syn::{Attribute, GenericArgument, PathArguments, Type, TypePath},
    walkdir::WalkDir,
};

#[derive(Serialize)]
struct InstructionFn {
    ix_name: String,
    ix_args: Vec<String>,
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
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
            .filter_map(|e| Utf8PathBuf::from_path_buf(e.path().to_path_buf()).ok());

        rust_files.extend(entries);
    }

    rust_files.sort();
    rust_files.dedup();

    Ok(rust_files)
}

fn has_instruction_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("instruction"))
}

fn collect_instruction_functions(rust_files: &[Utf8PathBuf]) -> Result<Vec<InstructionFn>> {
    let mut instructions = Vec::new();

    for file in rust_files {
        let source =
            std::fs::read_to_string(file).with_context(|| format!("Failed to read {}", file))?;
        let syntax =
            syn::parse_file(&source).with_context(|| format!("Failed to parse {}", file))?;

        for item in syntax.items {
            if let syn::Item::Fn(func) = item {
                if has_instruction_attr(&func.attrs) {
                    instructions.push(InstructionFn {
                        ix_name: func.sig.ident.to_string(),
                        ix_args: extract_fn_args(&func),
                    });
                }
            }
        }
    }
    instructions.sort_by(|a, b| a.ix_name.cmp(&b.ix_name));
    Ok(instructions)
}

fn type_to_simple_string(ty: &Type) -> String {
    let ty = match ty {
        Type::Reference(r) => &r.elem,
        Type::Paren(p) => &p.elem,
        other => other,
    };

    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(seg) = path.segments.last() {
            if let PathArguments::AngleBracketed(args) = &seg.arguments {
                for arg in &args.args {
                    if let GenericArgument::Type(Type::Path(TypePath {
                        path: inner_path, ..
                    })) = arg
                    {
                        if let Some(inner_seg) = inner_path.segments.last() {
                            return inner_seg.ident.to_string();
                        }
                    }
                }
            }
            return seg.ident.to_string();
        }
    }

    "_".to_string()
}

fn extract_fn_args(func: &syn::ItemFn) -> Vec<String> {
    func.sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Typed(pat_type) => Some(type_to_simple_string(&pat_type.ty)),
            _ => None,
        })
        .collect()
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

pub fn run_init(project_name: String) -> Result<()> {
    let crate_name = project_name.to_kebab_case();

    let root_path = Path::new(&crate_name);
    let src_path = root_path.join("src");

    if root_path.exists() {
        anyhow::bail!("Directory '{}' already exists", crate_name);
    }

    fs::create_dir_all(&src_path)
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

    fs::write(root_path.join("Cargo.toml"), cargo_toml).context("Failed to write Cargo.toml")?;

    fs::write(src_path.join("lib.rs"), lib_rs).context("Failed to write src/lib.rs")?;

    println!("Successfully initialized '{crate_name}'");
    Ok(())
}
