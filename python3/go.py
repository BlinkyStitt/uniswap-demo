#!/usr/bin/env python3
from web3.auto import w3

print(w3.eth.blockNumber);

if (w3.eth.blockNumber == 0):
    raise Exception("Syncing")

print(w3.eth.getBlock('latest'));

abi = [{"name": "NewExchange", "inputs": [{"type": "address", "name": "token", "indexed": True}, {"type": "address", "name": "exchange", "indexed": True}], "anonymous": False, "type": "event"}]

uniswap = w3.eth.contract('0xc0a47dFe034B400B47bDaD5FecDa2621de6c4d95', abi=abi)

past_events = uniswap.events.NewExchange.createFilter(fromBlock=6627917).get_all_entries()

# TODO: subscribe to future events, too

token_exchange = {e.args.token: e.args.exchange for e in past_events}

for token, exchange in token_exchange.items():
    print(token, exchange)
