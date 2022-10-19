let x = class {
    declare f: {
        v: number
    };
}

let y = new x();
//          ^ defined: 1

  y.f.v;
//^ defined: 7
//  ^ defined: 2
//    ^ defined: 3

export {};
