interface A {
    f: number;
}

interface B {
    g: number;
}

interface C
    extends A, B {}
//          ^ defined: 1
//             ^ defined: 5

function test(c: C) {
    return c.f + c.g;
    //       ^ defined: 2
    //             ^ defined: 6
}

export {};
