#[macro_use]
extern crate neon;
extern crate chrono;
extern crate itertools;
extern crate redis;
extern crate rustc_serialize;
extern crate trader;

#[allow(unused_imports)]
use itertools::enumerate;
use neon::prelude::*;

#[allow(unused_imports)]
use std::collections::VecDeque;
#[allow(unused_imports)]
use std::vec;
use std::vec::Vec;

#[allow(unused_imports)]
use chrono::prelude::*;
#[allow(unused_imports)]
use rustc_serialize::json::{self, Json, ToJson};
#[allow(unused_imports)]
use std::cmp;
use std::panic;

#[allow(non_snake_case)]

fn compute(_task_stuff: &ComputerOrdersTask) -> std::string::String {
  let new_rates = trader::compute_orders(
    _task_stuff.fee_rate,
    _task_stuff.tick,
    _task_stuff.quantity_limit,
    &_task_stuff.pdf_x,
    &_task_stuff.pdf_y,
    &_task_stuff.buy_rates,
    &_task_stuff.buy_quantities,
    &_task_stuff.sell_rates,
    &_task_stuff.sell_quantities,
    _task_stuff.hft_sim_testing,
    _task_stuff.allow_order_conflicts,
  );

  let ret = format!("{{\"buy\": {}, \"sell\": {}}}", new_rates.0, new_rates.1);

  ret.to_string()
}

struct ComputerOrdersTask {
  fee_rate: f64,
  tick: f64,
  quantity_limit: f64,
  pdf_x: Vec<f64>,
  pdf_y: Vec<f64>,
  buy_rates: Vec<f64>,
  buy_quantities: Vec<f64>,
  sell_rates: Vec<f64>,
  sell_quantities: Vec<f64>,
  hft_sim_testing: bool,
  allow_order_conflicts: bool,
}

impl Task for ComputerOrdersTask {
  //
  type Output = String;
  type Error = ();
  type JsEvent = JsString;

  fn perform(&self) -> Result<String, ()> {
    Ok(compute(self))
  }

  fn complete<'a>(self, mut cx: TaskContext<'a>, result: Result<String, ()>) -> JsResult<JsString> {
    match result {
      Ok(result2) => Ok(cx.string(result2.to_string())),
      Err(error) => panic!("CO Problem {:?}", error),
    }
  }
}

#[derive(Debug)]
pub struct EngineEvol {}

declare_types! {

  pub class JsEngine for EngineEvol {

    init(mut _cx) {

      Ok(EngineEvol {})
    }

    method ComputeOrders(mut cx){

        let mut js_arr_handle: Handle<JsArray> = cx.argument(0)?;
        let mut vec: Vec<Handle<JsValue>> = js_arr_handle.to_vec(&mut cx)?;

        // buy_rates
        let buy_rates= vec
            .iter()
            .map(|js_value| {
                js_value
                    .downcast::<JsNumber>()
                    // If downcast fails, default to using 0
                    .unwrap_or(cx.number(0))
                    // Get the value of the unwrapped value
                    .value()
            })
            .collect();

        // buy_quantities
        js_arr_handle = cx.argument(1)?;
        vec = js_arr_handle.to_vec(&mut cx)?;

        let buy_quantities = vec
            .iter()
            .map(|js_value| {
                js_value
                    .downcast::<JsNumber>()
                    // If downcast fails, default to using 0
                    .unwrap_or(cx.number(0))
                    // Get the value of the unwrapped value
                    .value()
            })
            .collect();

        // sell_rates
        js_arr_handle = cx.argument(2)?;
        vec = js_arr_handle.to_vec(&mut cx)?;

        let sell_rates= vec
            .iter()
            .map(|js_value| {
                js_value
                    .downcast::<JsNumber>()
                    // If downcast fails, default to using 0
                    .unwrap_or(cx.number(0))
                    // Get the value of the unwrapped value
                    .value()
            })
            .collect();

        // sell_quantities
        js_arr_handle = cx.argument(3)?;
        vec = js_arr_handle.to_vec(&mut cx)?;

        let sell_quantities= vec
            .iter()
            .map(|js_value| {
                js_value
                    .downcast::<JsNumber>()
                    // If downcast fails, default to using 0
                    .unwrap_or(cx.number(0))
                    // Get the value of the unwrapped value
                    .value()
            })
            .collect();

      let fee_rate = cx.argument::<JsNumber>(4)?.value();
      let tick = cx.argument::<JsNumber>(5)?.value();
      let quantity_limit = cx.argument::<JsNumber>(6)?.value();

       // pdf_x
        js_arr_handle = cx.argument(7)?;
        vec = js_arr_handle.to_vec(&mut cx)?;

        let pdf_x= vec
            .iter()
            .map(|js_value| {
                js_value
                    .downcast::<JsNumber>()
                    // If downcast fails, default to using 0
                    .unwrap_or(cx.number(0))
                    // Get the value of the unwrapped value
                    .value()
            })
            .collect();

        // pdf_y
        js_arr_handle = cx.argument(8)?;
        vec = js_arr_handle.to_vec(&mut cx)?;
         let pdf_y= vec
            .iter()
            .map(|js_value| {
                js_value
                    .downcast::<JsNumber>()
                    // If downcast fails, default to using 0
                    .unwrap_or(cx.number(0))
                    // Get the value of the unwrapped value
                    .value()
            })
            .collect();

      let hft_sim_testing = cx.argument::<JsBoolean>(9)?.value();
      let allow_order_conflicts = cx.argument::<JsBoolean>(10)?.value();

      let callback = cx.argument::<JsFunction>(11)?;

      let task = ComputerOrdersTask {
          fee_rate,
          quantity_limit,
          tick,
          pdf_x,
          pdf_y,
          buy_rates,
          buy_quantities,
          sell_rates,
          sell_quantities,
          hft_sim_testing,
          allow_order_conflicts,
      };

      task.schedule(callback);

      Ok(cx.string("").upcast())
    }
  }
}

register_module!(mut m, {
  m.export_class::<JsEngine>("EngineEvol");
  Ok(())
});
