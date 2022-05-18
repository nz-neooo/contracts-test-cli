use secp256k1::{SecretKey, ONE_KEY};
use std::collections::HashMap;
use web3::{
    contract::{tokens::Tokenize, Contract, Options},
    signing::{Key, SecretKeyRef, Signature},
    transports::Http,
    types::{Address, Bytes, TransactionParameters, U256},
    Web3,
};

async fn transfer(
    web3: &Web3<Http>,
    contract: &Contract<Http>,
    sk: &SecretKey,
    address: Address,
    amount: i32,
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
    let tx_hash = web3
        .eth()
        .send_raw_transaction(signed.raw_transaction)
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let result: U256 = contract
        .query("balanceOf", (address,), None, Options::default(), None)
        .await
        .unwrap();
    println!("{:?} Balance: {:?}", address, result);
}

#[tokio::main]
async fn main() {
    let miner_str = "5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133";
    let indexer_str = "f01e387d20ac6b12d0b334b394735a1756175aec31087c3c9acc5af32838d678";
    let consumer_str = "70d76f81693955b3c90f4f0ba399853d7b43a37448bb2947a5dbfc18b592e91d";

    // moonbeam local rpc.
    let web3_endpoint: String = String::from("http://127.0.0.1:9933");

    let miner_sk = SecretKey::from_slice(&hex::decode(miner_str).unwrap()).unwrap();
    let miner = SecretKeyRef::new(&miner_sk);
    let indexer_sk = SecretKey::from_slice(&hex::decode(indexer_str).unwrap()).unwrap();
    let indexer = SecretKeyRef::new(&indexer_sk);
    let consumer_sk = SecretKey::from_slice(&hex::decode(consumer_str).unwrap()).unwrap();
    let consumer = SecretKeyRef::new(&consumer_sk);

    let web3 = Web3::new(Http::new(&web3_endpoint).unwrap());
    let file = std::fs::File::open("./contracts/local.json").unwrap();
    let reader = std::io::BufReader::new(file);
    let list: serde_json::Value = serde_json::from_reader(reader).unwrap();
    let mut contracts = HashMap::new();
    for name in vec!["SQToken", "StateChannel", "IndexerRegistry"] {
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
    println!("Token Miner: {:?} ?= {:?}", result, miner.address());
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
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    println!("Miner Balance: {:?}", result);

    // Transfer SQT to indexer/consumer
    transfer(
        &web3,
        &contracts["SQToken"],
        &miner_sk,
        indexer.address(),
        10000,
    )
    .await;

    transfer(
        &web3,
        &contracts["SQToken"],
        &miner_sk,
        consumer.address(),
        10000,
    )
    .await;

    // Transfer DEV main token to indexer/consumer
}
