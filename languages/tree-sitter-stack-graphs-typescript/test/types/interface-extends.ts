interface V {
    v: number;
}
interface I extends V {}

let x: I;
//     ^ defined: 4

  x.v;
//^ defined: 6
//  ^ defined: 2

export {};
