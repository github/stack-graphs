let obj = { x: 1, y: 2, 0: "z" };
let obj_x = obj.x;
let x = obj_x;
//      ^ defined: 1, 2
let obj_y = obj["y"];
let y = obj_y;
//      ^ defined: 1, 5

let obj_z = obj[0];
let z = obj_z;
//      ^ defined: 1, 9

let obj2 = { x, y };
//           ^ defined: 1, 2, 3
//              ^ defined: 1, 5, 6
