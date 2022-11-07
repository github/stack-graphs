interface V {
  v: number;
}

declare function g(f: (v:V) => number): number;

g(v => {
  return v.v;
  //     ^ defined: 7
  //       ^ defined: 2
});

export {};
