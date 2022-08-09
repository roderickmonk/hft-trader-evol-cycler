extern crate chrono;
extern crate redis;
extern crate rustc_serialize;

use chrono::prelude::*;
use itertools::enumerate;
#[allow(unused_imports)]
use rustc_serialize::json::{self, Json, ToJson};
#[allow(unused_imports)]
use std::cmp;
use std::panic;

fn expire_key(con: &redis::Connection, key: &String) -> redis::RedisResult<()> {
    let _: () = redis::cmd("EXPIRE").arg(key).arg(300).query(con)?;
    Ok(())
}

fn save2redis(
    con: &redis::Connection,
    cycle_time: &String,
    key: &str,
    data: &Vec<f64>,
) -> redis::RedisResult<()> {
    //
    let data_key = [&cycle_time, key].join(":");
    let mut data_string = "".to_string();

    for x in data {
        data_string = data_string + &x.to_string() + ",";
    }

    let _: () = redis::cmd("SET")
        .arg(&data_key)
        .arg(data_string)
        .query(con)?;
    let _ = expire_key(con, &data_key);

    Ok(())
}

pub fn send2sim(
    pdf_x: &Vec<f64>,
    pdf_y: &Vec<f64>,
    buy_rates: &Vec<f64>,
    buy_quantities: &Vec<f64>,
    sell_rates: &Vec<f64>,
    sell_quantities: &Vec<f64>,
    buy_rate: f64,
    sell_rate: f64,
    buy_candidate_rates: &Vec<f64>,
    sell_candidate_rates: &Vec<f64>,
    buy_ev: &Vec<f64>,
    sell_ev: &Vec<f64>,
    buy_pv: &Vec<f64>,
    sell_pv: &Vec<f64>,
) -> redis::RedisResult<()> {
    //
    let client = redis::Client::open("redis://127.0.0.1:6379/")?;
    let con = client.get_connection()?;

    let cycle_time = Utc::now().format("%Y-%m-%dT%H:%M:%S%.6f").to_string();

    let _ = save2redis(&con, &cycle_time, "pdf_x", &pdf_x);
    let _ = save2redis(&con, &cycle_time, "pdf_y", &pdf_y);
    let _ = save2redis(&con, &cycle_time, "buy_rates", &buy_rates);
    let _ = save2redis(&con, &cycle_time, "sell_rates", &sell_rates);
    let _ = save2redis(&con, &cycle_time, "buy_quantities", &buy_quantities);
    let _ = save2redis(&con, &cycle_time, "sell_quantities", &sell_quantities);
    let _ = save2redis(
        &con,
        &cycle_time,
        "buy_candidate_rates",
        &buy_candidate_rates,
    );
    let _ = save2redis(
        &con,
        &cycle_time,
        "sell_candidate_rates",
        &sell_candidate_rates,
    );
    let _ = save2redis(&con, &cycle_time, "buy_ev", &buy_ev);
    let _ = save2redis(&con, &cycle_time, "sell_ev", &sell_ev);

    let _ = save2redis(&con, &cycle_time, "buy_pv", &buy_pv);
    let _ = save2redis(&con, &cycle_time, "sell_pv", &sell_pv);

    #[derive(RustcDecodable, RustcEncodable)]
    struct SendStruct {
        cycle_time: String,
        buy_rate: f64,
        sell_rate: f64,
    };

    let send_struct = SendStruct {
        cycle_time,
        buy_rate,
        sell_rate,
    };

    let _: () = redis::cmd("PUBLISH")
        .arg("hft")
        .arg(json::encode(&send_struct).unwrap())
        .query(&con)?;

    Ok(())
}

pub fn binary_search(x: &Vec<f64>, y: &Vec<f64>, value: f64) -> ((f64, f64), (f64, f64)) {
    //
    let mut first = 0;
    let mut last = x.len() - 1;

    if value < x[0] {
        //
        ((x[first], x[first + 1]), (y[first], y[first + 1]))
    } else if x[last] <= value {
        //
        ((x[last - 1], x[last]), (y[last - 1], y[last]))
    } else {
        //
        let mut _midpoint = 0;

        loop {
            //
            _midpoint = (first + last) / 2;

            if x[_midpoint] <= value && value < x[_midpoint + 1] {
                break;
            } else {
                if value < x[_midpoint] {
                    last = _midpoint;
                } else {
                    first = _midpoint + 1;
                }
            }
        }

        (
            (x[_midpoint], x[_midpoint + 1]),
            (y[_midpoint], y[_midpoint + 1]),
        )
    }
}

pub fn interpolate(pv: &Vec<f64>, x: &Vec<f64>, y: &Vec<f64>, out: &mut Vec<f64>) {
    //
    for pv in pv {
        // Find the interval within which pv belongs

        if *pv < x[0] {
            out.push(y[0]);
        } else if x[x.len() - 1] < *pv {
            out.push(y[y.len() - 1]);
        } else {
            //
            let (x, y) = binary_search(&x, &y, *pv);

            // Interpolate
            let m = (y.1 - y.0) / (x.1 - x.0);
            let b = y.0 - m * x.0;
            out.push(m * pv + b);
        }
    }
}

pub fn candidate_rates(
    rates: &Vec<f64>,
    tick: f64,
    pow10: f64,
    allow_order_conflicts: bool,
    _candidate_rates: &mut Vec<f64>,
) {
    let not_equal = |x0: f64, x1: f64| {
        if round((x0 - x1).abs(), pow10) < tick.abs() {
            false
        } else {
            true
        }
    };

    if allow_order_conflicts {
        for i in (1..rates.len()).rev() {
            _candidate_rates.push(round(rates[i] + tick, pow10));
        }
    } else {
        for i in (1..rates.len()).rev() {
            if not_equal(rates[i] + tick, rates[i - 1]) {
                _candidate_rates.push(round(rates[i] + tick, pow10));
            }
        }
    }

    // Always include 'rates[0] + tick'
    _candidate_rates.push(round(rates[0] + tick, pow10));
}

#[allow(dead_code)]
pub fn preceding_volume(tick: f64, rate: f64, rates: &Vec<f64>, cum_sum: &Vec<f64>) -> f64 {
    let mut volume: f64 = 0f64;

    if tick > 0f64 {
        for (i, &r) in rates.iter().enumerate() {
            if rate > r {
                break;
            } else {
                volume = cum_sum[i];
            }
        }
    } else {
        for (i, &r) in enumerate(rates) {
            if rate < r {
                break;
            } else {
                volume = cum_sum[i];
            }
        }
    }

    volume
}

#[inline(always)]
pub fn cumulative_sum(rates: &Vec<f64>, quantities: &Vec<f64>, pow10: f64) -> Vec<f64> {
    //
    let mut cumulative: f64 = 0f64;
    let mut cumulative_sum: Vec<f64> = Vec::with_capacity(quantities.len());

    for (&r, &q) in rates.iter().zip(quantities.iter()) {
        cumulative += r * q;
        cumulative_sum.push(round(cumulative, pow10));
    }
    cumulative_sum
}

#[inline(always)]
pub fn get_pv_and_rates(
    rates: &Vec<f64>,
    quantities: &Vec<f64>,
    opposite_best: f64,
    tick: f64,
    pow10: f64,
    allow_order_conflicts: bool,
    _candidate_rates: &mut Vec<f64>,
    pv: &mut Vec<f64>,
) {
    //
    candidate_rates(rates, tick, pow10, allow_order_conflicts, _candidate_rates);

    let cum_sum = cumulative_sum(rates, quantities, pow10);
    assert_eq!(cum_sum.len(), quantities.len());

    for r in _candidate_rates.clone() {
        pv.push(preceding_volume(tick, r, &rates.to_vec(), &cum_sum));
    }

    let last = _candidate_rates.len() - 1;

    if _candidate_rates[last] == opposite_best {
        _candidate_rates.remove(last);
        pv.remove(last);
    }
}

#[allow(dead_code)]
pub fn cmp_vectors(a: &Vec<f64>, b: &Vec<f64>, tolerance: f64) -> bool {
    let match_count = a
        .iter()
        .zip(b.iter())
        .filter(|&(x, y)| (x - y).abs() < tolerance)
        .count();
    match_count == a.len() && match_count == b.len()
}

pub fn round(x: f64, pow10: f64) -> f64 {
    (x * pow10).round() / pow10
}

#[allow(dead_code)]
pub fn precision10(tick: f64) -> f64 {
    10f64.powi(-tick.abs().log10().round() as i32)
}

#[derive(Debug)]
struct BotConfiguration {
  fee_rate: f64,
  quantity_limit: f64,
  tick: f64,
  allow_order_conflicts: bool,
  hft_sim_testing: bool,
}

#[allow(dead_code)]
fn load_configuration(r: &redis::Connection, bot_key: &str) -> BotConfiguration {
  let keys: Vec<String> = vec![
    "feeRate".to_string(),
    "quantityLimit".to_string(),
    "tick".to_string(),
    "allowOrderConflicts".to_string(),
    "hftSimTesting".to_string(),
  ];

  let config_data: Vec<String> = redis::cmd("HMGET")
    .arg(bot_key.clone())
    .arg(keys)
    .query(r)
    .unwrap();

  let bot_config = BotConfiguration {
    fee_rate: config_data[0].parse().unwrap(),
    quantity_limit: config_data[1].parse().unwrap(),
    tick: config_data[2].parse().unwrap(),
    allow_order_conflicts: config_data[3].parse().unwrap(),
    hft_sim_testing: config_data[4].parse().unwrap(),
  };

  bot_config
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_co1_buy_get_pv_and_rates_1() {
        //
        let r = vec![0.4f64, 0.3f64, 0.2f64, 0.1f64];
        let q = vec![10f64, 20f64, 30f64, 40f64];
        let tick = 0.01f64;
        let pow10 = precision10(tick);

        let mut rates: Vec<f64> = vec![];
        let mut pv: Vec<f64> = vec![];

        get_pv_and_rates(
            &r,
            &q,
            std::f64::INFINITY,
            tick,
            pow10,
            false,
            &mut rates,
            &mut pv,
        );

        println!("result pv: {:?}", pv);
        println!("result rates: {:?}", rates);

        let mut expected_pv = vec![0.0, 4.0, 10.0, 16.0];
        expected_pv.reverse();

        let mut expected_rates = vec![0.41f64, 0.31f64, 0.21f64, 0.11f64];
        expected_rates.reverse();

        println!("expected_rates: {:?}", expected_rates);

        assert!(pv == expected_pv);
        assert!(rates == expected_rates);
    }

    #[test]
    fn test_candidate_rates_0() {
        //
        let rates = vec![0.4, 0.3, 0.2, 0.1];
        let tick = 1e-8;
        let pow10 = precision10(tick);

        let mut expected_candidate_rates = vec![0.40000001, 0.30000001, 0.20000001, 0.10000001];
        expected_candidate_rates.reverse();

        let mut _candidate_rates: Vec<f64> = vec![];

        let _ = candidate_rates(&rates, tick, pow10, false, &mut _candidate_rates);

        assert_eq!(_candidate_rates, expected_candidate_rates);
    }

    #[test]
    fn test_candidate_rates_1() {
        //
        let rates = vec![0.4, 0.3, 0.2, 0.1];
        let tick = -1e-8;
        let pow10 = precision10(tick);

        let mut expected_candidate_rates = vec![0.39999999, 0.29999999, 0.19999999, 0.09999999];
        expected_candidate_rates.reverse();

        let mut _candidate_rates: Vec<f64> = vec![];

        candidate_rates(&rates, tick, pow10, false, &mut _candidate_rates);

        println!("Candidate Rates: {:?}", _candidate_rates);

        assert_eq!(_candidate_rates, expected_candidate_rates);
    }

    #[test]
    fn test_co1_buy_get_pv_and_rates_2() {
        //
        let r = vec![0.4f64, 0.3f64, 0.2f64, 0.1f64];
        let q = vec![10f64, 20f64, 30f64, 40f64];
        let tick = 0.1f64;
        let pow10 = precision10(tick);

        let mut rates: Vec<f64> = vec![];
        let mut pv: Vec<f64> = vec![];

        get_pv_and_rates(
            &r,
            &q,
            std::f64::INFINITY,
            tick,
            pow10,
            false,
            &mut rates,
            &mut pv,
        );

        println!("result pv: {:?}", pv);
        println!("result rates: {:?}", rates);

        let expected_pv = vec![0f64];
        let expected_rates = vec![0.5f64];

        println!("expected_pv: {:?}", expected_pv);
        println!("expected_rates: {:?}", expected_rates);

        assert!(rates == expected_rates);
        assert!(pv == expected_pv);
    }

    #[test]
    fn test_co1_buy_get_pv_and_rates_3() {
        //
        let r = vec![0.1f64];
        let q = vec![10f64];
        let tick = 0.01f64;
        let pow10 = precision10(tick);

        let mut rates: Vec<f64> = vec![];
        let mut pv: Vec<f64> = vec![];
        get_pv_and_rates(
            &r,
            &q,
            std::f64::INFINITY,
            tick,
            pow10,
            false,
            &mut rates,
            &mut pv,
        );

        let expected_pv = vec![0f64];
        let expected_rates = vec![0.11f64];

        assert!(pv == expected_pv);
        assert!(rates == expected_rates);
    }

    #[test]
    fn test_co1_sell_get_pv_and_rates_1() {
        //
        let r = vec![0.2f64, 0.3f64, 0.4f64, 0.5f64];
        let q = vec![10f64, 20f64, 30f64, 40f64];
        let tick = -0.01f64;
        let pow10 = precision10(tick);

        let mut rates: Vec<f64> = vec![];
        let mut pv: Vec<f64> = vec![];

        get_pv_and_rates(
            &r,
            &q,
            std::f64::INFINITY,
            tick,
            pow10,
            false,
            &mut rates,
            &mut pv,
        );

        println!("result pv: {:?}", pv);
        println!("result rates: {:?}", rates);

        let mut expected_pv = vec![0.0, 2.0, 8.0, 20.0];
        expected_pv.reverse();

        let mut expected_rates = vec![0.19f64, 0.29f64, 0.39f64, 0.49f64];
        expected_rates.reverse();

        assert!(rates == expected_rates);
        assert!(pv == expected_pv);
    }

    #[test]
    fn test_co1_sell_get_pv_and_rates_2() {
        //
        let r = vec![0.2f64, 0.3f64, 0.4f64, 0.5f64];
        let q = vec![10f64, 20f64, 30f64, 40f64];
        let tick = -0.1f64;
        let pow10 = precision10(tick);

        let mut rates: Vec<f64> = vec![];
        let mut pv: Vec<f64> = vec![];

        get_pv_and_rates(
            &r,
            &q,
            std::f64::INFINITY,
            tick,
            pow10,
            false,
            &mut rates,
            &mut pv,
        );

        let expected_pv = vec![0f64];
        let expected_rates = vec![0.1f64];

        assert!(pv == expected_pv);
        assert!(rates == expected_rates);
    }

    #[test]
    fn test_co1_sell_get_pv_and_rates_3() {
        //
        let r = vec![0.1f64];
        let q = vec![10f64];
        let tick = -0.01f64;
        let pow10 = precision10(tick);

        let mut rates: Vec<f64> = vec![];
        let mut pv: Vec<f64> = vec![];

        get_pv_and_rates(
            &r,
            &q,
            std::f64::INFINITY,
            tick,
            pow10,
            false,
            &mut rates,
            &mut pv,
        );

        let expected_pv = vec![0f64];
        let expected_rates = vec![0.09f64];

        assert!(pv == expected_pv);
        assert!(rates == expected_rates);
    }

    #[test]
    fn test_buy_remove_overlap_with_sell_side() {
        //
        let r = vec![0.4f64, 0.3f64, 0.2f64, 0.1f64];
        let q = vec![10f64, 20f64, 30f64, 40f64];
        let tick = 0.01f64;
        let pow10 = precision10(tick);

        let mut rates: Vec<f64> = vec![];
        let mut pv: Vec<f64> = vec![];

        get_pv_and_rates(&r, &q, 0.41f64, tick, pow10, false, &mut rates, &mut pv);

        println!("result pv: {:?}", pv);
        println!("result rates: {:?}", rates);

        let mut expected_pv = vec![4.0, 10.0, 16.0];
        expected_pv.reverse();

        let mut expected_rates = vec![0.31f64, 0.21f64, 0.11f64];
        expected_rates.reverse();

        println!("expected_rates: {:?}", expected_rates);

        assert!(pv == expected_pv);
        assert!(rates == expected_rates);
    }

       #[test]
    fn test_sell_remove_overlap_with_buy_side() {
        //
        let r = vec![0.2f64, 0.3f64, 0.4f64, 0.5f64];
        let q = vec![10f64, 20f64, 30f64, 40f64];
        let tick = -0.01f64;
        let pow10 = precision10(tick);

        let mut rates: Vec<f64> = vec![];
        let mut pv: Vec<f64> = vec![];

        get_pv_and_rates(
            &r,
            &q,
            0.19f64,
            tick,
            pow10,
            false,
            &mut rates,
            &mut pv,
        );

        println!("result pv: {:?}", pv);
        println!("result rates: {:?}", rates);

        let mut expected_pv = vec![2.0, 8.0, 20.0];
        expected_pv.reverse();

        let mut expected_rates = vec![0.29f64, 0.39f64, 0.49f64];
        expected_rates.reverse();

        assert!(rates == expected_rates);
        assert!(pv == expected_pv);
    }

    #[test]
    fn test_cumulative_sum_0() {
        //
        let r = vec![1f64, 2f64, 3f64, 4f64, 5f64, 6f64];
        let q = vec![1f64, 2f64, 3f64, 4f64, 5f64, 6f64];
        let expected = vec![1.0, 5.0, 14.0, 30.0, 55.0, 91.0];
        let pow10 = precision10(0.00000001);

        assert_eq!(cumulative_sum(&r, &q, pow10), expected);
    }

    #[test]
    fn test_cumulative_sum_1() {
        //
        let r = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let q = vec![
            1.00000001, 2.00000001, 3.00000001, 4.00000001, 5.00000001, 6.00000001,
        ];
        let expected = vec![
            1.00000001,
            3.00000002,
            6.00000003,
            10.00000004,
            15.00000005,
            21.00000006,
        ];
        let pow10 = precision10(0.00000001);

        let result = cumulative_sum(&r, &q, pow10);

        println!("result: {:?}", result);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_binary_search_0() {
        //
        let x_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 4f64];
        let y_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 4f64];

        let (x, y) = binary_search(&x_vec, &y_vec, 1.5f64);

        assert!(x.0 == 1f64);
        assert!(x.1 == 2f64);
        assert!(y.0 == 1f64);
        assert!(y.1 == 2f64);
    }

    #[test]
    fn test_binary_search_1() {
        //
        let x_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 4f64];
        let y_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 4f64];

        let (x, y) = binary_search(&x_vec, &y_vec, 1f64);

        assert!(x.0 == 1f64);
        assert!(x.1 == 2f64);
        assert!(y.0 == 1f64);
        assert!(y.1 == 2f64);
    }

    #[test]
    fn test_binary_search_2() {
        //
        let x_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 4f64];
        let y_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 4f64];

        let (x, y) = binary_search(&x_vec, &y_vec, 1.999999999f64);

        assert!(x.0 == 1f64);
        assert!(x.1 == 2f64);
        assert!(y.0 == 1f64);
        assert!(y.1 == 2f64);
    }

    #[test]
    fn test_binary_search_3() {
        //
        let x_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 4f64];
        let y_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 4f64];

        let (x, y) = binary_search(&x_vec, &y_vec, 2f64);

        assert!(x.0 == 2f64);
        assert!(x.1 == 3f64);
        assert!(y.0 == 2f64);
        assert!(y.1 == 3f64);
    }

    #[test]
    fn test_binary_search_4() {
        //
        let x_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 4f64];
        let y_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 4f64];

        let (x, y) = binary_search(&x_vec, &y_vec, -1f64);

        assert!(x.0 == 0f64);
        assert!(x.1 == 1f64);
        assert!(y.0 == 0f64);
        assert!(y.1 == 1f64);
    }

    #[test]
    fn test_binary_search_5() {
        //
        let x_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 4f64];
        let y_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 4f64];

        let (x, y) = binary_search(&x_vec, &y_vec, 100f64);

        assert!(x.0 == 3f64);
        assert!(x.1 == 4f64);
        assert!(y.0 == 3f64);
        assert!(y.1 == 4f64);
    }

    #[test]
    fn test_interpolate_0() {
        //
        let pv = vec![-0.5f64, 0.5f64, 1.5f64, 2.5f64, 3.5f64, 4.5f64];
        let x_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 4f64];
        let y_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 4f64];

        let mut evol: Vec<f64> = vec![];

        interpolate(&pv, &x_vec, &y_vec, &mut evol);

        let expected_evol = vec![0f64, 0.5f64, 1.5f64, 2.5f64, 3.5f64, 4f64];

        println!("evol: {:?}", evol);
        println!("expected_evol: {:?}", expected_evol);

        assert!(expected_evol == evol);
    }

    #[test]
    fn test_interpolate_1() {
        //
        let pv = vec![-0.5f64, 0.5f64, 1.5f64, 2.5f64, 3.5f64, 4.5f64];
        let x_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 5f64];
        let y_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 5f64];

        let mut evol: Vec<f64> = vec![];

        interpolate(&pv, &x_vec, &y_vec, &mut evol);

        let expected_evol = vec![0f64, 0.5f64, 1.5f64, 2.5f64, 3.5f64, 4.5f64];

        println!("evol: {:?}", evol);
        println!("expected_evol: {:?}", expected_evol);

        assert!(expected_evol == evol);
    }

    #[test]
    fn test_interpolate_2() {
        //
        let pv = vec![-0.5f64, 0.5f64, 1.5f64, 2.5f64, 3.5f64, 4f64];
        let x_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 5f64];
        let y_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 5f64];

        let mut evol: Vec<f64> = vec![];

        interpolate(&pv, &x_vec, &y_vec, &mut evol);

        let expected_evol = vec![0f64, 0.5f64, 1.5f64, 2.5f64, 3.5f64, 4f64];

        println!("evol: {:?}", evol);
        println!("expected_evol: {:?}", expected_evol);

        assert!(expected_evol == evol);
    }

    #[test]
    fn test_interpolate_3() {
        //
        let pv = vec![-0.5f64, 0.5f64, 1.5f64, 2.5f64, 3.5f64, 4f64];
        let x_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 5f64];
        let y_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 4f64];
        let tick = 1e-8;

        let mut evol: Vec<f64> = vec![];

        interpolate(&pv, &x_vec, &y_vec, &mut evol);

        let expected_evol = vec![0f64, 0.5f64, 1.5f64, 2.5f64, 3.25f64, 3.5f64];

        println!("evol: {:?}", evol);
        println!("expected_evol: {:?}", expected_evol);

        assert!(cmp_vectors(&evol, &expected_evol, tick));
    }

    #[test]
    fn test_interpolate_4() {
        //
        let pv = vec![-0.5f64, 0.5f64, 1.5f64, 2.5f64, 3.5f64, 6f64];
        let x_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 5f64];
        let y_vec: Vec<f64> = vec![0f64, 1f64, 2f64, 3f64, 4f64];
        let tick = 1e-8;

        let mut evol: Vec<f64> = vec![];

        interpolate(&pv, &x_vec, &y_vec, &mut evol);

        let expected_evol = vec![0f64, 0.5f64, 1.5f64, 2.5f64, 3.25f64, 4f64];

        println!("evol: {:?}", evol);
        println!("expected_evol: {:?}", expected_evol);

        assert!(cmp_vectors(&evol, &expected_evol, tick));
    }
}
