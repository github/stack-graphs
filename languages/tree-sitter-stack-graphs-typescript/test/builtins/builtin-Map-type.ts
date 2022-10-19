interface V {
  v: number;
}

declare let kv: Map<string, V>;
//                          ^ defined: 1

  kv.get("fortytwo")?.v;
//^ defined: 5
//                    ^ defined: 2

  kv.forEach((v, k, kv) => {
      kv.get(k)?.v === v.v;
    //^ defined: 12
    //       ^ defined: 12
    //           ^ defined: 2
    //                 ^ defined: 12
    //                   ^ defined: 2
  });

export {};
