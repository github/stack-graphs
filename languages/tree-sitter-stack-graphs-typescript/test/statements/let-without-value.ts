interface T {
    f: number;
}

let a:T;
//    ^ defined: 1

  a.f;
//^ defined: 5
//  ^ defined: 2

export {};
