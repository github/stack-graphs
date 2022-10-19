type V = { value: number; }

interface I {
    m(x: V): V;
    //   ^ defined: 1
    //       ^ defined: 1
}

class C implements I {
    m(x:V) { return x; }
}

let foo: C;
//       ^ defined: 9

  foo.m(null).value;
//^ defined: 13
//    ^ defined: 4, 10
//            ^ defined: 1

export {};
