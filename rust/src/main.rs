// TODO: proper error handling. don't unwrap
// use std::time;
use std::sync::Arc;
// use futures::future::IntoFuture;
use web3::contract;
use web3::futures::{Future, Stream};
use web3::types::{Address, FilterBuilder, U256};

// TODO: less strict type on web3
fn subscribe_new_heads(w3: Arc<web3::Web3<web3::transports::WebSocket>>) -> impl Future<Item = (), Error = ()> {
    println!("subscribing to new heads...");
    let the_future = w3
        .eth_subscribe()
        .subscribe_new_heads()
        .and_then(|sub| {
            sub.for_each(|log| {
                println!("got block log: {:?}", log);
                Ok(())
            })
        })
        .map_err(|e| eprintln!("block log err: {:#?}", e));

    Box::new(the_future)
}

fn subscribe_factory_logs(w3: Arc<web3::Web3<web3::transports::WebSocket>>, uniswap_factory_address: Address) -> impl Future<Item = (), Error = ()> {
    println!("subscribing to uniswap factory logs...");

    let filter = FilterBuilder::default()
        .address(vec![uniswap_factory_address])
        .build();

    let the_future = w3
        .eth_subscribe()
        .subscribe_logs(filter)
        .and_then(|sub| {
            sub.for_each(|log| {
                println!("got uniswap factory log from subscription: {:?}", log);
                // TODO: get the exchange and token addresses out of the log
                // TODO: subscribe to the exchange logs. if we get any, print the current price on the exchange
                // TODO: if sync status is behind, alert that the price is old
                Ok(())
            })
        })
        // TODO: proper error handling
        .map_err(|e| eprintln!("uniswap log err: {:#?}", e));

    // Box::new(the_future)
    the_future
}

// TODO: don't return Item = (). Instead, return the Address
fn get_token_with_id(
    token_index: u64,
    _uniswap_exchange_abi: &[u8],
    uniswap_factory_contract: Arc<contract::Contract<web3::transports::WebSocket>>,
    _w3: Arc<web3::Web3<web3::transports::WebSocket>>,    
// ) -> Box<Future<Item = Address, Error = web3::contract::Error> + Send> {
) -> impl Future<Item = (), Error = ()> {
    // println!("querying token index: {}", token_index);

    let the_future = uniswap_factory_contract
        .query(
            "getTokenWithId",
            (token_index,),
            None,
            contract::Options::default(),
            None,
        )
        .and_then(move |uniswap_token_address: Address| {
            println!("uniswap_token_address #{}: {:#?}", token_index, uniswap_token_address);

            Ok(())
        })
        .map_err(|e| eprintln!("uniswap exchange err: {:#?}", e));

    //         uniswap_factory_contract
    //             .query(
    //                 "getExchange",
    //                 (uniswap_token_address,),
    //                 None,
    //                 contract::Options::default(),
    //                 None,
    //             )
    //             // TODO: flatten this
    //             .and_then(move |uniswap_exchange_address: Address| {
    //                 println!("uniswap_token_address: {:#?}; uniswap_exchange_address: {:#?}", uniswap_token_address, uniswap_exchange_address);

    //                 let erc20_contract =
    //                     contract::Contract::from_json(
    //                         w3.eth(),
    //                         uniswap_token_address,
    //                         erc20_abi,
    //                     )
    //                     .unwrap();

    //                 // check the reserves so we can pick valid order sizes
    //                 erc20_contract.query(
    //                     "balanceOf",
    //                     (uniswap_exchange_address, ),
    //                     None,
    //                     contract::Options::default(),
    //                     None,
    //                 ).and_then(move |token_supply: U256| {
    //                     println!("uniswap_token_address: {:#?}; uniswap_exchange_address: {:#?}; token supply: {}", uniswap_token_address, uniswap_exchange_address, token_supply);

    //                     if token_supply == 0.into() {
    //                         // if no supply, skip this exchange
    //                         // TODO: what kind of error can we actually raise here?
    //                         // https://tokio.rs/docs/futures/combinators/#returning-from-multiple-branches
    //                         // panic!("what can i return here that won't break the futures?")
    //                         // Box::new(futures::future::err("token supply is 0. Skipping"))
    //                         // futures::future::Either::A(Ok(()))
    //                         // Ok(())
    //                     }

    //                     w3.eth().balance(uniswap_exchange_address, None).and_then(move |ether_supply: U256| {
    //                         println!("uniswap_token_address: {:#?}; uniswap_exchange_address: {:#?}; token supply: {}, ether_balance: {:#?}", uniswap_token_address, uniswap_exchange_address, token_supply, ether_supply);

    //                         let _uniswap_exchange_contract =
    //                             contract::Contract::from_json(
    //                                 w3.eth(),
    //                                 uniswap_exchange_address,
    //                                 uniswap_exchange_abi,
    //                             )
    //                             .unwrap();

    //                             // TODO: getTokenToEthInputPrice? getTokenToEthOutputPrice? getEthToTokenInputPrice? getEthToTokenOutputPir

    //                         Ok(())
    //                     }).or_else(|err| {
    //                         eprintln!("ether_balance err: {:#?}", err);
    //                         Ok(())
    //                     })
    //                 }).or_else(move |err| {
    //                     // if we got an error, skip this exchange
    //                     eprintln!("{:#?}.balanceOf({:#?}) failed: {:#?}", uniswap_token_address, uniswap_exchange_address, err);
    //                     Ok(())
    //                 })
    //             })
    //     })

    // Box::new(the_future)
    the_future
}

fn query_existing_exchanges(
    eloop_handle: Arc<tokio_core::reactor::Handle>,
    _erc20_abi: &[u8],
    uniswap_exchange_abi: &'static [u8],
    uniswap_factory_contract: Arc<contract::Contract<web3::transports::WebSocket>>,
    w3: Arc<web3::Web3<web3::transports::WebSocket>>,
) -> impl Future<Item = (), Error = ()> {
    println!("querying existing exchanges...");

    // instead of fetching historic logs, get the exchanges by querying the contract
    // Get token count. (getTokenCount())
    // For each token in token count, get the address with id "i". (getTokenWithId(id))
    // For each token address, get the exchange address. (getExchange(token))
    let the_future = uniswap_factory_contract
        .query("tokenCount", (), None, contract::Options::default(), None)
        .and_then(move |uniswap_token_count: U256| {
            let uniswap_token_count: u64 = uniswap_token_count.as_u64();
            println!("uniswap_token_count: {}", uniswap_token_count);

            // TODO: range over U256 instead of limited to u64
            for token_index in 1..=uniswap_token_count {
                // TODO: not sure about using eloop_handle here, but it seems to be working
                eloop_handle.spawn(get_token_with_id(token_index, uniswap_exchange_abi, uniswap_factory_contract.clone(), w3.clone()));
            }

            // TODO: i thought i should return _token_addresses with something done to it here instead, but I can't get that to work
            // TODO: use futures::stream::futures_unordered?
            Ok(())
        })
        .map_err(|e| eprintln!("uniswap exchange err: {:#?}", e));

    // Box::new(the_future)
    the_future
}

// fn my_operation(arg: String) -> impl Future<Item = String> {
//     if is_valid(&arg) {
//         return Either::A(get_message().map(|message| {
//             format!("MESSAGE = {}", message)
//         }));
//     }

//     Either::B(future::err("something went wrong"))
// }

fn main() {
    let mut eloop = tokio_core::reactor::Core::new().unwrap();
    let handle = Arc::new(eloop.handle());
    let w3 = Arc::new(web3::Web3::new(
        // TODO: read the websocket uri from an environment variable. default to localhost
        web3::transports::WebSocket::with_event_loop("wss://eth.stytt.com:8546", &eloop.handle())
            .unwrap(),
    ));

    // TODO: read these from env vars so we can use a development blockchain easily
    // let uniswap_genesis_block = 6_627_917;
    let uniswap_factory_address: Address =
        "c0a47dfe034b400b47bdad5fecda2621de6c4d95".parse().unwrap();

    // chain:   address                                  block height
    // mainnet: c0a47dfe034b400b47bdad5fecda2621de6c4d95 6_627_917

    let erc20_abi: &[u8] = include_bytes!("erc20.abi");
    let uniswap_exchange_abi: &[u8] = include_bytes!("uniswap_exchange.abi");
    let uniswap_factory_abi: &[u8] = include_bytes!("uniswap_factory.abi");

    // TODO: subscribe to sync status instead. if we are behind by more than X blocks, give a notice. except ganache doesn't support that
    let subscribe_new_heads_future = subscribe_new_heads(w3.clone());

    let subscribe_factory_logs_future = subscribe_factory_logs(w3.clone(), uniswap_factory_address);

    let uniswap_factory_contract = Arc::new(contract::Contract::from_json(w3.eth(), uniswap_factory_address, uniswap_factory_abi).unwrap());
    println!(
        "contract deployed at: {:#?}",
        uniswap_factory_contract.address()
    );

    let query_existing_exchanges_future = query_existing_exchanges(handle.clone(), erc20_abi, uniswap_exchange_abi, uniswap_factory_contract, w3.clone());

    let all_futures = futures::future::lazy(|| {
        subscribe_new_heads_future.join3(subscribe_factory_logs_future, query_existing_exchanges_future)
    });
    if let Err(_err) = eloop.run(all_futures) {
        println!("ERROR");
    }
}
