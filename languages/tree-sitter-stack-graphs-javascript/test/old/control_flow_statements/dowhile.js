let x = 1;

do {
  x = x * 2;
  //  ^ defined: 1, 4, 6
} while (x--);
//       ^ defined: 1, 4, 6

const y = x;
//        ^ defined: 1, 4, 6
