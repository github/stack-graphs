let obj = {
    x: 1, y: 2, 0: "z" };

let obj_x = obj.x;
//          ^ defined: 1
//              ^ defined: 2
let x = obj_x;
//      ^ defined: 4, 2

let obj_y = obj["y"];
//          ^ defined: 1
//              ^ defined: 2
let y = obj_y;
//      ^ defined: 10, 2

let obj_z = obj[0];
//          ^ defined: 1
//              ^ defined: 2
let z = obj_z;
//      ^ defined: 16, 2

let obj2 = { x, y };
//           ^ defined: 7, 4, 2
//              ^ defined: 13, 10, 2
