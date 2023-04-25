let x = 0;

if (true) {
  var y = x;
  //      ^ defined: 1
} else if (true) {
  var y = x+1;
  //      ^ defined: 1
} else {
  var y = x-2;
}

const z = y;
//        ^ defined: 1, 4, 7, 10
