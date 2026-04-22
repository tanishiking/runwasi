use std::sync::Arc;

use anyhow::Result;
use containerd_shim_wasm::sandbox::Sandbox;
use containerd_shim_wasm::sandbox::context::{Entrypoint, RuntimeContext};
use containerd_shim_wasm::shim::{Shim, Version, version};
use tokio::runtime::Handle;
use wasmer::Module;
use wasmer_types::ModuleHash;
use wasmer_wasix::WasiRuntimeError;
use wasmer_wasix::runners::wasi::{RuntimeOrEngine, WasiRunner};
use wasmer_wasix::virtual_fs::host_fs::FileSystem;

pub struct WasmerShim;

#[derive(Default)]
pub struct WasmerSandbox {
    engine: wasmer::sys::Cranelift,
}

impl Shim for WasmerShim {
    fn name() -> &'static str {
        "wasmer"
    }

    fn version() -> Version {
        version!()
    }

    type Sandbox = WasmerSandbox;
}

impl Sandbox for WasmerSandbox {
    async fn run_wasi(&self, ctx: &impl RuntimeContext) -> Result<i32> {
        let args = ctx.args();
        let envs = ctx
            .envs()
            .iter()
            .map(|v| match v.split_once('=') {
                None => (v.to_string(), String::new()),
                Some((key, value)) => (key.to_string(), value.to_string()),
            })
            .collect::<Vec<_>>();
        let Entrypoint {
            source,
            func,
            arg0: _,
            name,
        } = ctx.entrypoint();

        let mod_name = name.unwrap_or_else(|| "main".to_string());

        let wasm_bytes = source.as_bytes()?;
        let engine = wasmer::Engine::from(self.engine.clone());
        let module = Module::new(&engine, &wasm_bytes)?;

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        let _guard = runtime.enter();

        log::info!("Creating `WasiRunner`...: args {args:?}, envs: {envs:?}");
        let fs = FileSystem::new(Handle::current(), "/")?;
        log::info!("Running {func:?}");
        let mut runner = WasiRunner::new();
        runner
            .with_args(args.iter().skip(1).cloned())
            .with_envs(envs)
            .with_entry_function(func)
            .with_mount("/".to_string(), Arc::new(fs));

        match runner.run_wasm(
            RuntimeOrEngine::Engine(engine),
            &mod_name,
            module,
            ModuleHash::xxhash(wasm_bytes.as_ref()),
        ) {
            Ok(()) => Ok(0),
            Err(err) => {
                if let Some(code) = err
                    .downcast_ref::<WasiRuntimeError>()
                    .and_then(WasiRuntimeError::as_exit_code)
                {
                    Ok(code.raw() as i32)
                } else {
                    Err(err)
                }
            }
        }
    }
}
