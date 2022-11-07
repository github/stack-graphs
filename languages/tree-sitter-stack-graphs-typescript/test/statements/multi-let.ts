interface T {
    f: number;
}

let a:T, b:T = { f: 42 } as T;
//    ^ defined: 1
//         ^ defined: 1
//                          ^ defined: 1

  a.f;
//^ defined: 5
//  ^ defined: 2

  b.f;
//^ defined: 5
//  ^ defined: 2

export {};
