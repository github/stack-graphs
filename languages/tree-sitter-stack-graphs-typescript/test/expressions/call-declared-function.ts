type V = { value: number; }

function foo(x:V): V {
//                 ^ defined: 1
    return null;
}


  foo(null).value;
//^ defined: 3
//          ^ defined: 1

export {};
