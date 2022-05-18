use secp256k1::SecretKey;
use std::collections::HashMap;
use web3::{
    contract::{tokens::Tokenize, Contract, Options},
    signing::{Key, SecretKeyRef},
    transports::Http,
    types::{Address, Bytes, TransactionParameters, U256},
    Web3,
};

async fn transfer(web3: &Web3<Http>, sk: &SecretKey, address: Address, amount: i128) {
    println!("Transfer FEE to: {:?} ...", address);
    let tx = TransactionParameters {
        to: Some(address),
        value: U256::from(amount),
        ..Default::default()
    };
    let signed = web3.accounts().sign_transaction(tx, sk).await.unwrap();
    let _tx_hash = web3
        .eth()
        .send_raw_transaction(signed.raw_transaction)
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let result: U256 = web3.eth().balance(address, None).await.unwrap();
    println!("{:?} Balance: {:?}", address, result);
}

async fn transfer_token(
    web3: &Web3<Http>,
    contract: &Contract<Http>,
    sk: &SecretKey,
    address: Address,
    amount: i64,
) {
    println!("Transfer SQT to: {:?} ...", address);
    let fn_data = contract
        .abi()
        .function("transfer")
        .and_then(|function| function.encode_input(&(address, U256::from(amount)).into_tokens()))
        .unwrap();
    let tx = TransactionParameters {
        to: Some(contract.address()),
        data: Bytes(fn_data),
        ..Default::default()
    };
    let signed = web3.accounts().sign_transaction(tx, sk).await.unwrap();
    let _tx_hash = web3
        .eth()
        .send_raw_transaction(signed.raw_transaction)
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let result: U256 = contract
        .query("balanceOf", (address,), None, Options::default(), None)
        .await
        .unwrap();
    println!("{:?} SQT Balance: {:?}", address, result);
}

async fn token_approve(
    web3: &Web3<Http>,
    contract: &Contract<Http>,
    sk: &SecretKey,
    address: Address,
    amount: i64,
) {
    println!("Approve SQT to: {:?} ...", address);
    let fn_data = contract
        .abi()
        .function("increaseAllowance")
        .and_then(|function| function.encode_input(&(address, U256::from(amount)).into_tokens()))
        .unwrap();
    let tx = TransactionParameters {
        to: Some(contract.address()),
        data: Bytes(fn_data),
        ..Default::default()
    };
    let signed = web3.accounts().sign_transaction(tx, sk).await.unwrap();
    let _tx_hash = web3
        .eth()
        .send_raw_transaction(signed.raw_transaction)
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let result: U256 = contract
        .query(
            "allowance",
            (SecretKeyRef::new(sk).address(), address),
            None,
            Options::default(),
            None,
        )
        .await
        .unwrap();
    println!("Approved SQT {:?}", result);
}

async fn register_indexer(
    web3: &Web3<Http>,
    contract: &Contract<Http>,
    sk: &SecretKey,
    amount: i64,
) {
    let indexer = SecretKeyRef::new(&sk);
    let address = indexer.address();
    println!("Register Indexer: {:?} ...", indexer.address());
    let result: bool = contract
        .query("isIndexer", (address,), None, Options::default(), None)
        .await
        .unwrap();
    if result {
        println!("Had Register Indexer: {}", result);
        return;
    }

    let fn_data = contract
        .abi()
        .function("registerIndexer")
        .and_then(|function| {
            function.encode_input(&(U256::from(amount), [0u8; 32], U256::from(0i32)).into_tokens())
        })
        .unwrap();
    //let nonce = web3.eth().transaction_count(address, None).await.unwrap();
    let tx = TransactionParameters {
        to: Some(contract.address()),
        data: Bytes(fn_data),
        ..Default::default()
    };
    let signed = web3.accounts().sign_transaction(tx, sk).await.unwrap();
    let tx_hash = web3
        .eth()
        .send_raw_transaction(signed.raw_transaction)
        .await
        .unwrap();
    println!("tx: {:?}", tx_hash);

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let result: bool = contract
        .query("isIndexer", (address,), None, Options::default(), None)
        .await
        .unwrap();
    println!("Register Indexer: {}", result);
}

#[tokio::main]
async fn main() {
    //let miner_str = "df57089febbacf7ba0bc227dafbffa9fc08a93fdc68e1e42411a14efcf23656e";
    let miner_str = "5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133";
    //let indexer_str = "f01e387d20ac6b12d0b334b394735a1756175aec31087c3c9acc5af32838d678";
    let indexer_str = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
    let consumer_str = "70d76f81693955b3c90f4f0ba399853d7b43a37448bb2947a5dbfc18b592e91d";

    // moonbeam local rpc.
    let web3_endpoint: String = String::from("http://127.0.0.1:8545");

    let miner_sk = SecretKey::from_slice(&hex::decode(miner_str).unwrap()).unwrap();
    let miner = SecretKeyRef::new(&miner_sk);
    let indexer_sk = SecretKey::from_slice(&hex::decode(indexer_str).unwrap()).unwrap();
    let indexer = SecretKeyRef::new(&indexer_sk);
    let i_address = indexer.address();
    let consumer_sk = SecretKey::from_slice(&hex::decode(consumer_str).unwrap()).unwrap();
    let consumer = SecretKeyRef::new(&consumer_sk);
    let c_address = consumer.address();

    let web3 = Web3::new(Http::new(&web3_endpoint).unwrap());
    let file = std::fs::File::open("./contracts/local.json").unwrap();
    let reader = std::io::BufReader::new(file);
    let list: serde_json::Value = serde_json::from_reader(reader).unwrap();
    let mut contracts = HashMap::new();
    for name in vec!["SQToken", "StateChannel", "IndexerRegistry", "Staking"] {
        contracts.insert(
            name,
            Contract::from_json(
                web3.eth(),
                list[name]["address"].as_str().unwrap().parse().unwrap(),
                &std::fs::read(format!("./contracts/{}.json", name)).unwrap(),
            )
            .unwrap(),
        );
    }

    let result: String = contracts["SQToken"]
        .query("symbol", (), None, Options::default(), None)
        .await
        .unwrap();
    println!("Token Symbol: {:?}", result);
    let result: Address = contracts["SQToken"]
        .query("getMinter", (), None, Options::default(), None)
        .await
        .unwrap();
    println!("Token Miner: {:?} != {:?}", result, miner.address());
    let result: U256 = web3.eth().balance(miner.address(), None).await.unwrap();
    println!("Miner Balance: {:?}", result);

    let result: U256 = contracts["SQToken"]
        .query(
            "balanceOf",
            (miner.address(),),
            None,
            Options::default(),
            None,
        )
        .await
        .unwrap();
    println!("Miner SQT Balance: {:?}", result);

    println!("\x1b[92m------------------------------------\x1b[00m");
    // Transfer DEV main token to indexer/consumer
    //transfer(&web3, &miner_sk, i_address, 10_000_000_000_000_000_000).await;
    //transfer(&web3, &miner_sk, c_address, 1_000_000_000_000_000_000).await;

    println!("\x1b[92m------------------------------------\x1b[00m");
    // Transfer SQT to indexer/consumer
    //transfer_token(&web3, &contracts["SQToken"], &miner_sk, i_address, 1000000).await;
    //transfer_token(&web3, &contracts["SQToken"], &miner_sk, c_address, 1000000).await;

    println!("\x1b[92m------------------------------------\x1b[00m");
    // Register indexer
    let staking = contracts["Staking"].address();
    let channel = contracts["StateChannel"].address();
    //token_approve(&web3, &contracts["SQToken"], &indexer_sk, staking, 1000000).await;
    //token_approve(&web3, &contracts["SQToken"], &consumer_sk, channel, 1000000).await;

    register_indexer(&web3, &contracts["IndexerRegistry"], &indexer_sk, 100000).await;
}
