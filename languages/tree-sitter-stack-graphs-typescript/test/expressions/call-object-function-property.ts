type V = { value: number; }

type O = {
    m: (x:V) => V;
    //    ^ defined: 1
    //          ^ defined: 1
};

let foo: O;
//       ^ defined: 3

  foo.m(null).value;
//^ defined: 9
//    ^ defined: 4
//            ^ defined: 1

export {};
