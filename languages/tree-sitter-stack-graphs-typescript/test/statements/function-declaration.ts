type T = { f: number }

function foo(x: T): T {
//              ^ defined: 1
    return { f: x.f } as T;
    //          ^ defined: 3
    //            ^ defined: 1
    //                   ^ defined: 1
}

  foo;
//^ defined: 3

  foo(null).f;
//^ defined: 3
//          ^ defined: 1

export {};
