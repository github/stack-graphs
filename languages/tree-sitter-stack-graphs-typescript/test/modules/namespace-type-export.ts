namespace A {
    export type T = {
        v: number;
    };
};

let x: A.T;
//     ^ defined: 1
//       ^ defined: 2

  x.v;
//^ defined: 7
//  ^ defined: 3

export {};
