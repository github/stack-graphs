class A {
    f: number = 42;
}

let a = A;
//      ^ defined: 1

class B extends a {}
//              ^ defined: 5

let b: B;
//     ^ defined: 8

  b.f;
//^ defined: 11
//  ^ defined: 2

export {};
