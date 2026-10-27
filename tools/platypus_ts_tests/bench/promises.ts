// Promises, async/await, microtask order and timers. Each `async function t_NAME(): Promise<string>`
// is a test; `gen_promises_program.py` adds the glue that stores its result (see promises.ts).

let log = "";

function sleep(ms: number): Promise<void> {
  return new Promise<void>((resolve) => {
    setTimeout(() => {
      resolve();
    }, ms);
  });
}

async function add(a: number, b: number): Promise<number> {
  return a + b;
}

async function boom(): Promise<number> {
  throw new Error("boom");
}

async function slowDouble(n: number): Promise<number> {
  await sleep(n);
  return n * 2;
}

async function a1(): Promise<void> {
  log += "a1-start ";
  await a2();
  log += "a1-end ";
}

async function a2(): Promise<void> {
  log += "a2 ";
}

class Worker {
  base: number;
  constructor(base: number) {
    this.base = base;
  }
  async run(n: number): Promise<number> {
    await sleep(1);
    return this.base + n;
  }
}

async function t_basic(): Promise<string> {
  const v = await add(1, 2);
  const w = await add(v, 10);
  return "v=" + v + " w=" + w;
}

async function t_await_value(): Promise<string> {
  log = "";
  const n = await 5;
  log += "after-await ";
  return log + n;
}

async function t_order(): Promise<string> {
  log = "";
  log += "start ";
  a1();
  Promise.resolve(1)
    .then((v) => {
      log += "p1 ";
    })
    .then(() => {
      log += "p2 ";
    });
  queueMicrotask(() => {
    log += "micro ";
  });
  setTimeout(() => {
    log += "timeout ";
  }, 0);
  log += "end ";
  await sleep(10);
  return log;
}

async function t_throw_caught(): Promise<string> {
  let out = "";
  try {
    await boom();
    out = "no";
  } catch (e) {
    out = "caught:" + (e as Error).message;
  }
  return out;
}

async function t_catch_method(): Promise<string> {
  log = "";
  const r = await boom().catch((e) => {
    log += "handler:" + e.message + " ";
    return 7;
  });
  return log + r;
}

async function t_then_types(): Promise<string> {
  const s = await add(2, 3)
    .then((n) => n * 10)
    .then((n) => "n=" + n)
    .then((t) => t + "!");
  return s;
}

async function t_then_flatten(): Promise<string> {
  log = "";
  const v = await add(1, 1).then((n) => add(n, 40));
  return log + v;
}

async function t_finally_order(): Promise<string> {
  log = "";
  const v = await add(3, 4).finally(() => {
    log += "fin ";
  });
  let r = "none";
  try {
    await boom().finally(() => {
      log += "fin2 ";
    });
  } catch (e) {
    r = (e as Error).message;
  }
  return log + v + " " + r;
}

async function t_all(): Promise<string> {
  const rs = await Promise.all([slowDouble(3), add(1, 1), slowDouble(1)]);
  let sum = 0;
  for (const r of rs) sum += r;
  return rs.join(",") + "|" + sum;
}

async function t_all_reject(): Promise<string> {
  let out = "";
  try {
    await Promise.all([add(1, 1), boom(), slowDouble(2)]);
    out = "no";
  } catch (e) {
    out = "rejected:" + (e as Error).message;
  }
  return out;
}

async function t_race(): Promise<string> {
  const v = await Promise.race([slowDouble(20), slowDouble(3), slowDouble(9)]);
  return "winner=" + v;
}

async function t_timer_order(): Promise<string> {
  log = "";
  setTimeout(() => {
    log += "t10 ";
  }, 10);
  setTimeout(() => {
    log += "t5 ";
  }, 5);
  setTimeout(() => {
    log += "t0 ";
  }, 0);
  setTimeout(() => {
    log += "t5b ";
  }, 5);
  await sleep(20);
  return log;
}

async function t_interval(): Promise<string> {
  log = "";
  let n = 0;
  let id = 0;
  id = setInterval(() => {
    n += 1;
    log += "tick" + n + " ";
    if (n === 3) clearInterval(id);
  }, 4);
  await sleep(40);
  return log;
}

async function t_clear_timeout(): Promise<string> {
  log = "";
  const id = setTimeout(() => {
    log += "never ";
  }, 5);
  setTimeout(() => {
    log += "yes ";
  }, 6);
  clearTimeout(id);
  await sleep(15);
  return log;
}

async function t_methods(): Promise<string> {
  const w = new Worker(100);
  const a = await w.run(1);
  const f = async (x: number): Promise<number> => {
    await sleep(1);
    return x * 3;
  };
  const b = await f(5);
  const g = async (x: number): Promise<number> => x + 1;
  const c = await g(9);
  return a + "," + b + "," + c;
}

async function t_executor(): Promise<string> {
  log = "";
  const twice = new Promise<number>((resolve, reject) => {
    resolve(1);
    resolve(2);
    reject(new Error("late"));
  });
  const thrown = new Promise<number>(() => {
    throw new Error("executor");
  });
  const a = await twice;
  let b = "none";
  try {
    await thrown;
  } catch (e) {
    b = (e as Error).message;
  }
  return a + " " + b;
}

async function t_return_promise(): Promise<string> {
  log = "";
  function_ret();
  await sleep(5);
  return log;
}

async function inner(): Promise<number> {
  return 1;
}

async function viaReturn(): Promise<number> {
  return inner();
}

async function viaAwait(): Promise<number> {
  return await inner();
}

function function_ret(): void {
  viaReturn().then(() => {
    log += "R ";
  });
  viaAwait().then(() => {
    log += "A ";
  });
  Promise.resolve(0)
    .then(() => {
      log += "1 ";
    })
    .then(() => {
      log += "2 ";
    })
    .then(() => {
      log += "3 ";
    })
    .then(() => {
      log += "4 ";
    });
}

async function t_many(): Promise<string> {
  let total = 0;
  const ps: Promise<number>[] = [];
  for (let i = 0; i < 200; i++) ps.push(slowDouble((i % 7) + 1));
  const rs = await Promise.all(ps);
  for (const r of rs) total += r;
  return "total=" + total + " n=" + rs.length;
}

async function t_sequence(): Promise<string> {
  let acc = "";
  for (let i = 1; i <= 4; i++) {
    const v = await slowDouble(i);
    acc += v + ";";
  }
  return acc;
}

async function t_then_two(): Promise<string> {
  const a = await add(1, 2).then(
    (v) => "ok" + v,
    (e) => "err",
  );
  const b = await boom().then(
    (v) => "ok" + v,
    (e) => "err:" + e.message,
  );
  return a + " " + b;
}

async function t_any(): Promise<string> {
  const v = await Promise.any([boom(), slowDouble(4), slowDouble(2)]);
  let r = "none";
  try {
    await Promise.any([boom(), boom()]);
  } catch (e) {
    r = (e as Error).name + ":" + (e as AggregateError).errors.length;
  }
  return v + " " + r;
}

async function t_all_settled(): Promise<string> {
  const rs = await Promise.allSettled([add(1, 2), boom(), slowDouble(1)]);
  let out = "";
  for (const r of rs) {
    out += r.status + ":" + (r.status === "fulfilled" ? String(r.value) : (r.reason as Error).message) + ";";
  }
  return out + rs.length;
}

// ---- glue: start each test and keep its result ----

let r_basic = "";
export function basic(): string {
  r_basic = "";
  t_basic()
    .then((v) => {
      r_basic = v;
    })
    .catch((e) => {
      r_basic = "rejected:" + e.message;
    });
  return "";
}
export function basic_result(): string {
  return r_basic;
}

let r_await_value = "";
export function await_value(): string {
  r_await_value = "";
  t_await_value()
    .then((v) => {
      r_await_value = v;
    })
    .catch((e) => {
      r_await_value = "rejected:" + e.message;
    });
  return "";
}
export function await_value_result(): string {
  return r_await_value;
}

let r_order = "";
export function order(): string {
  r_order = "";
  t_order()
    .then((v) => {
      r_order = v;
    })
    .catch((e) => {
      r_order = "rejected:" + e.message;
    });
  return "";
}
export function order_result(): string {
  return r_order;
}

let r_throw_caught = "";
export function throw_caught(): string {
  r_throw_caught = "";
  t_throw_caught()
    .then((v) => {
      r_throw_caught = v;
    })
    .catch((e) => {
      r_throw_caught = "rejected:" + e.message;
    });
  return "";
}
export function throw_caught_result(): string {
  return r_throw_caught;
}

let r_catch_method = "";
export function catch_method(): string {
  r_catch_method = "";
  t_catch_method()
    .then((v) => {
      r_catch_method = v;
    })
    .catch((e) => {
      r_catch_method = "rejected:" + e.message;
    });
  return "";
}
export function catch_method_result(): string {
  return r_catch_method;
}

let r_then_types = "";
export function then_types(): string {
  r_then_types = "";
  t_then_types()
    .then((v) => {
      r_then_types = v;
    })
    .catch((e) => {
      r_then_types = "rejected:" + e.message;
    });
  return "";
}
export function then_types_result(): string {
  return r_then_types;
}

let r_then_flatten = "";
export function then_flatten(): string {
  r_then_flatten = "";
  t_then_flatten()
    .then((v) => {
      r_then_flatten = v;
    })
    .catch((e) => {
      r_then_flatten = "rejected:" + e.message;
    });
  return "";
}
export function then_flatten_result(): string {
  return r_then_flatten;
}

let r_finally_order = "";
export function finally_order(): string {
  r_finally_order = "";
  t_finally_order()
    .then((v) => {
      r_finally_order = v;
    })
    .catch((e) => {
      r_finally_order = "rejected:" + e.message;
    });
  return "";
}
export function finally_order_result(): string {
  return r_finally_order;
}

let r_all = "";
export function all(): string {
  r_all = "";
  t_all()
    .then((v) => {
      r_all = v;
    })
    .catch((e) => {
      r_all = "rejected:" + e.message;
    });
  return "";
}
export function all_result(): string {
  return r_all;
}

let r_all_reject = "";
export function all_reject(): string {
  r_all_reject = "";
  t_all_reject()
    .then((v) => {
      r_all_reject = v;
    })
    .catch((e) => {
      r_all_reject = "rejected:" + e.message;
    });
  return "";
}
export function all_reject_result(): string {
  return r_all_reject;
}

let r_race = "";
export function race(): string {
  r_race = "";
  t_race()
    .then((v) => {
      r_race = v;
    })
    .catch((e) => {
      r_race = "rejected:" + e.message;
    });
  return "";
}
export function race_result(): string {
  return r_race;
}

let r_timer_order = "";
export function timer_order(): string {
  r_timer_order = "";
  t_timer_order()
    .then((v) => {
      r_timer_order = v;
    })
    .catch((e) => {
      r_timer_order = "rejected:" + e.message;
    });
  return "";
}
export function timer_order_result(): string {
  return r_timer_order;
}

let r_interval = "";
export function interval(): string {
  r_interval = "";
  t_interval()
    .then((v) => {
      r_interval = v;
    })
    .catch((e) => {
      r_interval = "rejected:" + e.message;
    });
  return "";
}
export function interval_result(): string {
  return r_interval;
}

let r_clear_timeout = "";
export function clear_timeout(): string {
  r_clear_timeout = "";
  t_clear_timeout()
    .then((v) => {
      r_clear_timeout = v;
    })
    .catch((e) => {
      r_clear_timeout = "rejected:" + e.message;
    });
  return "";
}
export function clear_timeout_result(): string {
  return r_clear_timeout;
}

let r_methods = "";
export function methods(): string {
  r_methods = "";
  t_methods()
    .then((v) => {
      r_methods = v;
    })
    .catch((e) => {
      r_methods = "rejected:" + e.message;
    });
  return "";
}
export function methods_result(): string {
  return r_methods;
}

let r_executor = "";
export function executor(): string {
  r_executor = "";
  t_executor()
    .then((v) => {
      r_executor = v;
    })
    .catch((e) => {
      r_executor = "rejected:" + e.message;
    });
  return "";
}
export function executor_result(): string {
  return r_executor;
}

let r_return_promise = "";
export function return_promise(): string {
  r_return_promise = "";
  t_return_promise()
    .then((v) => {
      r_return_promise = v;
    })
    .catch((e) => {
      r_return_promise = "rejected:" + e.message;
    });
  return "";
}
export function return_promise_result(): string {
  return r_return_promise;
}

let r_many = "";
export function many(): string {
  r_many = "";
  t_many()
    .then((v) => {
      r_many = v;
    })
    .catch((e) => {
      r_many = "rejected:" + e.message;
    });
  return "";
}
export function many_result(): string {
  return r_many;
}

let r_sequence = "";
export function sequence(): string {
  r_sequence = "";
  t_sequence()
    .then((v) => {
      r_sequence = v;
    })
    .catch((e) => {
      r_sequence = "rejected:" + e.message;
    });
  return "";
}
export function sequence_result(): string {
  return r_sequence;
}

let r_then_two = "";
export function then_two(): string {
  r_then_two = "";
  t_then_two()
    .then((v) => {
      r_then_two = v;
    })
    .catch((e) => {
      r_then_two = "rejected:" + e.message;
    });
  return "";
}
export function then_two_result(): string {
  return r_then_two;
}

let r_any = "";
export function any(): string {
  r_any = "";
  t_any()
    .then((v) => {
      r_any = v;
    })
    .catch((e) => {
      r_any = "rejected:" + e.message;
    });
  return "";
}
export function any_result(): string {
  return r_any;
}

let r_all_settled = "";
export function all_settled(): string {
  r_all_settled = "";
  t_all_settled()
    .then((v) => {
      r_all_settled = v;
    })
    .catch((e) => {
      r_all_settled = "rejected:" + e.message;
    });
  return "";
}
export function all_settled_result(): string {
  return r_all_settled;
}
