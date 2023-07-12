var x = 1;

for (let y = 2;
      x + y < 2;
    //^ defined: 1, 7, 13
    //    ^ defined: 3, 7, 14
      y++, x--) {
    //^ defined: 3, 7, 14
    //     ^ defined: 1, 7, 13
  alert(x + y);
  //    ^ defined: 1, 7, 13
  //        ^ defined: 3, 7, 14
  x = 2;
  y = 3;
}

const z = x - y;
//        ^ defined: 1, 7, 13
//            ^ defined: 3, 7, 14
