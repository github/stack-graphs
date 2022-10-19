interface V {
    v: number;
}
class A {
    f: number = 42;
}




class B extends A implements V {}
//              ^ defined: 4
//                           ^ defined: 1

let b: B;
//     ^ defined: 11

  b.v;
//^ defined: 15
//  ^ defined: 2

  b.f;
//^ defined: 15
//  ^ defined: 5


export {};
