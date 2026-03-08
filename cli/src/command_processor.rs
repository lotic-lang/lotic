use {
    anyhow::{Context, Result},
    camino::Utf8PathBuf,
    cargo_metadata::MetadataCommand,
    serde::Serialize,
    std::process::Stdio,
    syn::{Attribute, GenericArgument, PathArguments, Type, TypePath},
    walkdir::WalkDir,
};

#[derive(Serialize)]
struct InstructionFn {
    ix_name: String,
    ix_args: Vec<String>,
}

pub fn run_build(manifest_path: Utf8PathBuf, cargo_args: Vec<String>) -> Result<()> {
    if !manifest_path.exists() {
        anyhow::bail!("Manifest file not found: {}", manifest_path);
    }

    let absolute_manifest = manifest_path
        .canonicalize_utf8()
        .context("Failed to canonicalize manifest path")?;

    let rust_files = collect_workspace_rust_files(&absolute_manifest)?;
    let instructions = collect_instruction_functions(&rust_files)?;
    let json = serde_json::to_string_pretty(&instructions)?;

    let metadata = MetadataCommand::new()
        .manifest_path(&absolute_manifest)
        .exec()?;
    let target_dir = metadata.target_directory.clone();
    let output_path = target_dir.join("instructions.json");

    std::fs::write(&output_path, &json)?;
    let exit = std::process::Command::new("cargo")
        .args(&["build-sbf"])
        .args(cargo_args.clone())
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

    let workspace_root = metadata.workspace_root.clone();
    let mut rust_files = Vec::new();

    let entries = WalkDir::new(workspace_root.as_std_path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        // Skip target/ and tests/
        .filter(|e| {
            !e.path().components().any(|c| {
                let name = c.as_os_str();
                name == "target" || name == "tests"
            })
        })
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
        .filter_map(|e| Utf8PathBuf::from_path_buf(e.path().to_path_buf()).ok());

    rust_files.extend(entries);

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
