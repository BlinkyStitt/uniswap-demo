// TODO: proper error handling. don't unwrap
// use std::time;
use std::sync::Arc;
use web3::contract;
use web3::futures::{Future, Stream};
use web3::types::{Address, FilterBuilder, U256};

fn main() {
    let mut eloop = tokio_core::reactor::Core::new().unwrap();
    let web3 = Arc::new(web3::Web3::new(
        // TODO: read the websocket uri from an environment variable. default to localhost
        web3::transports::WebSocket::with_event_loop("ws://127.0.0.1:8546", &eloop.handle())
            .unwrap(),
    ));

    // TODO: read these from env vars so we can use a development blockchain easily
    // let uniswap_genesis_block = 6_627_917;
    let uniswap_factory_address: Address =
        "c0a47dfe034b400b47bdad5fecda2621de6c4d95".parse().unwrap();

    // chain:   address                                  block height
    // mainnet: c0a47dfe034b400b47bdad5fecda2621de6c4d95 6_627_917

    let erc20_abi: &[u8] = include_bytes!("erc20.abi");
    let uniswap_factory_abi: &[u8] = include_bytes!("uniswap_factory.abi");
    let uniswap_exchange_abi: &[u8] = include_bytes!("uniswap_exchange.abi");

    let web3_futures = web3.eth().accounts().then(|accounts| {
        let accounts = accounts.unwrap();
        println!("accounts: {:#?}", accounts);

        let uniswap_factory_contract = Arc::new(
            contract::Contract::from_json(web3.eth(), uniswap_factory_address, uniswap_factory_abi)
                .unwrap(),
        );
        println!(
            "contract deployed at: {:#?}",
            uniswap_factory_contract.address()
        );

        // log new blocks
        // TODO: subscribe to sync status. if we are behind by more than X blocks, give a notice. except ganache doesn't support that
        let blocks_future = web3
            .eth_subscribe()
            .subscribe_new_heads()
            .and_then(|sub| {
                sub.for_each(|log| {
                    // TODO: wtf. this isn't ever printing anything now...
                    println!("got block log: {:?}", log);
                    Ok(())
                })
            })
            .map_err(|e| eprintln!("block log err: {:#?}", e));

        // Filter for NewExchange(address,address) event on the uniswap factory contract
        // TODO: i think the contract has a helper for this
        let factory_filter_builder =
            FilterBuilder::default().address(vec![uniswap_factory_contract.address()]);
        // println!(
        //     "factory_filter defaults: {:#?}",
        //     factory_filter_builder.build()
        // );

        // notifications are send for current events and not for past events
        // TODO: maybe from_block should be the current block number? or just skip it entirely?
        let factory_future_new_logs = web3
            .eth_subscribe()
            .subscribe_logs(factory_filter_builder.build())
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

        // TODO: put a to_block on the filter since we subscribe to new logs with factory_future_new?
        // TODO: fetching historic logs seems to be not working. if it is just very slow, maybe we should step through the exchanges by numeric id
        // let factory_future_past_logs = web3
        //     .eth_filter()
        //     .create_logs_filter(factory_filter_builder.from_block(uniswap_genesis_block.into()).build())
        //     .then(|filter| {
        //         filter
        //             .unwrap()
        //             .stream(time::Duration::from_secs(0))
        //             .for_each(|log| {
        //                 println!("got uniswap factory log from filter: {:?}", log);
        //                 // TODO: handle the log like factory_future_new does
        //                 Ok(())
        //             })
        //     })
        //     // TODO: proper error handling
        //     .map_err(|e| eprintln!("uniswap log err: {:?}", e));

        // instead of fetching historic logs, get the exchanges by querying the contract
        // Get token count. (getTokenCount())
        // For each token in token count, get the address with id "i". (getTokenWithId(id))
        // For each token address, get the exchange address. (getExchange(token))
        // TODO: do this async
        let factory_future_existing_exchanges = uniswap_factory_contract
            .query("tokenCount", (), None, contract::Options::default(), None)
            .and_then(move |uniswap_token_count: U256| {
                println!("uniswap_token_count: {}", uniswap_token_count);
                let uniswap_token_count: u64 = uniswap_token_count.as_u64();

                // TODO: range over U256 instead of limited to u64
                let token_address_futures: Vec<_> = (1..=uniswap_token_count)
                    .map(|token_index| {
                        // the borrow checker and closures means we need to use Arcs
                        let web3 = web3.clone();
                        let uniswap_factory_contract = uniswap_factory_contract.clone();

                        uniswap_factory_contract
                            .query(
                                "getTokenWithId",
                                (token_index,),
                                None,
                                contract::Options::default(),
                                None,
                            )
                            .and_then(move |uniswap_token_address: Address| {
                                // println!("uniswap_token_address: {:#?}", uniswap_token_address);

                                uniswap_factory_contract
                                    .query(
                                        "getExchange",
                                        (uniswap_token_address,),
                                        None,
                                        contract::Options::default(),
                                        None,
                                    )
                                    // TODO: flatten this
                                    .and_then(move |uniswap_exchange_address: Address| {
                                        println!("uniswap_token_address: {:#?}; uniswap_exchange_address: {:#?}", uniswap_token_address, uniswap_exchange_address);

                                        let erc20_contract =
                                            contract::Contract::from_json(
                                                web3.eth(),
                                                uniswap_token_address,
                                                erc20_abi,
                                            )
                                            .unwrap();

                                        // check the reserves so we can pick valid order sizes
                                        erc20_contract.query(
                                            "balanceOf",
                                            (uniswap_exchange_address, ),
                                            None,
                                            contract::Options::default(),
                                            None,
                                        ).and_then(move |token_supply: U256| {
                                            println!("uniswap_token_address: {:#?}; uniswap_exchange_address: {:#?}; token supply: {}", uniswap_token_address, uniswap_exchange_address, token_supply);

                                            if token_supply == 0.into() {
                                                // if no supply, skip this exchange
                                                // TODO: what kind of error can we actually raise here?
                                                // https://tokio.rs/docs/futures/combinators/#returning-from-multiple-branches
                                                // panic!("what can i return here that won't break the futures?")
                                                // Box::new(futures::future::err("token supply is 0. Skipping"))
                                                // futures::future::Either::A(Ok(()))
                                                // Ok(())
                                            }

                                            web3.eth().balance(uniswap_exchange_address, None).and_then(move |ether_supply: U256| {
                                                println!("uniswap_token_address: {:#?}; uniswap_exchange_address: {:#?}; token supply: {}, ether_balance: {:#?}", uniswap_token_address, uniswap_exchange_address, token_supply, ether_supply);

                                                let _uniswap_exchange_contract =
                                                    contract::Contract::from_json(
                                                        web3.eth(),
                                                        uniswap_exchange_address,
                                                        uniswap_exchange_abi,
                                                    )
                                                    .unwrap();

                                                    // TODO: getTokenToEthInputPrice? getTokenToEthOutputPrice? getEthToTokenInputPrice? getEthToTokenOutputPir

                                                Ok(())
                                            }).or_else(|err| {
                                                eprintln!("ether_balance err: {:#?}", err);
                                                Ok(())
                                            })
                                        }).or_else(move |err| {
                                            // if we got an error, skip this exchange
                                            eprintln!("{:#?}.balanceOf({:#?}) failed: {:#?}", uniswap_token_address, uniswap_exchange_address, err);
                                            Ok(())
                                        })
                                    })
                            })
                    })
                    .collect();

                // TODO: i think we might need a map/map_err here 
                futures::future::join_all(token_address_futures)
            })
            .map_err(|e| eprintln!("uniswap exchange err: {:#?}", e));

        // blocks_future
        blocks_future.join3(factory_future_new_logs, factory_future_existing_exchanges)
        // factory_future_new_logs.join(factory_future_existing_exchanges)
        // balance_future.join4(blocks_future, factory_future_new_logs, factory_future_existing_exchanges)
    });

    if let Err(e) = eloop.run(web3_futures) {
        eprintln!("ERROR! {:#?}", e);
    };
}
