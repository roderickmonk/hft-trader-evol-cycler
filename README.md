# High Frequency Trader - Evol Cycler

## Introduction ##
This repo contains the implementation of a deterministic HFT trader.  Its prime input is the current Orderbook, although it also has a number of other input considerations (parameters) as well.  

It is meant to be called from NodeJS using the Neon binder (https://neon-bindings.com/).  Given that NodeJS is primarily for IO bound processing and the trading decisions must be made in < 20 msecs, hence the need for NodeJS to drop into a language such as Rust (or C or C++) for the actual trading decisions (one for the buy side and one for the sell side).

Little information is provided here describing the specifics of the trading decision process, as the prime focus for this repo is to demonstration a typical NEON / Rust usage (albeit a careful study of the code would reveal all, if the reader so desired).