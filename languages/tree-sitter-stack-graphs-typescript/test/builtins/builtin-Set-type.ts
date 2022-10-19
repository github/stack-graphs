interface V {
  v: number;
}

declare let xs: Set<V>;
//                  ^ defined: 1

  xs.forEach((x1, x2) => {
      x1.v;
    //^ defined: 8
    //   ^ defined: 2
      x2.v;
    //^ defined: 8
    //   ^ defined: 2
  });

export {};
