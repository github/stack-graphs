with ({ x: 1, y: 2 }) {
   x + y;
// ^ defined: 1
//     ^ defined: 1
}

let obj = { z: 3,
            w: 4 };
with (obj) {
   z + w;
// ^ defined: 7
//     ^ defined: 8
}
