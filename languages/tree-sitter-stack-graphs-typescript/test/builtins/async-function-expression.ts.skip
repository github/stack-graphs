interface V {
  v: number;
}

let f = async function(): Promise<V> { return { v: 42 } };
//                                ^ defined: 1

async function test() {
  (await f()).v;
  //     ^ defined: 5
  //          ^ defined: 2
}

export {};
