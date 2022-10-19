interface T {
    f: number;
}

const a:T = { f: 42 } as T;
//      ^ defined: 1
//                       ^ defined: 1

  a.f;
//^ defined: 5
//  ^ defined: 2

export {};
