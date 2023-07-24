var xs = [0,1,2];

for (x in xs) {
  //      ^ defined: 1
  alert(x);
  //    ^ defined: 1, 3
  var y = 0;
}

let z = y;
//      ^ defined: 7
