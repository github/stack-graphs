interface V {
  v: number;
}

type Id<T> = T;

declare let x: Id<V>;
//             ^ defined: 5
//                ^ defined: 1

  x.v;
//^ defined: 7
//  ^ defined: 2

export {};
