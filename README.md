# High Frequency Trader - Evol Cycler

## Introduction ##
This repo contains the implementation of a deterministic HFT trader.  Its prime input is the market's orderbook, although it also has a number of other input considerations (parameters) as well.  

It is meant to be called from NodeJS using the Neon binder (https://neon-bindings.com/).  NodeJS is primarily for IO bound processing and the trading decisions must be made in < 20 msecs, hence the need for NodeJS to drop into a language such as Rust (or C or C++) for the actual trading decisions (one for the buy side and one for the sell side).

Little information is provided here describing the specifics of the trading decision process, as the prime focus for this repo is to demonstrate a typical NEON / Rust usage (albeit a careful study of the code would reveal all, if the reader so desired).

En route and if configured so, the code is capable of the following:
1. Record the following to an instance of Redis: the current orderbook, all other input parameters, and the resulting buy/sell decision (note: a buy / sell decision of (-1, -1) means to cancel any existing orders from both sides).
2. Publish the buy/sell decision to Redis (from which a bespoke backtester is listening; the backtester is not discussed further here).

