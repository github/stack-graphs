let obj = { x: 1, y: { z: 2 } };
let { x: xval, y: yval } = obj;
let x = xval;
//      ^ defined: 1, 2

let y = yval;
//      ^ defined: 1, 2

 y.z;
// ^ defined: 1

let [w,
     q] = [1,2];

   w;
// ^ defined: 12

   q;
// ^ defined: 13
