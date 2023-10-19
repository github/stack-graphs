var x = 1;

try {
  var y = 2;
  alert(x);
  //    ^ defined: 1
} catch (e) {
  const z = 3;
  alert(x, y, e);
  //    ^ defined: 1
  //       ^ defined: 4
  //          ^ defined: 7
} finally {
  let w = 4;
  alert(x, y, z);
  //    ^ defined: 1
  //       ^ defined: 4
  //          ^ defined: 8
}

alert(w);
//    ^ defined: 14
