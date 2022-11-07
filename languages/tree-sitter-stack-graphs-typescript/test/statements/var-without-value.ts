interface T {
    f: number;
}

var a:T;
//    ^ defined: 1

  a.f;
//^ defined: 5
//  ^ defined: 2

export {};
