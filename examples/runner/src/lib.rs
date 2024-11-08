use alloy::primitives::{address, b256, bytes, Address, Bytes, B256, U256};
use alloy::{
    network::AnyNetwork,
    providers::{ProviderBuilder, RootProvider},
    rpc::client::ClientBuilder,
    transports::http::{Client, Http},
};
use eyre::bail;
use revm::{
    db::{CacheDB, EmptyDB},
    handler::register::EvmHandler,
    primitives::{AccountInfo, Bytecode, ExecutionResult, Output},
    Database,
};

include!("./common.rs");

// This dependency is needed to define the necessary symbols used by the compiled bytecodes,
// but we don't use it directly, so silence the unused crate depedency warning.
use revmc_builtins as _;

use std::sync::Arc;

use revmc_context::EvmCompilerFn;

revmc_context::extern_revmc! {
    pub fn uniswap_v2_router;
    pub fn uniswap_v2_pair;
}

pub struct ExternalContext;

pub fn execute_transaction(
    calldata: Bytes,
    evm: &mut revm::Evm<'_, ExternalContext, CacheDB<EmptyDB>>,
) -> eyre::Result<()> {
    evm.tx_mut().data = calldata;

    let result = match evm.transact() {
        Ok(result) => result.result,
        Err(e) => bail!("{e}"),
    };

    match result {
        ExecutionResult::Success { output, .. } => match output {
            Output::Call(o) => o,
            Output::Create(o, _) => o,
        },
        ExecutionResult::Revert { output, .. } => return Err(eyre::eyre!("Revert: {:?}", output)),
        ExecutionResult::Halt { reason, .. } => return Err(eyre::eyre!("Halt: {:?}", reason)),
    };

    Ok(())
}

impl ExternalContext {
    fn new() -> Self {
        Self
    }

    fn get_function(&self, bytecode_hash: B256) -> Option<EvmCompilerFn> {
        let f = match bytecode_hash {
            UNISWAP_V2_ROUTER_CODE_HASH => uniswap_v2_router,
            UNISWAP_V2_PAIR_CODE_HASH => uniswap_v2_pair,
            _ => return None,
        };

        Some(EvmCompilerFn::new(f))
    }
}

#[inline]
fn register_handler<DB: Database + 'static>(handler: &mut EvmHandler<'_, ExternalContext, DB>) {
    let prev = handler.execution.execute_frame.clone();
    handler.execution.execute_frame = Arc::new(move |frame, memory, tables, context| {
        let interpreter = frame.interpreter_mut();
        let bytecode_hash = interpreter.contract.hash.unwrap_or_default();
        if let Some(f) = context.external.get_function(bytecode_hash) {
            Ok(unsafe { f.call_with_interpreter_and_memory(interpreter, memory, context) })
        } else {
            prev(frame, memory, tables, context)
        }
    });
}

#[inline]
pub fn build_evm<'a, DB: Database + 'static>(db: DB) -> revm::Evm<'a, ExternalContext, DB> {
    revm::Evm::builder()
        .with_db(db)
        .with_external_context(ExternalContext::new())
        .append_handler_register(register_handler)
        .build()
}

pub fn get_http_provider(endpoint: &str) -> RootProvider<Http<Client>, AnyNetwork> {
    ProviderBuilder::new()
        .network::<AnyNetwork>()
        .on_client(ClientBuilder::default().http(endpoint.parse().unwrap()))
}

#[inline]
pub fn spoof_storage(db: &mut CacheDB<EmptyDB>) -> eyre::Result<()> {
    let router_info = AccountInfo::new(
        U256::ZERO,
        1,
        UNISWAP_V2_ROUTER_CODE_HASH,
        Bytecode::new_raw(UNISWAP_V2_ROUTER_CODE.clone()),
    );
    let pair_info = AccountInfo::new(
        U256::ZERO,
        1,
        UNISWAP_V2_PAIR_CODE_HASH,
        Bytecode::new_raw(UNISWAP_V2_PAIR_CODE.clone()),
    );
    db.insert_account_info(UNISWAP_V2_ROUTER, router_info);
    db.insert_account_info(UNISWAP_V2_PAIR, pair_info);

    db.insert_account_storage(
        UNISWAP_V2_PAIR,
        U256::from(8),
        b256!("672cefab00000000031e2dc3da6b0fc5e69600000000000000002602aab8db9a").into(),
    )?;

    Ok(())
}
