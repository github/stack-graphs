type V = { value: number; }

abstract class A {
    m(x: V): V { return x; };
    //   ^ defined: 1
    //       ^ defined: 1
}

class C extends A {
//              ^ defined: 3
}

let foo: C;
//       ^ defined: 9

  foo.m(null).value;
//^ defined: 13
//    ^ defined: 4
//            ^ defined: 1

export {};
