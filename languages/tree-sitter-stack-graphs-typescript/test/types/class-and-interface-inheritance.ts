interface I {
    i: number;
    m(): I;
    //   ^ defined: 1
}

interface J extends I {
//                  ^ defined: 1
    j: number;
    n(): J;
    //   ^ defined: 7
}

class A {
    f: I;
    // ^ defined: 1
}

class B extends A {
//              ^ defined: 14
    g: J;
//     ^ defined: 7
}

var x: B;
//     ^ defined: 19

  x.f.m().i;
//^ defined: 25
//  ^ defined: 15
//    ^ defined: 3
//        ^ defined: 2

  x.g.n().j;
//^ defined: 25
//  ^ defined: 21
//    ^ defined: 10
//        ^ defined: 9

export {};
