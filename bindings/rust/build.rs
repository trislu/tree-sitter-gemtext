fn main() {
    #[cfg(feature = "dev")]
    {
        let tree_sitter_available = std::process::Command::new("tree-sitter")
            .arg("--version")
            .status()
            .is_ok();

        if tree_sitter_available {
            let output = std::process::Command::new("tree-sitter")
                .arg("generate")
                .arg("--abi=15")
                .output()
                .expect("Failed to execute tree-sitter build command");

            if !output.status.success() {
                let error_message = String::from_utf8_lossy(&output.stderr);
                panic!("Tree-sitter build failed: {error_message}");
            }
        }

        use codegen::Scope;
        use convert_case::{Case, Casing};
        use serde::{
            Deserialize, Serialize,
            de::{DeserializeOwned, Error},
        };
        use serde_json;
        use std::{
            fs::{File, read_to_string},
            io::Write,
            path::Path,
        };

        #[derive(Serialize, Deserialize)]
        struct NodeType {
            #[serde(alias = "type")]
            type_name: String,
            named: bool,
            root: Option<bool>,
        }

        // rust codegen
        fn read_to_json<T>(path: &Path) -> Result<T, impl Error>
        where
            T: DeserializeOwned,
        {
            let json_str =
                read_to_string(path).unwrap_or_else(|_| panic!("Failed to read file: {:?}", path));
            serde_json::from_str::<T>(&json_str)
        }

        let nodetype_json_path = Path::new("src/node-types.json");
        println!(
            "cargo:rerun-if-changed={}",
            nodetype_json_path.to_str().unwrap()
        );

        let nodetype_json: Result<Vec<NodeType>, _> = read_to_json(nodetype_json_path);

        let mut token_scope = Scope::new();
        token_scope.import("strum_macros", "Display");
        token_scope.import("strum_macros", "EnumCount");
        token_scope.import("strum_macros", "EnumIter");
        token_scope.import("strum_macros", "EnumString");

        // enum for Token Types
        let enum_rule = token_scope
            .new_enum("TokenType")
            .vis("pub")
            .derive("Clone")
            .derive("Copy")
            .derive("Debug")
            .derive("Display")
            .derive("EnumCount")
            .derive("EnumIter")
            .derive("EnumString")
            .derive("Eq")
            .derive("Hash")
            .derive("PartialEq");

        for node_type in nodetype_json.unwrap() {
            if let Some(true) = node_type.root {
                // skip the root "source_file"
                continue;
            }
            let name = node_type.type_name;
            enum_rule
                .new_variant(name.to_case(Case::Pascal))
                .annotation(format!(r#"#[strum( serialize = "{}")]"#, name));
        }
        // code gen for token.rs
        let token_rs_path = Path::new("bindings").join("rust").join("token_types.rs");
        let mut token_rs = File::create(token_rs_path).unwrap();
        token_rs
            .write_all(token_scope.to_string().as_bytes())
            .unwrap();
    }

    let src_dir = std::path::Path::new("src");

    let mut c_config = cc::Build::new();
    c_config.std("c11").include(src_dir);

    #[cfg(target_env = "msvc")]
    c_config.flag("-utf-8");

    if std::env::var("TARGET").unwrap() == "wasm32-unknown-unknown" {
        let Ok(wasm_headers) = std::env::var("DEP_TREE_SITTER_LANGUAGE_WASM_HEADERS") else {
            panic!(
                "Environment variable DEP_TREE_SITTER_LANGUAGE_WASM_HEADERS must be set by the language crate"
            );
        };
        let Ok(wasm_src) =
            std::env::var("DEP_TREE_SITTER_LANGUAGE_WASM_SRC").map(std::path::PathBuf::from)
        else {
            panic!(
                "Environment variable DEP_TREE_SITTER_LANGUAGE_WASM_SRC must be set by the language crate"
            );
        };

        c_config.include(&wasm_headers);
        c_config.files([
            wasm_src.join("stdio.c"),
            wasm_src.join("stdlib.c"),
            wasm_src.join("string.c"),
        ]);
    }

    let parser_path = src_dir.join("parser.c");
    c_config.file(&parser_path);
    println!("cargo:rerun-if-changed={}", parser_path.to_str().unwrap());

    let scanner_path = src_dir.join("scanner.c");
    if scanner_path.exists() {
        c_config.file(&scanner_path);
        println!("cargo:rerun-if-changed={}", scanner_path.to_str().unwrap());
    }

    c_config.compile("tree-sitter-gemtext");

    println!("cargo:rustc-check-cfg=cfg(with_highlights_query)");
    if !"queries/highlights.scm".is_empty()
        && std::path::Path::new("queries/highlights.scm").exists()
    {
        println!("cargo:rustc-cfg=with_highlights_query");
    }
    println!("cargo:rustc-check-cfg=cfg(with_injections_query)");
    if !"queries/injections.scm".is_empty()
        && std::path::Path::new("queries/injections.scm").exists()
    {
        println!("cargo:rustc-cfg=with_injections_query");
    }
    println!("cargo:rustc-check-cfg=cfg(with_locals_query)");
    if !"queries/locals.scm".is_empty() && std::path::Path::new("queries/locals.scm").exists() {
        println!("cargo:rustc-cfg=with_locals_query");
    }
    println!("cargo:rustc-check-cfg=cfg(with_tags_query)");
    if !"queries/tags.scm".is_empty() && std::path::Path::new("queries/tags.scm").exists() {
        println!("cargo:rustc-cfg=with_tags_query");
    }
}
