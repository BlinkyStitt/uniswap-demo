// TODO: proper error handling. don't unwrap
// TODO: proper error logging. some errors should just be warnings
// use std::time;
use env_logger;
use std::rc::Rc;
// use futures::future::IntoFuture;
use web3::contract;
use web3::futures::{Future, Stream};
use web3::types::{Address, FilterBuilder, U256};

// TODO: less strict type on web3
fn subscribe_new_heads(
    w3: Rc<web3::Web3<web3::transports::WebSocket>>,
) -> impl Future<Item = (), Error = ()> {
    println!("subscribing to new heads...");
    w3.eth_subscribe()
        .subscribe_new_heads()
        .and_then(|sub| {
            sub.for_each(|log| {
                println!("got block log: {:?}", log);
                Ok(())
            })
        })
        .map_err(|e| eprintln!("block log err: {}", e))
}

fn subscribe_factory_logs(
    w3: Rc<web3::Web3<web3::transports::WebSocket>>,
    uniswap_factory_address: Address,
) -> impl Future<Item = (), Error = ()> {
    println!("subscribing to uniswap factory logs...");

    let filter = FilterBuilder::default()
        .address(vec![uniswap_factory_address])
        .build();

    w3.eth_subscribe()
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
        .map_err(|e| eprintln!("uniswap log err: {}", e))
}

fn subscribe_exchange_logs(
    eloop_handle: tokio_core::reactor::Handle,
    erc20_abi: &'static [u8],
    uniswap_exchange_abi: &'static [u8],
    uniswap_exchange_address: Address,
    uniswap_token_address: Address,
    w3: Rc<web3::Web3<web3::transports::WebSocket>>,
) -> impl Future<Item = (), Error = ()> {
    // println!("subscribing to uniswap exchange {:#?} logs for token {:#?}...", uniswap_exchange_address, uniswap_token_address);

    let filter = FilterBuilder::default()
        .address(vec![uniswap_exchange_address])
        .build();

    w3.eth_subscribe()
        .subscribe_logs(filter)
        .and_then(move |sub| {
            sub.for_each(move |log| {
                println!("got uniswap exchange log from subscription: {:?}", log);

                // TODO: if sync status is behind, alert that the price is old and return
                // if log.block_number <  in_sync_block_number: return Ok(());

                // TODO: do we even care about parsing the log? i think we should just recalculate the prices

                let check_prices_future = check_prices(
                    eloop_handle.clone(),
                    erc20_abi,
                    uniswap_exchange_abi,
                    uniswap_exchange_address,
                    uniswap_token_address,
                    w3.clone(),
                );
                eloop_handle.spawn(check_prices_future);

                Ok(())
            })
        })
        // TODO: proper error handling
        .map_err(|e| eprintln!("uniswap exchange logs err: {}", e))
}

fn check_prices(
    eloop_handle: tokio_core::reactor::Handle,
    erc20_abi: &'static [u8],
    uniswap_exchange_abi: &'static [u8],
    uniswap_exchange_address: Address,
    uniswap_token_address: Address,
    w3: Rc<web3::Web3<web3::transports::WebSocket>>,
) -> impl Future<Item = (), Error = ()> {
    let erc20_contract =
        contract::Contract::from_json(w3.eth(), uniswap_token_address, erc20_abi).unwrap();

    // check the reserves so we can pick valid order sizes
    erc20_contract
        .query(
            "balanceOf",
            (uniswap_exchange_address,),
            None,
            contract::Options::default(),
            None,
        )
        .and_then(move |token_supply: U256| {
            // println!("#{}: uniswap_token_address: {:#?}; uniswap_exchange_address: {:#?}; token supply: {}", token_index, uniswap_token_address, uniswap_exchange_address, token_supply);

            if token_supply == 0.into() {
                // if no supply, skip this exchange
                return Ok(());
            }

            // clone the handle so it can be moved into the ether_balance_future
            let eloop_handle_clone = eloop_handle.clone();

            let ether_balance_future = w3
                .eth()
                .balance(uniswap_exchange_address, None)
                .and_then(move |ether_supply: U256| {
                    // the exchange contract has token and eth in reserves.
                    // println!("uniswap_token_address: {:#?}; uniswap_exchange_address: {:#?}; token supply: {}; ether_balance: {:#?}", uniswap_token_address, uniswap_exchange_address, token_supply, ether_supply);

                    // TODO: what amounts should we checks?
                    let ether_to_buy = ether_supply / 100;
                    let token_to_buy = token_supply / 100;

                    let uniswap_exchange_contract = contract::Contract::from_json(
                        w3.eth(),
                        uniswap_exchange_address,
                        uniswap_exchange_abi,
                    )
                    .unwrap();

                    // TODO: getTokenToEthInputPrice? getTokenToEthOutputPrice? getEthToTokenInputPrice? getEthToTokenOutputPrice
                    // i think we should use input price functions. either should work, but we should only need one
                    // TODO: do this in a loop so that we can check multiple prices instead of just 10% of the supply
                    if ether_to_buy > 0.into() {
                        let token_to_eth_future = uniswap_exchange_contract
                            .query(
                                "getTokenToEthOutputPrice",
                                (ether_to_buy,),
                                None,
                                contract::Options::default(),
                                None,
                            )
                            .and_then(move |token_price: U256| {
                                println!(
                                    "can buy {} ETH for {} {:#?}",
                                    ether_to_buy, token_price, uniswap_token_address
                                );

                                // TODO: create an order object here and send it through the channel

                                Ok(())
                            })
                            .map_err(|_err| {
                                // TODO: better errors
                                // eprintln!("getTokenToEthOutputPrice err: {}", err);
                            });
                        eloop_handle_clone.spawn(token_to_eth_future);
                    }

                    if token_to_buy > 0.into() {
                        let eth_to_token_future = uniswap_exchange_contract
                            .query(
                                "getEthToTokenOutputPrice",
                                (token_to_buy,),
                                None,
                                contract::Options::default(),
                                None,
                            )
                            .and_then(move |ether_price: U256| {
                                println!(
                                    "can buy {} {:#?} for {} ETH",
                                    token_to_buy, uniswap_token_address, ether_price
                                );

                                // TODO: create an order object here and send it through the channel

                                Ok(())
                            })
                            .map_err(|_err| {
                                // TODO: better errors
                                // eprintln!("getEthToTokenOutputPrice err: {}", err);
                            });
                        eloop_handle_clone.spawn(eth_to_token_future);
                    }

                    Ok(())
                })
                .map_err(|_err| {
                    // TODO: better errors
                    // eprintln!("ether_balance err: {}", err);
                });

            eloop_handle.spawn(ether_balance_future);

            Ok(())
        })
        // TODO: better errors
        .map_err(|_err| {
            // eprintln!("token balance err: {}", err)
        })
}

// TODO: don't return Item = (). Instead, return the Address
fn get_orders_for_id(
    token_index: u64,
    eloop_handle: tokio_core::reactor::Handle,
    erc20_abi: &'static [u8],
    uniswap_exchange_abi: &'static [u8],
    uniswap_factory_contract: Rc<contract::Contract<web3::transports::WebSocket>>,
    w3: Rc<web3::Web3<web3::transports::WebSocket>>,
    // ) -> Box<Future<Item = Address, Error = web3::contract::Error> + Send> {
) -> impl Future<Item = (), Error = ()> {
    // println!("querying token index: {}", token_index);

    uniswap_factory_contract
        .query(
            "getTokenWithId",
            (token_index,),
            None,
            contract::Options::default(),
            None,
        )
        .and_then(move |uniswap_token_address: Address| {
            // println!("#{}: uniswap_token_address: {:#?}", token_index, uniswap_token_address);

            uniswap_factory_contract
                .query(
                    "getExchange",
                    (uniswap_token_address,),
                    None,
                    contract::Options::default(),
                    None,
                )
                // TODO: flatten this while keeping uniswap_token_address in scope
                .and_then(move |uniswap_exchange_address: Address| {
                    // println!("#{}: uniswap_token_address: {:#?}; uniswap_exchange_address: {:#?}", token_index, uniswap_token_address, uniswap_exchange_address);

                    // TODO: after adding this, i'm not seeing any block logs. I've missed block logs before though so maybe its unrelated
                    eloop_handle.spawn(subscribe_exchange_logs(
                        eloop_handle.clone(),
                        erc20_abi,
                        uniswap_exchange_abi,
                        uniswap_exchange_address,
                        uniswap_token_address,
                        w3.clone(),
                    ));

                    eloop_handle.spawn(check_prices(
                        eloop_handle.clone(),
                        erc20_abi,
                        uniswap_exchange_abi,
                        uniswap_exchange_address,
                        uniswap_token_address,
                        w3.clone(),
                    ));

                    Ok(())
                })
        })
        // TODO: better errors
        .map_err(|e| eprintln!("getTokenWithId err: {}", e))
}

fn query_existing_exchanges(
    eloop_handle: tokio_core::reactor::Handle,
    erc20_abi: &'static [u8],
    uniswap_exchange_abi: &'static [u8],
    uniswap_factory_contract: Rc<contract::Contract<web3::transports::WebSocket>>,
    w3: Rc<web3::Web3<web3::transports::WebSocket>>,
) -> impl Future<Item = (), Error = ()> {
    println!("querying existing exchanges...");

    // instead of fetching historic logs, get the exchanges by querying the contract
    // Get token count. (getTokenCount())
    // For each token in token count, get the address with id "i". (getTokenWithId(id))
    // For each token address, get the exchange address. (getExchange(token))
    uniswap_factory_contract
        .query("tokenCount", (), None, contract::Options::default(), None)
        .and_then(move |uniswap_token_count: U256| {
            let uniswap_token_count: u64 = uniswap_token_count.as_u64();
            println!("uniswap_token_count: {}", uniswap_token_count);

            // TODO: range over U256 instead of limited to u64
            for token_index in 1..=uniswap_token_count {
                // TODO: not sure about using eloop_handle here, but it seems to be working. i thought i was supposed to return the futures
                eloop_handle.spawn(get_orders_for_id(
                    token_index,
                    eloop_handle.clone(),
                    erc20_abi,
                    uniswap_exchange_abi,
                    uniswap_factory_contract.clone(),
                    w3.clone(),
                ));
            }

            Ok(())
        })
        .map_err(|e| eprintln!("uniswap exchange err: {}", e))

    // Box::new(the_future)
    // the_future
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
    env_logger::init();

    let mut eloop = tokio_core::reactor::Core::new().unwrap();
    let handle = eloop.handle();
    let w3 = Rc::new(web3::Web3::new(
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
    let _subscribe_new_heads_future = subscribe_new_heads(w3.clone());

    let subscribe_factory_logs_future = subscribe_factory_logs(w3.clone(), uniswap_factory_address);

    let uniswap_factory_contract = Rc::new(
        contract::Contract::from_json(w3.eth(), uniswap_factory_address, uniswap_factory_abi)
            .unwrap(),
    );
    println!(
        "contract deployed at: {:#?}",
        uniswap_factory_contract.address()
    );

    let query_existing_exchanges_future = query_existing_exchanges(
        handle.clone(),
        erc20_abi,
        uniswap_exchange_abi,
        uniswap_factory_contract,
        w3.clone(),
    );

    let all_futures = futures::future::lazy(|| {
        // subscribe_new_heads_future.join3(
        //     subscribe_factory_logs_future,
        //     query_existing_exchanges_future,
        // )
        subscribe_factory_logs_future.join(query_existing_exchanges_future)
    });
    if let Err(_err) = eloop.run(all_futures) {
        eprintln!("ERROR");
    }
}
