let x = {
    y: {
        z: number
    }
};

function test(a: typeof x.y.z) {
//                      ^ defined: 1
//                        ^ defined: 2
//                          ^ defined: 3
}
