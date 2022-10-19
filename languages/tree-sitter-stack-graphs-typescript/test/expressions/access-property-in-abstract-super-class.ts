type V = { value: number; }

abstract class A {
    f: V;
    // ^ defined: 1
}

class C extends A {
//              ^ defined: 3
}

let x:C;
//    ^ defined: 8

  x.f;
//^ defined: 12
//  ^ defined: 4

export {};
