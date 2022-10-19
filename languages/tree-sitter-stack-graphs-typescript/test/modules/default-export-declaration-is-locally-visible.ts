export default class T {
    v = 42;
};

declare let x: T;
//             ^ defined: 1

  x.v;
//^ defined: 5
//  ^ defined: 2

export {};
