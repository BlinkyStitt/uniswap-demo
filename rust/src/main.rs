use std::time;
use web3::contract::Contract;
use web3::futures::{Future, Stream};
use web3::types::{Address, FilterBuilder};

fn main() {
    let mut eloop = tokio_core::reactor::Core::new().unwrap();
    let web3 = web3::Web3::new(
        // TODO: read the websocket uri from an environment variable. default to localhost
        web3::transports::WebSocket::with_event_loop("ws://127.0.0.1:8546", &eloop.handle())
            .unwrap(),
    );

    // TODO: read these from env vars so we can use a development blockchain easily
    let uniswap_genesis_block = 6_627_917;
    let uniswap_factory_address: Address =
        "c0a47dfe034b400b47bdad5fecda2621de6c4d95".parse().unwrap();

    let uniswap_factory_abi: &[u8] = include_bytes!("uniswap_factory.abi");

    let web3_futures = web3.eth().accounts().then(|accounts| {
        let accounts = accounts.unwrap();
        println!("accounts: {:#?}", &accounts);

        let uniswap_factory_contract =
            Contract::from_json(web3.eth(), uniswap_factory_address, uniswap_factory_abi).unwrap();

        println!(
            "contract deployed at: {:#?}",
            uniswap_factory_contract.address()
        );

        // log new blocks
        // TODO: subscribe to syncing instead of actual blocks
        let blocks_future = web3
            .eth_subscribe()
            .subscribe_new_heads()
            .then(|sub| {
                sub.unwrap().for_each(|log| {
                    println!("got block log: {:?}", log);
                    Ok(())
                })
            })
            .map_err(|_| ());

        // TODO: subscribe to sync status. if we are behind by more than X blocks, give a notice

        // Filter for NewExchange(address,address) event on the uniswap factory contract
        let factory_filter_builder = FilterBuilder::default().address(vec![uniswap_factory_contract.address()]);
            // .topics(
            //     Some(vec![
            //         "0x9d42cb017eb05bd8944ab536a8b35bc68085931dd5f4356489801453923953f9".into(),
            //     ]),
            //     None,
            //     None,
            //     None,
            // )

        println!("factory_filter defaults: {:#?}", factory_filter_builder.build());

        // notifications are send for current events and not for past events
        // TODO: maybe from_block should be the current block number? or just skip it entirely?
        let factory_future_new_logs = web3
            .eth_subscribe()
            .subscribe_logs(factory_filter_builder.build())
            .then(|sub| {
                sub.unwrap().for_each(|log| {
                    println!("got uniswap factory log from subscription: {:?}", log);
                    // TODO: get the exchange and token addresses out of the log
                    // TODO: subscribe to the exchange logs. if we get any, print the current price on the exchange
                    // TODO: if sync status is behind, alert that the price is old
                    Ok(())
                })
            })
            // TODO: proper error handling
            .map_err(|_| ());

        // TODO: put a to_block on the filter since we subscribe to new logs with factory_future_new?
        // TODO: fetching historic logs seems to be not working. if it is just very slow, maybe we should step through the exchanges by numeric id
        let factory_future_past_logs = web3
            .eth_filter()
            .create_logs_filter(factory_filter_builder.from_block(uniswap_genesis_block.into()).build())
            .then(|filter| {
                filter
                    .unwrap()
                    .stream(time::Duration::from_secs(0))
                    .for_each(|log| {
                        println!("got uniswap factory log from filter: {:?}", log);
                        // TODO: handle the log like factory_future_new does
                        Ok(())
                    })
            })
            // TODO: proper error handling
            .map_err(|_| ());
        // TODO: instead of fetching historic logs

        blocks_future.join3(factory_future_new_logs, factory_future_past_logs)
    });

    eloop.run(web3_futures).unwrap();
}
