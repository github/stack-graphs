function foo() {
    this.x = 1;
}

let obj = new foo();

obj.x;
//  ^ defined: 2



function bar(y) {
    this.z = y;
}

let obj_2 = new bar({
    w: 1
});

obj_2.z.w;
//      ^ defined: 17
//    ^ defined: 12, 13