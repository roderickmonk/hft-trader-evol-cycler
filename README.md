# High Frequency Trader - Evol Cycler

## Introduction ##
This repo contains the implementation of a deterministic HFT trader.  Its prime input is the market's orderbook, although it also has a number of other input considerations (parameters) as well.  

It is meant to be called from NodeJS using the Neon binder (https://neon-bindings.com/).  NodeJS is primarily for IO bound processing and the trading decisions must be made in < 20 msecs, hence the need for NodeJS to drop into a language such as Rust (or C or C++) for the actual trading decisions (one for the buy side and one for the sell side).

Little information is provided here describing the specifics of the trading decision process, as the prime focus for this repo is to demonstrate a typical NEON / Rust usage (albeit a careful study of the code would reveal all, if the reader so desired).

En route and if configured so, the code is capable of the following:
1. Recording the following to an instance of Redis: the current orderbook, all other input parameters, and the resulting buy/sell decision (note: a buy / sell decision of (-1, -1) means to cancel any existing orders from both sides).
2. Publishing the buy/sell decision to Redis (a bespoke 'backtester' subscribes to such Redis messaging; the backtester is not discussed further here).

## Crates ##
This repo contains 3 crates:
### native ###
`native` is the executive crate which carries out the formalisms required by Neon.  Ultimately it creates an export referred to as `EngineEvol` which in turn calls upon a task referred to as `ComputerOrdersTask`.  A `task` is a concept defined by Neon.  Within `ComputerOrdersTask` find a function `compute` which in turn calls the routine trader::compute_orders (found in the `trader` crate which is discussed in the next section).  Note that a fair amount of the code found in this crate is given over to the marshalling of the input parameters into a form usable from the subsequent Rust code.

### trader ###
This routine contains the specifics of the code for the Evol Cycler trader.  Little attempt is made to explain it here, but do not that it calls upon the trader-util crate for a number of utility functions (which are discussed next).

The public function, called from `native` is the routine `compute_orders`.  It in turn calls:
* trader_util::get_pv_and_rates()
* evol() # A local function specific to this trader
* maximize_profit() # A local function specific to this trader
  
A number of test routines are also provided.
### trader-util
This crate contains a number of general purpose utility functions.  These are:
* get_pv_and_rates ()
* A binary search routine
* A 2D interpolation routine
* A routine to save to a Redis instance
* A routine to publish to an instance of Redis Pub/Sub 

A number of test routines are also provided.

