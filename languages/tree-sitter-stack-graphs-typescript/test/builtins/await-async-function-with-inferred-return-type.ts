async function f() {
  return { v: 42 };
};

async function test() {
  (await f()).v;
  //     ^ defined: 1
  //          ^ defined: 2
};

export {};
