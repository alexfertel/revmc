use criterion::{criterion_group, criterion_main, Criterion};
use eyre::Result;

use alloy::{primitives::U256, sol, sol_types::SolCall};
use revm::{
    db::{CacheDB, EmptyDB},
    primitives::{address, Address, TransactTo},
};

use revmc_examples_runner::{
    build_evm, execute_transaction, spoof_storage, ExternalContext, UNISWAP_V2_ROUTER,
    USDC_ADDRESS, WETH_ADDRESS,
};

const CALLER: Address = address!("5555555555555555555555555555555555555555");

fn setup_evm() -> Result<revm::Evm<'static, ExternalContext, CacheDB<EmptyDB>>> {
    let db = CacheDB::new(EmptyDB::new());
    let mut evm = build_evm(db);
    spoof_storage(evm.db_mut())?;

    evm.context.evm.env.tx.caller = CALLER;
    evm.context.evm.env.tx.transact_to = TransactTo::Call(UNISWAP_V2_ROUTER);

    Ok(evm)
}

sol! {
    function getAmountsOut(uint256 amountIn, address[] calldata path) external view returns (uint256[] memory amounts);
}

fn calldata_for_amount(sell_amount: U256) -> Vec<u8> {
    getAmountsOutCall { amountIn: sell_amount, path: vec![WETH_ADDRESS, USDC_ADDRESS] }.abi_encode()
}

const SAMPLE_COUNT: usize = 1000;

fn benchmark_transaction(c: &mut Criterion) {
    let mut evm = setup_evm().unwrap();
    let max_amount: U256 = U256::from(10).pow(U256::from(20));
    let delta = max_amount / U256::from(SAMPLE_COUNT);
    c.bench_function("uniswapv2_simulate", |b| {
        b.iter(|| {
            let mut sell_amount = delta;
            while sell_amount < max_amount {
                let calldata = calldata_for_amount(sell_amount);
                let _tx = execute_transaction(calldata.into(), &mut evm).unwrap();
                sell_amount += delta;
            }
        })
    });
}

criterion_group!(benches, benchmark_transaction);
criterion_main!(benches);
