use alloy::primitives::U256;
use eyre::{bail, Result};
use revm::{
    db::{CacheDB, EmptyDB},
    primitives::{address, ExecutionResult, Output, TransactTo},
};

use revmc_examples_runner::{build_evm, spoof_storage, CALLDATA, UNISWAP_V2_ROUTER};

// const ENDPOINT: Option<&str> = option_env!("RPC_URL");

fn main() -> Result<()> {
    // let Some(endpoint) = ENDPOINT else {
    //     return Ok(());
    // };
    //
    // let provider = get_http_provider(endpoint);
    // let meta = BlockchainDbMeta {
    //     cfg_env: Default::default(),
    //     block_env: Default::default(),
    //     hosts: BTreeSet::from([endpoint.to_string()]),
    // };
    //
    // let backend = SharedBackend::spawn_backend_thread(
    //     Arc::new(provider),
    //     BlockchainDb::new(meta, None),
    //     Some(BlockNumberOrTag::Latest.into()),
    // );

    let db = CacheDB::new(EmptyDB::new());
    let mut evm = build_evm(db);

    spoof_storage(evm.db_mut())?;

    let tx = evm.tx_mut();

    tx.caller = address!("5555555555555555555555555555555555555555");
    tx.transact_to = TransactTo::Call(UNISWAP_V2_ROUTER);
    tx.data = CALLDATA.clone();
    tx.value = U256::ZERO;

    let result = match evm.transact_commit() {
        Ok(result) => result,
        Err(e) => bail!("{e}"),
    };

    let output = match result {
        ExecutionResult::Success { output, .. } => match output {
            Output::Call(o) => o,
            Output::Create(o, _) => o,
        },
        ExecutionResult::Revert { output, .. } => return Err(eyre::eyre!("Revert: {:?}", output)),
        ExecutionResult::Halt { reason, .. } => return Err(eyre::eyre!("Halt: {:?}", reason)),
    };

    println!("{:?}", output);

    Ok(())
}
