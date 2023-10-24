let x = 1;

while (x--) {
  //   ^ defined: 1, 3, 5
  x = x * 2;
  //  ^ defined: 1, 3, 5
}

const y = x;
//        ^ defined: 1, 3, 5
