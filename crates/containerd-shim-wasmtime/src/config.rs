use std::path::PathBuf;
use std::sync::LazyLock;

use anyhow::{Context, Result};
use serde::Deserialize;

/// The environment variable that specifies the path to the TOML config file.
const WASMTIME_CONFIG_ENV: &str = "WASMTIME_CONFIG";

/// Default path for the wasmtime TOML config file.
const DEFAULT_CONFIG_PATH: &str = "/etc/runwasi/wasmtime.toml";

/// Global shared configuration, loaded once from the TOML file.
/// Panics if the config file is specified but cannot be read/parsed.
pub(crate) static WASMTIME_TOML_CONFIG: LazyLock<WasmtimeConfig> =
    LazyLock::new(|| load_config().expect("failed to load wasmtime TOML config"));

/// Top-level wasmtime TOML configuration.
///
/// Follows the upstream wasmtime CLI TOML config format where applicable.
/// All fields are optional — unspecified fields keep the shim's defaults.
///
/// Example:
/// ```toml
/// [wasm]
/// gc = true
/// function-references = true
///
/// [codegen]
/// parallel-compilation = true
/// opt-level = 2
///
/// [optimize]
/// pooling-allocator = true
/// ```
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct WasmtimeConfig {
    #[serde(default)]
    pub wasm: WasmConfig,
    #[serde(default)]
    pub codegen: CodegenConfig,
    #[serde(default)]
    pub optimize: OptimizeConfig,
}

/// Wasm feature flags (`[wasm]` section).
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct WasmConfig {
    pub gc: Option<bool>,
    pub function_references: Option<bool>,
    pub threads: Option<bool>,
    pub simd: Option<bool>,
    pub relaxed_simd: Option<bool>,
    pub bulk_memory: Option<bool>,
    pub multi_value: Option<bool>,
    pub multi_memory: Option<bool>,
    pub memory64: Option<bool>,
    pub tail_call: Option<bool>,
    pub wide_arithmetic: Option<bool>,
    pub custom_page_sizes: Option<bool>,
    pub reference_types: Option<bool>,
    pub extended_const: Option<bool>,
    pub component_model: Option<bool>,
    pub stack_switching: Option<bool>,
}

/// Codegen settings (`[codegen]` section).
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct CodegenConfig {
    pub parallel_compilation: Option<bool>,
    /// Optimization level: 0 = None, 1 = Speed, 2 = SpeedAndSize.
    pub opt_level: Option<u8>,
}

/// Optimization/allocator settings (`[optimize]` section).
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct OptimizeConfig {
    /// If `true`, force enable the pooling allocator.
    /// If `false`, force disable it.
    /// If absent (`None`), auto-detect based on system capabilities.
    pub pooling_allocator: Option<bool>,
}

/// Load wasmtime configuration from a TOML file.
///
/// Discovery order:
/// 1. `WASMTIME_CONFIG` env var — if set, read from that path (error if unreadable/unparseable)
/// 2. Default path (`/etc/runwasi/wasmtime.toml`) — if the file exists, read it (error if unparseable)
/// 3. No config file found — return `WasmtimeConfig::default()`
pub(crate) fn load_config() -> Result<WasmtimeConfig> {
    // 1. Check env var
    if let Ok(p) = std::env::var(WASMTIME_CONFIG_ENV) {
        if !p.is_empty() {
            let path = PathBuf::from(p);
            log::info!("Loading wasmtime config from {}", path.display());
            return load_config_from_path(&path);
        }
    }

    // 2. Check default path
    let default_path = PathBuf::from(DEFAULT_CONFIG_PATH);
    if default_path.exists() {
        log::info!(
            "Loading wasmtime config from default path {}",
            default_path.display()
        );
        return load_config_from_path(&default_path);
    }

    // 3. No config file found
    log::debug!(
        "No {WASMTIME_CONFIG_ENV} env var set and {DEFAULT_CONFIG_PATH} not found, using default config"
    );
    Ok(WasmtimeConfig::default())
}

fn load_config_from_path(path: &PathBuf) -> Result<WasmtimeConfig> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read wasmtime config from {}", path.display()))?;

    let config: WasmtimeConfig = toml::from_str(&contents)
        .with_context(|| format!("failed to parse wasmtime config from {}", path.display()))?;

    log::debug!("Loaded wasmtime config: {config:?}");
    Ok(config)
}

impl WasmtimeConfig {
    /// Apply the loaded TOML configuration onto a `wasmtime::Config`.
    ///
    /// Only sets values that are explicitly specified in the TOML file.
    pub(crate) fn apply(&self, config: &mut wasmtime::Config) {
        // [wasm] section
        if let Some(v) = self.wasm.gc {
            config.wasm_gc(v);
        }
        if let Some(v) = self.wasm.function_references {
            config.wasm_function_references(v);
        }
        if let Some(v) = self.wasm.threads {
            config.wasm_threads(v);
        }
        if let Some(v) = self.wasm.simd {
            config.wasm_simd(v);
        }
        if let Some(v) = self.wasm.relaxed_simd {
            config.wasm_relaxed_simd(v);
        }
        if let Some(v) = self.wasm.bulk_memory {
            config.wasm_bulk_memory(v);
        }
        if let Some(v) = self.wasm.multi_value {
            config.wasm_multi_value(v);
        }
        if let Some(v) = self.wasm.multi_memory {
            config.wasm_multi_memory(v);
        }
        if let Some(v) = self.wasm.memory64 {
            config.wasm_memory64(v);
        }
        if let Some(v) = self.wasm.tail_call {
            config.wasm_tail_call(v);
        }
        if let Some(v) = self.wasm.wide_arithmetic {
            config.wasm_wide_arithmetic(v);
        }
        if let Some(v) = self.wasm.custom_page_sizes {
            config.wasm_custom_page_sizes(v);
        }
        if let Some(v) = self.wasm.reference_types {
            config.wasm_reference_types(v);
        }
        if let Some(v) = self.wasm.extended_const {
            config.wasm_extended_const(v);
        }
        if let Some(v) = self.wasm.component_model {
            if !v {
                log::warn!(
                    "component-model is explicitly disabled in config; \
                     this will prevent running WASI components"
                );
            }
            config.wasm_component_model(v);
        }
        if let Some(v) = self.wasm.stack_switching {
            config.wasm_stack_switching(v);
        }

        // [codegen] section
        if let Some(v) = self.codegen.parallel_compilation {
            config.parallel_compilation(v);
        }
        if let Some(level) = self.codegen.opt_level {
            let opt = match level {
                0 => wasmtime::OptLevel::None,
                1 => wasmtime::OptLevel::Speed,
                _ => wasmtime::OptLevel::SpeedAndSize,
            };
            config.cranelift_opt_level(opt);
        }
    }
}

/// Create a `wasmtime::Config` from the loaded TOML configuration.
///
/// The `include_pooling` parameter controls whether the pooling allocator
/// is configured. Pass `true` for the sandbox engine, `false` for the compiler engine.
pub(crate) fn create_engine_config(
    toml_config: &WasmtimeConfig,
    include_pooling: bool,
    use_pooling_allocator_by_default: impl FnOnce() -> bool,
) -> wasmtime::Config {
    let mut config = wasmtime::Config::new();

    // Step 1: Shim defaults
    // Disable Wasmtime parallel compilation for the tests
    // see https://github.com/containerd/runwasi/pull/405#issuecomment-1928468714 for details
    config.parallel_compilation(!cfg!(test));
    config.wasm_component_model(true); // enable component linking

    // Step 2: Pooling allocator (sandbox only)
    if include_pooling {
        let use_pooling = toml_config
            .optimize
            .pooling_allocator
            .unwrap_or_else(use_pooling_allocator_by_default);
        if use_pooling {
            let cfg = wasmtime::PoolingAllocationConfig::default();
            config.allocation_strategy(wasmtime::InstanceAllocationStrategy::Pooling(cfg));
        }
    }

    // Step 3: TOML overrides
    toml_config.apply(&mut config);

    // Step 4: Forced invariant
    config.async_support(true); // must be on

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_config_is_default() {
        let config: WasmtimeConfig = toml::from_str("").unwrap();
        assert_eq!(config, WasmtimeConfig::default());
    }

    #[test]
    fn test_partial_wasm_section() {
        let toml = r#"
            [wasm]
            gc = true
            function-references = true
        "#;
        let config: WasmtimeConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.wasm.gc, Some(true));
        assert_eq!(config.wasm.function_references, Some(true));
        assert_eq!(config.wasm.threads, None);
        assert_eq!(config.wasm.simd, None);
    }

    #[test]
    fn test_full_config() {
        let toml = r#"
            [wasm]
            gc = true
            function-references = true
            component-model = true
            threads = false

            [codegen]
            parallel-compilation = false
            opt-level = 1

            [optimize]
            pooling-allocator = true
        "#;
        let config: WasmtimeConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.wasm.gc, Some(true));
        assert_eq!(config.wasm.function_references, Some(true));
        assert_eq!(config.wasm.component_model, Some(true));
        assert_eq!(config.wasm.threads, Some(false));
        assert_eq!(config.codegen.parallel_compilation, Some(false));
        assert_eq!(config.codegen.opt_level, Some(1));
        assert_eq!(config.optimize.pooling_allocator, Some(true));
    }

    #[test]
    fn test_unknown_field_rejected() {
        let toml = r#"
            [wasm]
            unknown-feature = true
        "#;
        let result: Result<WasmtimeConfig, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_section_rejected() {
        let toml = r#"
            [unknown]
            foo = true
        "#;
        let result: Result<WasmtimeConfig, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_kebab_case_naming() {
        let toml = r#"
            [wasm]
            function-references = true
            relaxed-simd = true
            wide-arithmetic = true
            custom-page-sizes = false
            bulk-memory = true
            multi-value = true
            multi-memory = false
            tail-call = true
            reference-types = true
            extended-const = true
            stack-switching = false
        "#;
        let config: WasmtimeConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.wasm.function_references, Some(true));
        assert_eq!(config.wasm.relaxed_simd, Some(true));
        assert_eq!(config.wasm.wide_arithmetic, Some(true));
        assert_eq!(config.wasm.custom_page_sizes, Some(false));
        assert_eq!(config.wasm.bulk_memory, Some(true));
        assert_eq!(config.wasm.multi_value, Some(true));
        assert_eq!(config.wasm.multi_memory, Some(false));
        assert_eq!(config.wasm.tail_call, Some(true));
        assert_eq!(config.wasm.reference_types, Some(true));
        assert_eq!(config.wasm.extended_const, Some(true));
        assert_eq!(config.wasm.stack_switching, Some(false));
    }

    #[test]
    fn test_apply_default_produces_valid_engine() {
        let config = WasmtimeConfig::default();
        let mut wt_config = wasmtime::Config::new();
        config.apply(&mut wt_config);
        wt_config.async_support(true);
        wasmtime::Engine::new(&wt_config).expect("engine creation should succeed");
    }

    #[test]
    fn test_apply_gc_and_function_refs() {
        let toml_str = r#"
            [wasm]
            gc = true
            function-references = true
        "#;
        let config: WasmtimeConfig = toml::from_str(toml_str).unwrap();
        let mut wt_config = wasmtime::Config::new();
        wt_config.async_support(true);
        config.apply(&mut wt_config);
        wasmtime::Engine::new(&wt_config).expect("engine should support gc + function-references");
    }

    #[test]
    fn test_create_engine_config_defaults() {
        let config = WasmtimeConfig::default();
        let wt_config = create_engine_config(&config, false, || false);
        wasmtime::Engine::new(&wt_config).expect("engine creation should succeed");
    }

    #[test]
    fn test_create_engine_config_with_pooling_auto_detect_false() {
        let config = WasmtimeConfig::default();
        let wt_config = create_engine_config(&config, true, || false);
        wasmtime::Engine::new(&wt_config).expect("engine creation should succeed");
    }

    #[test]
    fn test_create_engine_config_toml_overrides() {
        let toml_str = r#"
            [wasm]
            gc = true
            function-references = true

            [codegen]
            opt-level = 2
        "#;
        let config: WasmtimeConfig = toml::from_str(toml_str).unwrap();
        let wt_config = create_engine_config(&config, false, || false);
        wasmtime::Engine::new(&wt_config).expect("engine creation should succeed");
    }

    #[test]
    fn test_load_config_no_env() {
        temp_env::with_var_unset(WASMTIME_CONFIG_ENV, || {
            let config = load_config().unwrap();
            assert_eq!(config, WasmtimeConfig::default());
        });
    }

    #[test]
    fn test_load_config_empty_env() {
        temp_env::with_var(WASMTIME_CONFIG_ENV, Some(""), || {
            let config = load_config().unwrap();
            assert_eq!(config, WasmtimeConfig::default());
        });
    }

    #[test]
    fn test_load_config_nonexistent_file() {
        temp_env::with_var(
            WASMTIME_CONFIG_ENV,
            Some("/nonexistent/path/config.toml"),
            || {
                let result = load_config();
                assert!(result.is_err());
            },
        );
    }

    #[test]
    fn test_load_config_from_file() {
        let dir = std::env::temp_dir().join("runwasi-test-config");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("wasmtime.toml");
        std::fs::write(
            &path,
            r#"
            [wasm]
            gc = true
            function-references = true
        "#,
        )
        .unwrap();

        temp_env::with_var(WASMTIME_CONFIG_ENV, Some(path.to_str().unwrap()), || {
            let config = load_config().unwrap();
            assert_eq!(config.wasm.gc, Some(true));
            assert_eq!(config.wasm.function_references, Some(true));
        });

        std::fs::remove_dir_all(&dir).ok();
    }
}
