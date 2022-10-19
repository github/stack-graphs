type V = { value: number; }

let foo: {
    (x:V): V;
    // ^ defined: 1
    //     ^ defined: 1
};

  foo(null).value;
//^ defined: 3
//          ^ defined: 1

export {};
