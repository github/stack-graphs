namespace A {
    export let b = { v: 42 };
};

  A.b.v;
//^ defined: 1
//  ^ defined: 2
//    ^ defined: 2

export {};
