use leptos::serde_json;
use leptos::{IntoView, WasmSplitManifest, component, config::LeptosOptions, prelude::*, view};
use log::error;
use std::collections::HashMap;
use std::fmt::Write;
use std::{path::PathBuf, sync::OnceLock};

/// Inserts hydration scripts that add interactivity to your server-rendered HTML.
///
/// This should be included in the `<head>` of your application shell.
#[allow(dead_code)]
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "LeptosOptions should be owned"
)]
pub fn HydrationScripts(
    /// Configuration options for this project.
    options: LeptosOptions,
    /// Should be `true` to hydrate in `islands` mode.
    #[prop(optional)]
    islands: bool,
    /// Should be `true` to add the “islands router,” which enables limited client-side routing
    /// when running in islands mode.
    #[prop(optional)]
    islands_router: bool,
    /// A base url, not including a trailing slash
    #[prop(optional, into)]
    root: Option<String>,
) -> impl IntoView {
    static SPLIT_MANIFEST: OnceLock<Option<WasmSplitManifest>> = OnceLock::new();

    if let Some(splits) = SPLIT_MANIFEST.get_or_init(|| {
        let root = root.clone().unwrap_or_default();

        let site_dir = &options.site_root;
        let pkg_dir = &options.site_pkg_dir;
        let path = PathBuf::from(site_dir.to_string());
        let path = path
            .join(pkg_dir.to_string())
            .join("__wasm_split_manifest.json");
        let file = std::fs::read_to_string(path).ok()?;
        let manifest = WasmSplitManifest(ArcStoredValue::new((
            format!("{root}/{pkg_dir}"),
            serde_json::from_str(&file).unwrap_or_else(|e| {
                error!("could not read manifest file: {e}");
                HashMap::new()
            }),
            "__wasm_split_manifest.json".to_string(),
        )));

        Some(manifest)
    }) {
        provide_context(splits.clone());
    }

    let mut js_file_name = options.output_name.to_string();
    let mut wasm_file_name = options.output_name.to_string();
    if options.hash_files {
        if let Err(e) =
            append_filename_hashes(&mut js_file_name, &mut wasm_file_name, &options.hash_file)
        {
            leptos::logging::error!(
                "File hashing is active but could not build file names with hashes: {e}"
            );
        }
    } else if std::option_env!("LEPTOS_OUTPUT_NAME").is_none() {
        wasm_file_name.push_str("_bg");
    }

    let pkg_path = &options.site_pkg_dir;
    #[cfg(feature = "nonce")]
    let nonce = leptos::nonce::use_nonce();
    #[cfg(not(feature = "nonce"))]
    let nonce = None::<String>;
    let script = if islands {
        if let Some(sc) = Owner::current_shared_context() {
            sc.set_is_hydrating(false);
        }
        include_str!("./assets/scripts/island_script.js")
    } else {
        include_str!("./assets/scripts/hydration_script.js")
    };

    let islands_router = islands_router
        .then_some(include_str!("./assets/scripts/islands_routing.js"))
        .unwrap_or_default();

    let root = root.unwrap_or_default();
    view! {
        <link
            rel="modulepreload"
            href=format!("{root}/{pkg_path}/{js_file_name}.js")
            crossorigin=nonce.clone()
        />
        <link
            rel="preload"
            href=format!("{root}/{pkg_path}/{wasm_file_name}.wasm")
            r#as="fetch"
            r#type="application/wasm"
            crossorigin=nonce.clone().unwrap_or_default()
        />
        <script type="module" nonce=nonce>
            {format!(
                "{script}({root:?}, {pkg_path:?}, {js_file_name:?}, {wasm_file_name:?});{islands_router}",
            )}
        </script>
    }
}

fn append_filename_hashes(
    js_file_name: &mut String,
    wasm_file_name: &mut String,
    hash_file: impl AsRef<str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let hash_path = std::env::current_exe()?
        .parent()
        .ok_or("current executable does not have parent directory")?
        .to_path_buf()
        .join(hash_file.as_ref());
    let hashes = std::fs::read_to_string(&hash_path)?;
    for line in hashes.lines() {
        let line = line.trim();
        if !line.is_empty()
            && let Some((file, hash)) = line.split_once(':')
        {
            if file == "js" {
                write!(js_file_name, ".{}", hash.trim())?;
            } else if file == "wasm" {
                write!(wasm_file_name, ".{}", hash.trim())?;
            }
        }
    }
    Ok(())
}
