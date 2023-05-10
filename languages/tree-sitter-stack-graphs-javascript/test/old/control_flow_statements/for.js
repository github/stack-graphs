var x = 1;

for (let y = 2; x + y < 2; y++, x--) {
  //            ^ defined: 1, 11
  //                ^ defined: 3, 12
  //                       ^ defined: 3, 12
  //                            ^ defined: 1, 11
  alert(x + y);
  //    ^ defined: 1, 11
  //        ^ defined: 3, 12
  x = 2;
  y = 3;
}

const z = x - y;
//        ^ defined: 1, 11
//            ^ defined: 3, 12
