var obj = {
  x: 1,
  y: 2
};

obj.extend({
  z: 3
});

let x = obj.x;
//          ^ defined: 2
let y = obj.y;
//          ^ defined: 3
let z = obj.z;
//          ^ defined: 7
