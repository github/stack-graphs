type V = { value: number; }

interface I {
    m(x: V): V;
    //   ^ defined: 1
    //       ^ defined: 1
}

interface J extends I {}

let foo: J;
//       ^ defined: 9

  foo.m(null).value;
//^ defined: 11
//    ^ defined: 4
//            ^ defined: 1

export {};
