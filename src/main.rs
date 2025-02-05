use openrpc_testgen::utils::{
    get_deployed_contract_address,
    starknet_hive::StarknetHive,
    v7::{
        accounts::{
            account::{Account, ConnectedAccount},
            call::Call,
        },
        contract::factory::ContractFactory,
        endpoints::utils::{get_selector_from_name, wait_for_sent_transaction},
        providers::provider::Provider,
    },
};
use rand::{rngs::StdRng, Rng, RngCore, SeedableRng};
use starknet::providers::Url;
use starknet_types_core::felt::Felt;
use starknet_types_rpc::{BlockId, BlockTag};
use tracing::Level;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    let node_url = Url::parse("https://starknet-sepolia.g.alchemy.com/starknet/version/rpc/v0_7/ffGrvRxnad_MXHATSkRmHUu8vUlghEok").unwrap();
    let address = Felt::from_hex_unchecked(
        "0x64fa47c02430e5d69c0c5d340e23397bca308f7b9d85247565fc91f2c2ad2f2",
    );
    let private_key = Felt::from_hex_unchecked(
        "0x4b10399eb3eee3bddbe485790b2d81ab30be8019b634e44728b96e6ec383d47",
    );

    // Step 1: Initialize hive for tests
    let hive = StarknetHive::new_founding(node_url, address, private_key)
        .await
        .unwrap();

    // Step 2: Deploy the declared contract
    let account = hive.account.clone();

    let class_hash = Felt::from_hex_unchecked(
        "0x053759fefeea6bf916dddd5f69f7e6b2eefc81ce80c3c2e235cef1effc1a5034",
    );

    let factory = ContractFactory::new(class_hash, account.clone());

    let mut salt_buffer = [0u8; 32];
    let mut rng = StdRng::from_entropy();
    rng.fill_bytes(&mut salt_buffer[1..]);

    let deployment_result = factory
        .deploy_v3(vec![], Felt::from_bytes_be(&salt_buffer), true)
        .send()
        .await
        .unwrap();

    wait_for_sent_transaction(deployment_result.transaction_hash, &account)
        .await
        .unwrap();

    // Step 3: Retrieve the deployed contract address
    let deployed_contract_address = get_deployed_contract_address::get_contract_address(
        hive.provider(),
        deployment_result.transaction_hash,
    )
    .await
    .unwrap();

    // Step 4: Prepare to invoke the contract
    let increase_balance_call = Call {
        to: deployed_contract_address,
        selector: get_selector_from_name("increase_balance").unwrap(),
        calldata: vec![Felt::from_hex("0x50").unwrap()],
    };

    let txn_target_count = rand::thread_rng().gen_range(3..=10);
    let mut txn_count = 0;

    let mut initial_block_number = hive.provider().block_number().await.unwrap();

    // Step 5: Wait for a new block to start with a clean slate
    loop {
        let current_block_number = hive.provider().block_number().await.unwrap();
        if current_block_number > initial_block_number {
            initial_block_number = current_block_number;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    // Step 6: Execute transactions until the target count is reached or a new block is detected
    let mut nonce = hive
        .provider()
        .get_nonce(BlockId::Tag(BlockTag::Pending), hive.address())
        .await
        .unwrap();
    loop {
        println!("Nonce before tx: {}", nonce);
        hive.execute_v3(vec![increase_balance_call.clone()])
            .nonce(nonce)
            .send()
            .await
            .unwrap();

        txn_count += 1;
        nonce += Felt::ONE;
        println!("Nonce after tx: {}", nonce);

        if txn_count >= txn_target_count {
            let block_number = hive.provider().block_number().await.unwrap();

            loop {
                let current_block_number = hive.provider().block_number().await.unwrap();
                if current_block_number > block_number {
                    initial_block_number = current_block_number;
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            break;
        }

        let current_block_number = hive.provider().block_number().await.unwrap();
        if initial_block_number < current_block_number {
            initial_block_number = current_block_number;
            break;
        }
    }

    // // Step 7: Verify the transaction count in the block and if the response is ok
    let block_txn_count = hive
        .provider()
        .get_block_transaction_count(BlockId::Number(initial_block_number))
        .await
        .unwrap();
    println!("Block transaction count: {}", block_txn_count);
}
