interface V {
  v: number;
}

let g: (_:V) => any = function(x) {
//        ^ defined: 1
    x.v;
  //^ defined: 5
  //  ^ defined: 2
};

export {};
