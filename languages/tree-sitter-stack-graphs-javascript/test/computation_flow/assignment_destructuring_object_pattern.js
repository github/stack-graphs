let obj = {
    x: 1,
    y: {
        x: 2
    }
};
let {
    x: num,
    y: obj2
} = obj;

/**/ obj2;
//   ^ defined: 3, 9

obj2.x;
//   ^ defined: 4