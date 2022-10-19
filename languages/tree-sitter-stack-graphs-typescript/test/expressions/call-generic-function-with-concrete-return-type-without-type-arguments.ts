interface V {
  value: number;
}

let id: <X>(x: X) => V;
//             ^ defined: 5
//                   ^ defined: 1

let v: number;

  id(v).value;
//^ defined: 5
//   ^ defined: 9
//      ^ defined: 2

export {};
