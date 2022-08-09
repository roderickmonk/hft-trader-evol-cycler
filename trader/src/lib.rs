extern crate chrono;
extern crate rustc_serialize;
extern crate trader_util;

#[allow(unused_imports)]
use chrono::prelude::*;
#[allow(unused_imports)]
use itertools::enumerate;
#[allow(unused_imports)]
use rustc_serialize::json::{self, Json, ToJson};
#[allow(unused_imports)]
use std::cmp;
use std::panic;
#[allow(unused_imports)]
use std::sync::{Arc, Mutex};
#[allow(unused_imports)]
use std::thread;
#[allow(unused_imports)]
use std::vec;

extern crate redis;

fn evol(
    _quantity_limit: f64,
    pv: &Vec<f64>,
    pdf_x: &Vec<f64>,
    pdf_y: &Vec<f64>,
    _pow10: f64,
    _evol: &mut Vec<f64>,
) {
    //
    assert_eq!(pdf_x.len(), pdf_y.len(), "Corrupt PDF");

    let log_pdf_x: Vec<f64> = pdf_x.into_iter().map(|pdf_x| pdf_x.log10()).collect();

    trader_util::interpolate(pv, &log_pdf_x, pdf_y, _evol);
}

fn maximize_profit(
    quantity_limit: f64,
    sell_ev: &Vec<f64>,
    sell_rates: &Vec<f64>,
    buy_ev: &Vec<f64>,
    buy_rates: &Vec<f64>,
    fee: f64,
) -> (f64, f64) {
    let mut buy_rate: f64 = 0.0;
    let mut sell_rate: f64 = 0.0;
    let mut max: f64 = 0.0;

    for (&_sell_ev, &_sell_rate) in sell_ev.iter().zip(sell_rates.iter()) {
        //
        for (&_buy_ev, &_buy_rate) in buy_ev.iter().zip(buy_rates.iter()) {
            //
            //let _cycle_rate = _sell_ev + _buy_ev;
            let _cycle_rate = quantity_limit / _sell_ev + quantity_limit / _buy_ev;

            let expected_profit: f64 =
                (_sell_rate * (1f64 - fee) - _buy_rate / (1f64 - fee)) / _cycle_rate;

            if expected_profit > max {
                max = expected_profit;
                buy_rate = _buy_rate;
                sell_rate = _sell_rate;
            }
        }
    }
    (buy_rate, sell_rate)
}

pub fn compute_orders(

    fee_rate: f64,
    tick: f64,
    quantity_limit: f64,
    pdf_x: &Vec<f64>,
    pdf_y: &Vec<f64>,
    in_buy_rates: &Vec<f64>,
    buy_quantities: &Vec<f64>,
    in_sell_rates: &Vec<f64>,
    sell_quantities: &Vec<f64>,
    hft_sim_testing: bool,
    allow_order_conflicts: bool,
) -> (f64, f64) {
    //
    let pow10 = trader_util::precision10(tick);

    panic::set_hook(Box::new(|info| {
        println!("panic happened: {}", info);
    }));

    if hft_sim_testing {
        println! ("Trader: evol_a_cycler");
        println! ("best buy: {}", in_buy_rates[0]);
        println! ("best sell: {}", in_sell_rates[0]);
        println! ("fee_rate: {}", fee_rate);
        println! ("tick: {}", tick);
        println! ("quantity_limit: {}", quantity_limit);
        println! ("hft_sim_testing: {}", hft_sim_testing);
        println! ("allow_order_conflicts: {}", allow_order_conflicts);
    }

    let mut buy_rates: Vec<f64> = Vec::with_capacity(in_buy_rates.len());
    let mut buy_pv: Vec<f64> = Vec::with_capacity(in_buy_rates.len());

    trader_util::get_pv_and_rates(
        &in_buy_rates,
        &buy_quantities,
        in_sell_rates[0],
        tick,
        pow10,
        allow_order_conflicts,
        &mut buy_rates,
        &mut buy_pv,
    );

    let mut sell_rates: Vec<f64> = Vec::with_capacity(in_buy_rates.len());
    let mut sell_pv: Vec<f64> = Vec::with_capacity(in_buy_rates.len());

    trader_util::get_pv_and_rates(
        &in_sell_rates,
        &sell_quantities,
        in_buy_rates[0],
        -tick,
        pow10,
        allow_order_conflicts,
        &mut sell_rates,
        &mut sell_pv,
    );

    let mut buy_ev: Vec<f64> = Vec::with_capacity(buy_rates.len());
    let mut sell_ev: Vec<f64> = Vec::with_capacity(sell_rates.len());

    evol(quantity_limit, &buy_pv, &pdf_x, &pdf_y, pow10, &mut buy_ev);

    evol(
        quantity_limit,
        &sell_pv,
        &pdf_x,
        &pdf_y,
        pow10,
        &mut sell_ev,
    );

    let result = maximize_profit(
        quantity_limit,
        &sell_ev,
        &sell_rates,
        &buy_ev,
        &buy_rates,
        fee_rate,
    );

    if hft_sim_testing {
        let _save_redis_result = trader_util::send2sim(
            pdf_x,
            pdf_y,
            in_buy_rates,
            buy_quantities,
            in_sell_rates,
            sell_quantities,
            result.0,
            result.1,
            &buy_rates,
            &sell_rates,
            &buy_ev,
            &sell_ev,
            &buy_pv,
            &sell_pv,
        );
    }

    result
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_evol_0() {
        //
        let quantity_limit = 0.1f64;
        let pv = vec![0f64, 10f64, 20f64, 30f64];
        let pdf_x = vec![0.1f64];
        let pdf_y = vec![1f64];
        let tick: f64 = 1e-8f64;
        let pow10 = trader_util::precision10(tick);
        let mut _evol: Vec<f64> = vec![];

        evol(quantity_limit, &pv, &pdf_x, &pdf_y, pow10, &mut _evol);
        println!("evol: {:?}", _evol);

        let expected_evol = vec![0.1f64, 0f64, 0f64, 0f64];
        println!("expected_evol: {:?}", &expected_evol);

        assert!(_evol == expected_evol);
    }

    #[test]
    fn test_evol_1() {
        //

        let quantity_limit = 0.1f64;
        let pv = vec![0f64, 10f64, 20f64, 30f64];
        let pdf_x = vec![0.1f64];
        let pdf_y = vec![1f64];
        let tick: f64 = 1e-8f64;
        let pow10 = trader_util::precision10(tick);
        let mut _evol: Vec<f64> = vec![];

        evol(quantity_limit, &pv, &pdf_x, &pdf_y, pow10, &mut _evol);
        println!("evol: {:?}", _evol);

        let expected_evol = vec![0.1f64, 0f64, 0f64, 0f64];
        println!("expected_evol\n{:?}", expected_evol);

        assert!(_evol == expected_evol);
    }

    #[test]
    fn test_evol_2() {
        //
        let quantity_limit = 0.1f64;
        let pv = vec![0f64, 10f64, 20f64, 30f64];
        let pdf_x = vec![0.1f64, 1.0f64];
        let pdf_y = vec![0.9f64, 0.1f64];
        let tick: f64 = 1e-8f64;
        let pow10 = trader_util::precision10(tick);
        let mut _evol: Vec<f64> = vec![];

        evol(quantity_limit, &pv, &pdf_x, &pdf_y, pow10, &mut _evol);
        println!("evol: {:?}", _evol);

        let expected_evol = vec![0.1f64, 0f64, 0f64, 0f64];
        println!("expected_evol\n{:?}", expected_evol);

        assert!(_evol == expected_evol);
    }

    #[test]
    fn test_evol_3() {
        //
        let quantity_limit = 0.1f64;
        let pv = vec![0f64, 10f64, 20f64, 30f64];
        let pdf_x = vec![0.1f64, 1.0f64, 2.0f64];
        let pdf_y = vec![0.8f64, 0.1f64, 0.1f64];
        let tick: f64 = 1e-8f64;
        let pow10 = trader_util::precision10(tick);
        let mut _evol: Vec<f64> = vec![];

        evol(quantity_limit, &pv, &pdf_x, &pdf_y, pow10, &mut _evol);
        println!("evol: {:?}", _evol);

        let expected_evol = vec![0.1f64, 0f64, 0f64, 0f64];
        println!("expected_evol\n{:?}", expected_evol);

        assert!(_evol == expected_evol);
    }

    #[test]
    fn test_evol_4() {
        //
        let quantity_limit = 0.05f64;
        let pv = vec![0f64, 10f64, 20f64, 30f64];
        let pdf_x = vec![0.1f64, 1.0f64, 2.0f64];
        let pdf_y = vec![0.8f64, 0.1f64, 0.1f64];
        let tick: f64 = 1e-8f64;
        let pow10 = trader_util::precision10(tick);
        let mut _evol: Vec<f64> = vec![];

        evol(quantity_limit, &pv, &pdf_x, &pdf_y, pow10, &mut _evol);
        println!("evol: {:?}", _evol);

        let expected_evol = vec![0.05f64, 0f64, 0f64, 0f64];
        println!("expected_evol\n{:?}", expected_evol);

        assert!(_evol == expected_evol);
    }

    #[test]
    fn test_evol_5() {
        //
        let quantity_limit = 0.05f64;
        let pv = vec![0f64, 0.05f64, 0.10f64, 0.15f64];
        let pdf_x = vec![0.1f64, 1.0f64];
        let pdf_y = vec![0.5f64, 0.5f64];
        let tick: f64 = 1e-8f64;
        let pow10 = trader_util::precision10(tick);
        let mut _evol: Vec<f64> = vec![];

        evol(quantity_limit, &pv, &pdf_x, &pdf_y, pow10, &mut _evol);

        println!("evol: {:?}", _evol);
        let expected_evol = vec![0.05f64, 0.05f64, 0.025f64, 0.025f64];
        println!("expected_evol\n{:?}", expected_evol);

        assert!(_evol == expected_evol);
    }

    #[test]
    fn test_evol_6() {
        //
        let quantity_limit = 0.05f64;
        let pv = vec![0f64, 0.05f64, 0.10f64, 0.15f64];
        let pdf_x = vec![0.1f64, 1.0f64];
        let pdf_y = vec![0.5f64, 0.25f64];
        let tick: f64 = 1e-8f64;
        let pow10 = trader_util::precision10(tick);
        let mut _evol: Vec<f64> = vec![];

        evol(quantity_limit, &pv, &pdf_x, &pdf_y, pow10, &mut _evol);
        println!("evol: {:?}", _evol);

        let expected_evol = vec![0.0375f64, 0.0375f64, 0.0125f64, 0.0125f64];

        assert!(_evol == expected_evol);
    }

    #[test]
    fn test_evol_7() {
        //
        let quantity_limit = 0.06f64;
        let pv = vec![0.05, 0.05, 0.05, 0.05];
        let pdf_x = vec![
            0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1,
        ];
        let pdf_y = vec![
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
        ];
        let tick: f64 = 1e-8f64;
        let pow10 = trader_util::precision10(tick);
        let mut _evol: Vec<f64> = vec![];

        evol(quantity_limit, &pv, &pdf_x, &pdf_y, pow10, &mut _evol);
        println!("evol: {:?}", _evol);

        let expected_evol = vec![0.8, 0.8, 0.8, 0.8];
        println!("expected_evol\n{:?}", expected_evol);

        assert!(_evol == expected_evol);
    }

    #[test]
    fn test_evol_8() {
        //
        let quantity_limit = 0.06f64;
        let pv = vec![0.15, 0.15, 0.15, 0.15];
        let pdf_x = vec![
            0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1,
        ];
        let pdf_y = vec![
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
        ];
        let tick: f64 = 1e-8f64;
        let pow10 = trader_util::precision10(tick);
        let mut _evol: Vec<f64> = vec![];

        evol(quantity_limit, &pv, &pdf_x, &pdf_y, pow10, &mut _evol);
        println!("evol: {:?}", _evol);
        let expected_evol = vec![0.0, 0.0, 0.0, 0.0];
        println!("expected_evol\n{:?}", expected_evol);

        assert!(_evol == expected_evol);
    }

    #[test]
    fn test_evol_9() {
        //
        let mut handles = vec![];

        let quantity_limit = 0.06f64;
        let pv = vec![0.15, 0.15, 0.15, 0.15];
        let pdf_x = vec![
            0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1,
        ];
        let pdf_y = vec![
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
        ];
        let tick: f64 = 1e-8f64;
        let pow10 = trader_util::precision10(tick);
        let expected_evol = vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

        let pv_arc = Arc::new(Mutex::new(pv));
        let pdf_x_arc = Arc::new(Mutex::new(pdf_x));
        let pdf_y_arc = Arc::new(Mutex::new(pdf_y));
        let evol_arc = Arc::new(Mutex::new(vec![]));

        {
            let pv = Arc::clone(&pv_arc);
            let pdf_x = Arc::clone(&pdf_x_arc);
            let pdf_y = Arc::clone(&pdf_y_arc);
            let _evol = Arc::clone(&evol_arc);

            let handle = thread::spawn(move || {
                evol(
                    quantity_limit,
                    &pv.lock().unwrap(),
                    &pdf_x.lock().unwrap(),
                    &pdf_y.lock().unwrap(),
                    pow10,
                    &mut _evol.lock().unwrap(),
                );
            });
            handles.push(handle);
        }

        {
            let _evol = Arc::clone(&evol_arc);
            let pv = Arc::clone(&pv_arc);
            let pdf_x = Arc::clone(&pdf_x_arc);
            let pdf_y = Arc::clone(&pdf_y_arc);
            let _evol = Arc::clone(&evol_arc);

            let handle = thread::spawn(move || {
                evol(
                    quantity_limit,
                    &pv.lock().unwrap(),
                    &pdf_x.lock().unwrap(),
                    &pdf_y.lock().unwrap(),
                    pow10,
                    &mut _evol.lock().unwrap(),
                );
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let mut _evol = evol_arc.lock().unwrap().clone();

        println!("expected_evol\n{:?}", expected_evol);
        println!("evol:\n{:?}", _evol);

        assert!(_evol == expected_evol);
    }

    #[test]
    fn test_evol_10() {
        //
        let quantity_limit = 0.06f64;
        let pv = vec![0.05, 0.05, 0.05, 0.05];
        let pdf_x = vec![
            0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1,
        ];
        let pdf_y = vec![
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
        ];
        let tick: f64 = 1e-8f64;
        let pow10 = trader_util::precision10(tick);
        let expected_evol = vec![0.8, 0.8, 0.8, 0.8];

        let pv_arc = Arc::new(Mutex::new(pv));
        let pdf_x_arc = Arc::new(Mutex::new(pdf_x));
        let pdf_y_arc = Arc::new(Mutex::new(pdf_y));
        let evol_arc = Arc::new(Mutex::new(vec![]));

        let mut handles = vec![];

        {
            let _evol = Arc::clone(&evol_arc);
            let pv = Arc::clone(&pv_arc);
            let pdf_x = Arc::clone(&pdf_x_arc);
            let pdf_y = Arc::clone(&pdf_y_arc);
            let _evol = Arc::clone(&evol_arc);

            let handle = thread::spawn(move || {
                evol(
                    quantity_limit,
                    &pv.lock().unwrap(),
                    &pdf_x.lock().unwrap(),
                    &pdf_y.lock().unwrap(),
                    pow10,
                    &mut _evol.lock().unwrap(),
                );
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let mut _evol = evol_arc.lock().unwrap().clone();

        println!("expected_evol\n{:?}", expected_evol);

        assert!(_evol == expected_evol);
    }

    #[test]
    fn test_compute_orders_0() {
        //
        let fee_rate = 0.002;
        let tick = 1e-8f64;
        let quantity_limit = 0.1f64;
        let pdf_x = vec![0.01f64, 0.1f64];
        let pdf_y = vec![0.8f64, 0.2f64];
        let buy_rates = vec![0.011f64, 0.010f64];
        let buy_quantities = vec![0.05f64, 0.1f64];
        let sell_rates = vec![0.012f64, 0.013f64];
        let sell_quantities = vec![0.05f64, 0.1f64];

        let (buy_rate, sell_rate) = compute_orders(
            fee_rate,
            tick,
            quantity_limit,
            &pdf_x,
            &pdf_y,
            &buy_rates,
            &buy_quantities,
            &sell_rates,
            &sell_quantities,
            false,
            false,
        );

        println!("buy_rate: {}", buy_rate);
        println!("sell_rate: {}", sell_rate);

        assert!(buy_rate == 0.01000001f64);
        assert!(sell_rate == 0.01299999f64);
    }

    #[test]
    fn test_compute_orders_1() {
        //
        let fee_rate = 0.02;
        let tick = 1e-6f64;
        let quantity_limit = 0.1f64;
        let pdf_x = vec![0.01f64, 0.1f64];
        let pdf_y = vec![0.8f64, 0.2f64];
        let buy_rates = vec![0.011f64, 0.010f64];
        let buy_quantities = vec![0.05f64, 0.1f64];
        let sell_rates = vec![0.012f64, 0.013f64];
        let sell_quantities = vec![0.05f64, 0.1f64];

        let (buy_rate, sell_rate) = compute_orders(
            fee_rate,
            tick,
            quantity_limit,
            &pdf_x,
            &pdf_y,
            &buy_rates,
            &buy_quantities,
            &sell_rates,
            &sell_quantities,
            false,
            false,
        );

        println!("buy_rate: {}", buy_rate);
        println!("sell_rate: {}", sell_rate);

        assert!(buy_rate == 0.010001f64);
        assert!(sell_rate == 0.012999f64);
    }
}
