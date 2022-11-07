interface V { v: number; }

declare function f(): Promise<V>;
//                            ^ defined: 1

async function test() {
  (await f()).v;
  //     ^ defined: 3
//            ^ defined: 1
}

export {};
