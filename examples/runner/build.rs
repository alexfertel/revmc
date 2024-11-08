use alloy::primitives::{address, b256, bytes, Address, Bytes, B256};
use revmc::{primitives::SpecId, EvmCompiler, EvmLlvmBackend, OptimizationLevel, Result};
use std::path::PathBuf;

include!("./src/common.rs");

fn main() -> Result<()> {
    // Emit the configuration to run compiled bytecodes
    // This not used if we are only using statically linked bytecodes
    revmc_build::emit();

    // Uniswap V2 Router.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let context = revmc::llvm::inkwell::context::Context::create();
    let backend = EvmLlvmBackend::new(&context, true, OptimizationLevel::Aggressive)?;
    let mut compiler = EvmCompiler::new(backend);
    compiler.gas_metering(false);
    unsafe { compiler.stack_bound_checks(false) }

    let name = "uniswap_v2_router";
    let bytecode = UNISWAP_V2_ROUTER_CODE;
    compiler.translate(name, &bytecode, SpecId::CANCUN)?;
    let object = out_dir.join(name).with_extension("o");
    compiler.write_object_to_file(&object)?;

    cc::Build::new().object(&object).static_flag(true).compile(name);

    // Uniswap V2 Pair.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let context = revmc::llvm::inkwell::context::Context::create();
    let backend = EvmLlvmBackend::new(&context, true, OptimizationLevel::Aggressive)?;
    let mut compiler = EvmCompiler::new(backend);
    compiler.gas_metering(false);
    unsafe { compiler.stack_bound_checks(false) }

    let name = "uniswap_v2_pair";
    compiler.translate(name, &UNISWAP_V2_PAIR_CODE, SpecId::CANCUN)?;
    let object = out_dir.join(name).with_extension("o");
    compiler.write_object_to_file(&object)?;

    cc::Build::new().object(&object).static_flag(true).compile(name);

    Ok(())
}
