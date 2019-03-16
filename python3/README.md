# Uniswap Watcher in Python

## Development

Setup:

    virtualenv -p python3 env
    . ./env/bin/activate
    pip install -r requirements.txt

Run:

    ./env/bin/python go.py

## Problems

This occassionally errors with `concurrent.futures._base.TimeoutError` when calling `get_all_entries`. Maybe querying logs isn't as reliable as I need.
