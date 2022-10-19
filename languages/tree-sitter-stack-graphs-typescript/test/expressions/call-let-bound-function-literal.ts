type V = { value: number; }

let foo = function(x:V): V {
//                       ^ defined: 1
    return null;
}

  foo(null).value;
//^ defined: 3
//          ^ defined: 1

export {};
