class A {
    async m() {
        return { v: 42 };
    }
}

declare let x:A;

async function test() {
    (await x.m()).v;
    //     ^ defined: 7
    //       ^ defined: 2
    //            ^ defined: 3
}

export {};
