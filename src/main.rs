use secp256k1::{SecretKey, ONE_KEY};
use web3::{
    contract::{tokens::Tokenize, Contract, Options},
    signing::{Key, SecretKeyRef, Signature},
    transports::Http,
    types::{Address, Bytes, TransactionParameters, U256},
    Web3,
};

#[tokio::main]
async fn main() {
    let token_contract = "0xeAB4eEBa1FF8504c124D031F6844AD98d07C318f";
    let indexer_registry_contract = "0x3649E46eCD6A0bd187f0046C4C35a7B31C92bA1E";
    let state_channel_contract = "0xe78A45427B4797ae9b1852427476A956037B5bC2";

    let miner_str = "";
    let indexer_str = "";
    let consumer_str = "";

    // moonbeam local rpc.
    let web3_endpoint: String = String::from("http://127.0.0.1:9933");

    let miner_sk = SecretKey::from_slice(&hex::decode(miner_str).unwrap()).unwrap();
    let miner = SecretKeyRef::new(&miner_sk);
    let indexer_sk = SecretKey::from_slice(&hex::decode(indexer_str).unwrap()).unwrap();
    let indexer = SecretKeyRef::new(&indexer_sk);
    let consumer_sk = SecretKey::from_slice(&hex::decode(consumer_str).unwrap()).unwrap();
    let consumer = SecretKeyRef::new(&consumer_sk);

    // init web3
    let http = Http::new(&web3_endpoint).unwrap();
    let mut web3 = Web3::new(http);
    let mut token = Contract::from_json(
        web3.eth(),
        token_contract.parse().unwrap(),
        include_bytes!("../contracts/SQToken.json"),
    )
    .unwrap();
    let mut indexer_registry = Contract::from_json(
        web3.eth(),
        indexer_registry_contract.parse().unwrap(),
        include_bytes!("../contracts/IndexerRegistry.json"),
    )
    .unwrap();
    let mut state_channel = Contract::from_json(
        web3.eth(),
        state_channel_contract.parse().unwrap(),
        include_bytes!("../contracts/StateChannel.json"),
    )
    .unwrap();

    let result: String = token
        .query("symbol", (), None, Options::default(), None)
        .await
        .unwrap();
    println!("Token Symbol: {:?}", result);
    let result: Address = token
        .query("getMinter", (), None, Options::default(), None)
        .await
        .unwrap();
    println!("Token Miner: {:?} ?= {:?}", result, miner.address());

    let fn_data = token
        .abi()
        .function("mint")
        .and_then(|function| {
            function.encode_input(&(indexer.address(), U256::from(10000i32)).into_tokens())
        })
        .unwrap();
    let tx = TransactionParameters {
        to: Some(token.address()),
        data: Bytes(fn_data),
        ..Default::default()
    };
    let signed = web3
        .accounts()
        .sign_transaction(tx, &miner_sk)
        .await
        .unwrap();
    let tx_hash = web3
        .eth()
        .send_raw_transaction(signed.raw_transaction)
        .await
        .unwrap();

    let result: U256 = token
        .query(
            "balanceOf",
            (indexer.address()),
            None,
            Options::default(),
            None,
        )
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    println!("Indexer Balance: {:?}", result);
}
